# Zero allocation: code

## Count heap calls on the hotpath thread

Compile-checked: cargo check and clippy, aarch64-apple-darwin, x86_64-unknown-linux-gnu and aarch64-unknown-linux-gnu. Run in release on aarch64-apple-darwin: 0 heap calls in 1,000,000 packets. Not run under Miri.

- Counts `alloc`, `realloc` and `dealloc` on the current thread only. Test-harness and logging threads add no noise.
- `try_with` stops a panic while thread-local storage is torn down.
- `DeterministicMarketDataProcessor` stands in for the hotpath under test. Put your own entry point in its place.
- Run in the release profile. Warm up once outside the window.

```rust
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::hint::black_box;

thread_local! {
    // const init and no Drop: reading it never allocates.
    static HEAP_CALLS: Cell<u64> = const { Cell::new(0) };
}

pub struct CountingAlloc;

// SAFETY: every call forwards to System with the same arguments, so System's
// guarantees hold. The counter update does not allocate and does not panic.
unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let _ = HEAP_CALLS.try_with(|c| c.set(c.get() + 1));
        // SAFETY: same contract as the caller's call to `alloc`.
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // A free on the hotpath is also a fault: think of the last Arc drop.
        let _ = HEAP_CALLS.try_with(|c| c.set(c.get() + 1));
        // SAFETY: `ptr` came from `System.alloc` with this `layout`.
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let _ = HEAP_CALLS.try_with(|c| c.set(c.get() + 1));
        // SAFETY: same contract as the caller's call to `realloc`.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: CountingAlloc = CountingAlloc;

#[derive(Clone, Copy, Default)]
#[repr(C, align(64))]
pub struct MarketUpdate {
    pub current_price: f64,
    pub instrument_id: u32,
}

pub struct DeterministicMarketDataProcessor {
    pre_allocated_updates: Box<[MarketUpdate]>,
    current_write_index: usize,
}

impl DeterministicMarketDataProcessor {
    pub fn new(maximum_capacity: usize) -> Self {
        assert!(maximum_capacity > 0, "capacity must be above zero");
        // A boxed slice has no spare capacity, so it can never grow.
        // Do not count on this fill to pre-fault: a zero fill can become a
        // lazy zeroed allocation. Pre-fault with mlockall or MAP_POPULATE.
        let pre_allocated_updates = vec![MarketUpdate::default(); maximum_capacity].into_boxed_slice();
        Self { pre_allocated_updates, current_write_index: 0 }
    }

    #[inline]
    pub fn process_incoming_packet(&mut self, id: u32, price: f64) {
        let active_slot = &mut self.pre_allocated_updates[self.current_write_index];
        active_slot.instrument_id = id;
        active_slot.current_price = price;

        // A compare and reset in place of `%`: no integer division.
        self.current_write_index += 1;
        if self.current_write_index == self.pre_allocated_updates.len() {
            self.current_write_index = 0;
        }
    }
}

fn main() {
    let mut engine = DeterministicMarketDataProcessor::new(4096);
    engine.process_incoming_packet(0, 0.0); // warm-up outside the measured window

    let before = HEAP_CALLS.with(Cell::get);
    for i in 0..1_000_000u32 {
        engine.process_incoming_packet(black_box(i), f64::from(i));
    }
    black_box(&engine); // keeps the optimizer from deleting the loop
    let after = HEAP_CALLS.with(Cell::get);
    assert_eq!(after - before, 0, "hotpath made {} heap calls", after - before);
}
```

Other proofs. Each one is the `#[global_allocator]` of its own binary.

| Tool | Calls | Not obvious |
| --- | --- | --- |
| `assert_no_alloc` 1.1.2 (abort) | `static GLOBAL: AllocDisabler = AllocDisabler;`, then `assert_no_alloc(\|\| { .. })` | This thread only. `permit_alloc(\|\| ..)` opens a hole for a known cold path. `default-features = false` keeps it on in release. |
| `dhat` 0.3.3 (call stack) | `static ALLOC: dhat::Alloc = dhat::Alloc;`, `dhat::Profiler::builder().testing().build()`, `dhat::HeapStats::get()` before and after, `dhat::assert_eq!(after.total_blocks, before.total_blocks)` | Writes `dhat-heap.json` (DHAT viewer) before it panics. Counts every thread. One `Profiler` at a time: one test per `tests/*.rs` file. |
| `#![no_std]` crate, no `alloc` | none | `Box::new` fails with "cannot find type `Box` in this scope". |

Whole binary, no code change (not tested): `heaptrack`, `valgrind --tool=dhat`, or a `bpftrace` uprobe on `malloc` filtered by the hotpath thread id.

## Fixed-block pool behind the `Allocator` trait

IF a `Vec` or `hashbrown` map must use pre-faulted pool memory, THEN use this. ELSE use `slab`, `heapless` or `bumpalo`: no `unsafe`.

Compile-checked as printed: cargo check and clippy, aarch64-apple-darwin, x86_64-unknown-linux-gnu and aarch64-unknown-linux-gnu. Tests ran on aarch64-apple-darwin: zero size, oversize and over-align refused, exhaustion refused, LIFO reuse, 64-byte alignment, `Vec` growth from capacity 4 to 8, hash table refused at 64-byte blocks and fits 128. Not run under Miri.

