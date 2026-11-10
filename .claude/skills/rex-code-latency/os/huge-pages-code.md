# Huge pages code

Startup code that puts hotpath data in one locked, pre-faulted huge page arena. No listing has an `unsafe` block: `memmap2`, `nix` and `bytemuck` hold it.

```toml
[dependencies]
bytemuck = { version = "1.25.2", default-features = false }
memmap2 = { version = "0.9.12", default-features = false }
nix = { version = "0.31.3", default-features = false, features = ["mman", "process"] }
thiserror = { version = "2.0.21", default-features = false }
```

## Map, lock and pre-fault a 1 GB huge page arena

Compile-checked: cargo check and clippy, x86_64-unknown-linux-gnu, in a module gated by `#[cfg(target_os = "linux")]`. Not run: no reserved 1 GB pages were available.

```rust
use memmap2::{MmapMut, MmapOptions};
use nix::sys::mman::{MlockAllFlags, mlockall};

pub const ONE_GIGABYTE: usize = 1024 * 1024 * 1024;
// The page size as a power of two. memmap2 turns 30 into the MAP_HUGE_1GB size bits.
const HUGE_PAGE_BITS_1GB: u8 = 30;

#[derive(Debug, thiserror::Error)]
pub enum ArenaError {
    #[error(
        "kernel rejected the 1 GB huge page request; check that 1 GB huge pages are reserved in the boot parameters"
    )]
    HugePageRejected(#[source] std::io::Error),
    #[error("mlockall failed; the process needs CAP_IPC_LOCK or a larger RLIMIT_MEMLOCK")]
    LockRejected(#[source] nix::Error),
}

pub struct ContiguousHugePageAllocator;

impl ContiguousHugePageAllocator {
    // MAP_PRIVATE | MAP_ANONYMOUS | MAP_HUGETLB | MAP_HUGE_1GB, PROT_READ | PROT_WRITE, fd -1, offset 0
    pub fn claim_gigabyte_block() -> Result<MmapMut, ArenaError> {
        MmapOptions::new()
            .len(ONE_GIGABYTE)
            .huge(Some(HUGE_PAGE_BITS_1GB))
            .map_anon()
            .map_err(ArenaError::HugePageRejected)
    }
}
// Dropping the MmapMut unmaps it with the length it was mapped with.

pub fn secure_and_wire_arena(memory_block: &mut [u8]) -> Result<(), ArenaError> {
    // Lock first, so the kernel can never swap a page of this process out.
    lock_physical_memory()?;

    // Then touch each page, so the kernel maps it now and not on the hotpath.
    execute_pre_fault_loop(memory_block);
    Ok(())
}

fn lock_physical_memory() -> Result<(), ArenaError> {
    // MCL_CURRENT locks every page mapped now; MCL_FUTURE locks every later mapping as it is made
    mlockall(MlockAllFlags::MCL_CURRENT | MlockAllFlags::MCL_FUTURE)
        .map_err(ArenaError::LockRejected)
}

fn execute_pre_fault_loop(memory_block: &mut [u8]) {
    const STANDARD_PAGE_SIZE: usize = 4096;

    for byte in memory_block.iter_mut().step_by(STANDARD_PAGE_SIZE) {
        *byte = 0;
    }
    // The block is observed after the loop, so the optimizer must keep every store
    std::hint::black_box(&mut *memory_block);
}
```

- Use on the pinned hotpath thread, after the pin and before the network connects: `let mut arena = ContiguousHugePageAllocator::claim_gigabyte_block()?;` then `secure_and_wire_arena(&mut arena)?;`. `MmapMut` derefs to `&mut [u8]`. Keep it alive for the life of the process.
- `mlockall(MCL_CURRENT)` already faults in every mapped page, so the loop is a second guard. It still works IF the lock is skipped in a test build.
- To fault the arena inside the map call instead, add `.populate()` before `.map_anon()`.
- For 2 MB pages, pass `Some(21)` and reserve 2 MB pages on the boot line.

## Carve the arena into struct-of-arrays slices

Compile-checked: cargo check, aarch64-apple-darwin and x86_64-unknown-linux-gnu. A unit test on a 1 MB anonymous map passed on aarch64-apple-darwin.

```rust
use bytemuck::PodCastError;

#[derive(Debug, thiserror::Error)]
pub enum CarveError {
    #[error("arena is too small for {capacity} orders")]
    TooSmall { capacity: usize },
    #[error("arena slice is not aligned for its element type: {0:?}")]
    Misaligned(PodCastError),
}

// Struct-of-arrays view over raw arena bytes; no copy, no allocation
pub struct ArenaBook<'a> {
    pub unique_order_ids: &'a mut [u64],
    pub resting_prices: &'a mut [f64],
    pub active_volumes: &'a mut [u32],
}

pub fn carve_book(arena: &mut [u8], capacity: usize) -> Result<ArenaBook<'_>, CarveError> {
    let too_small = || CarveError::TooSmall { capacity };
    let wide = capacity.checked_mul(8).ok_or_else(too_small)?;
    let narrow = capacity.checked_mul(4).ok_or_else(too_small)?;

    let (ids, rest) = arena.split_at_mut_checked(wide).ok_or_else(too_small)?;
    let (prices, rest) = rest.split_at_mut_checked(wide).ok_or_else(too_small)?;
    let (volumes, _) = rest.split_at_mut_checked(narrow).ok_or_else(too_small)?;

    Ok(ArenaBook {
        unique_order_ids: bytemuck::try_cast_slice_mut(ids).map_err(CarveError::Misaligned)?,
        resting_prices: bytemuck::try_cast_slice_mut(prices).map_err(CarveError::Misaligned)?,
        active_volumes: bytemuck::try_cast_slice_mut(volumes).map_err(CarveError::Misaligned)?,
    })
}
```

- Order the pieces from widest element to narrowest (8, 8, 4 bytes). The arena starts on a page boundary, so each piece is then aligned.
- `ArenaBook` borrows the arena. The borrow checker stops the `MmapMut` from being unmapped while `ArenaBook` lives.
- CAUTION: Do not cast arena bytes to a type with invariants, such as `bool`, an `enum` or a reference. That is undefined behavior. Use only `Pod` element types.