```toml
[dependencies]
thiserror = { version = "2.0.21", default-features = false }
allocator-api2 = { version = "0.2.21", default-features = false, features = ["alloc"] }
hashbrown = { version = "0.17.1", default-features = false, features = ["allocator-api2", "default-hasher"] }
memmap2 = { version = "0.9.12", default-features = false }
```

```rust
use allocator_api2::alloc::{AllocError, Allocator, Layout};
use allocator_api2::vec::Vec as PoolVec;
use core::cell::RefCell;
use core::ptr::NonNull;
use memmap2::{MmapMut, MmapOptions};

pub const BLOCK_ALIGN: usize = 64;

#[derive(Debug, thiserror::Error)]
pub enum PoolError {
    #[error("pool block size and block count must be above zero")]
    ZeroSize,
    #[error("pool size overflows the address space")]
    TooLarge,
    #[error("pool block address is null")]
    NullAddress,
    #[error("cannot map pool memory")]
    Map(#[from] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
#[error("pool has no free block")]
pub struct PoolExhausted;

pub struct BareMetalObjectPool {
    block_size: usize,
    region_start: usize,
    region_end: usize,
    available_pointers_stack: RefCell<Vec<NonNull<u8>>>,
    // Field order matters: the map is dropped last, after the stack.
    _physical_memory_arena: MmapMut,
}

impl BareMetalObjectPool {
    pub fn new(block_size: usize, maximum_blocks: usize) -> Result<Self, PoolError> {
        if block_size == 0 || maximum_blocks == 0 {
            return Err(PoolError::ZeroSize);
        }
        // Round up to a full cache line.
        let block_size = block_size.next_multiple_of(BLOCK_ALIGN);
        let bytes = block_size.checked_mul(maximum_blocks).ok_or(PoolError::TooLarge)?;

        let mut options = MmapOptions::new();
        options.len(bytes);
        #[cfg(target_os = "linux")]
        options.populate(); // MAP_POPULATE: fault every page in now.
        let mut arena = options.map_anon()?;

        let base = arena.as_mut_ptr();
        let mut stack = Vec::with_capacity(maximum_blocks);
        for index in 0..maximum_blocks {
            // wrapping_add makes the address with no unsafe; it stays inside the map.
            let block = base.wrapping_add(index * block_size);
            stack.push(NonNull::new(block).ok_or(PoolError::NullAddress)?);
        }

        let region_start = base.addr();
        Ok(Self {
            block_size,
            region_start,
            region_end: region_start + bytes,
            available_pointers_stack: RefCell::new(stack),
            _physical_memory_arena: arena,
        })
    }

    fn owns(&self, block: NonNull<u8>) -> bool {
        (self.region_start..self.region_end).contains(&block.addr().get())
    }
}

// SAFETY: each block handed out is inside the live map, is `block_size`
// bytes, and starts on a 64-byte boundary: the map is page-aligned and
// `block_size` is a multiple of 64. A block is on the free stack at most
// once. Moving the pool does not move the map, and the map lives as long
// as the pool, so no block outlives its memory.
unsafe impl Allocator for BareMetalObjectPool {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        // Refuse an over-aligned request as well as an oversized one.
        if layout.size() > self.block_size || layout.align() > BLOCK_ALIGN {
            return Err(AllocError);
        }
        // Empty stack is an error. There is no fallback to the global heap.
        let block = self.available_pointers_stack.borrow_mut().pop().ok_or(AllocError)?;
        Ok(NonNull::slice_from_raw_parts(block, self.block_size))
    }

    unsafe fn deallocate(&self, block: NonNull<u8>, _layout: Layout) {
        // A pointer from any other allocator here is a caller bug.
        debug_assert!(self.owns(block), "block does not belong to this pool");
        // Capacity is `maximum_blocks`, so this push never reallocates.
        self.available_pointers_stack.borrow_mut().push(block);
    }
}

pub fn build_transient_chain(order_pool: &BareMetalObjectPool) -> Result<PoolVec<u64, &BareMetalObjectPool>, PoolExhausted> {
    // Reserve the whole block once, so the pushes below cannot ask again.
    let mut transient_order_chain = PoolVec::new_in(order_pool);
    transient_order_chain
        .try_reserve_exact(BLOCK_ALIGN / size_of::<u64>())
        .map_err(|_| PoolExhausted)?;
    transient_order_chain.push_within_capacity(1001).map_err(|_| PoolExhausted)?;
    transient_order_chain.push_within_capacity(1002).map_err(|_| PoolExhausted)?;
    Ok(transient_order_chain)
}

/// One vector type for every pool, chosen at run time.
pub type DynVec<'a, T> = PoolVec<T, &'a dyn Allocator>;
```

- [ ] CAUTION: Do not pass a block from another allocator to `deallocate`. The ownership test is a `debug_assert!`, so a release build puts the foreign block on the stack and hands it out later.
- [ ] Growth asks the pool again: `allocator_api2::vec::Vec<u64>` takes capacity 4 (32 bytes) on first push, then 8. Each growth = new block + free of the old one.
- [ ] `Vec<T, &Pool>` and `Vec<T, Global>` are different types. `DynVec` = one type, one virtual call per allocation.
- [ ] Nightly-only in std: `Allocator`, `Vec::new_in` (`allocator_api`), `Vec::push_within_capacity` (`vec_push_within_capacity`). `allocator-api2` has them on stable. Code moves to std with a path change.
- [ ] Production on Linux: add `huge(Some(30))` to the map options for one 1 GB page.
