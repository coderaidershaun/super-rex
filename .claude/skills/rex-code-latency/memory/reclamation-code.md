# Memory reclamation: code

## Lock-free stack over a fixed pool with a tagged head

Compile-checked: cargo check, aarch64-apple-darwin and x86_64-unknown-linux-gnu. Replay test passes: the stale CAS fails and the stack stays A, C. Not run under loom.

- [ ] Nodes live in a fixed pool and are named by `u32` index. No `unsafe`, no 128-bit atomic.
- [ ] Each successful push or pop increments the tag, so a recycled index comes back with a new tag and a stale CAS fails.
- [ ] Limit: a thread that stalls through exactly 2^32 head changes can still see ABA.
- [ ] Constructor not shown: one `AtomicU32::new(NIL)` per slot in `next_node`, and `global_head` at `pack(0, NIL)`.
- [ ] Std only. No `Cargo.toml` lines.

```rust
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering::{Acquire, Relaxed, Release}};

pub const NIL: u32 = u32::MAX;

const fn pack(tag: u32, index: u32) -> u64 { ((tag as u64) << 32) | index as u64 }
const fn index_of(word: u64) -> u32 { word as u32 }
const fn tag_of(word: u64) -> u32 { (word >> 32) as u32 }

/// Lock-free LIFO stack over a fixed pool. The head word carries a 32-bit version tag next to a 32-bit index.
pub struct TaggedStack {
    global_head: AtomicU64,
    next_node: Box<[AtomicU32]>,
}

impl TaggedStack {
    pub fn push_node(&self, index: u32) {
        let mut head = self.global_head.load(Relaxed);
        loop {
            self.next_node[index as usize].store(index_of(head), Relaxed);
            let new = pack(tag_of(head).wrapping_add(1), index);
            match self.global_head.compare_exchange_weak(head, new, Release, Relaxed) {
                Ok(_) => return,
                Err(seen) => head = seen,
            }
        }
    }

    pub fn pop_node(&self) -> Option<u32> {
        let mut head = self.global_head.load(Acquire);
        while index_of(head) != NIL {
            let next = self.next_node[index_of(head) as usize].load(Relaxed);
            // A recycled index comes back with a new tag, so a stale CAS fails.
            let new = pack(tag_of(head).wrapping_add(1), next);
            match self.global_head.compare_exchange_weak(head, new, Acquire, Acquire) {
                Ok(_) => return Some(index_of(head)),
                Err(seen) => head = seen,
            }
        }
        None
    }
}

const _: () = assert!(size_of::<AtomicU64>() == 8);
```

## Hazard domain over pool indices

Compile-checked: cargo check, aarch64-apple-darwin and x86_64-unknown-linux-gnu. Compiled, not run. Not run under loom.

- [ ] Slots hold pool indices, so the domain needs no `unsafe`. Over raw addresses it hands back a `*mut T`, and `&T` from that needs `unsafe`: use `haphazard` there.
- [ ] One slot per core, each `CachePadded` (128 bytes on x86_64 and aarch64): a reader writes only its own line. Build the domain at startup.
- [ ] CAUTION: Read the node only between `protect` and `release_hazard_protection`. After the release, the writer can recycle the slot under the reader.
- [ ] Run `reclaim` in batches on a housekeeping core. Reserve `retired` at startup. The scan is O(retired × cores): for hundreds of retired indices, copy the slots into a fixed array and sort it first.
- [ ] Reader is lock-free, not wait-free: the loop retries while a fast writer keeps swapping.
- [ ] `fence(SeqCst)` = `mfence` on x86_64, `dmb ish` on aarch64.
- [ ] `CachePadded` needs no `std` feature. Keep `crossbeam-utils` defaults off in every crate of the build: one `std` unifies into all, and `Backoff::snooze()` can then yield to the OS.

```toml
[dependencies]
crossbeam-utils = { version = "0.8.23", default-features = false }
```

```rust
use crossbeam_utils::CachePadded;
use std::sync::atomic::{AtomicU32, fence, Ordering::{Acquire, Release, SeqCst}};

pub const NO_HAZARD: u32 = u32::MAX;

/// One hazard slot per core. Each slot sits on its own cache line pair.
pub struct BareMetalHazardDomain {
    global_hazard_array: Box<[CachePadded<AtomicU32>]>,
}

impl BareMetalHazardDomain {
    /// Called once at startup. The hotpath never allocates.
    pub fn new(core_count: usize) -> Self {
        let global_hazard_array =
            (0..core_count).map(|_| CachePadded::new(AtomicU32::new(NO_HAZARD))).collect();
        Self { global_hazard_array }
    }

    /// Reader: load, publish, fence, re-load, compare. Returns a protected index.
    pub fn protect(&self, core_id: usize, shared: &AtomicU32) -> u32 {
        let slot = &self.global_hazard_array[core_id];
        let mut seen = shared.load(Acquire);
        loop {
            slot.store(seen, Release);
            // The hazard store must leave the store buffer before the validation load.
            fence(SeqCst);
            let again = shared.load(Acquire);
            if again == seen {
                return seen;
            }
            seen = again;
        }
    }

    pub fn release_hazard_protection(&self, core_id: usize) {
        self.global_hazard_array[core_id].store(NO_HAZARD, Release);
    }

    /// Writer, off the hotpath: return to the pool every retired index that no core protects.
    pub fn reclaim(&self, retired: &mut Vec<u32>, mut recycle_to_pool: impl FnMut(u32)) {
        // Pairs with the reader fence. The unlink must be visible before the scan loads.
        fence(SeqCst);
        retired.retain(|&index| {
            let hazardous = self.global_hazard_array.iter().any(|slot| slot.load(Acquire) == index);
            if !hazardous {
                recycle_to_pool(index);
            }
            hazardous
        });
    }
}
```

## Read-mostly config with `arc-swap` and a writer-owned retired list

Compile-checked: cargo check, aarch64-apple-darwin and x86_64-unknown-linux-gnu. Test passes: an old version stays alive while a reader cache holds it, then is freed.

- [ ] Constructors not shown: `ArcSwap::from_pointee(initial)`, `Cache::new(&manager.global_routing_state)`, `Vec::with_capacity(n)` for `retired`.
- [ ] Reader: one relaxed load and a compare while the table is unchanged. After a swap, each reader pays once: one count increment and one drop of its old `Arc`.
- [ ] `Cache::load` takes `&mut self`: one `RoutingReader` per thread.
- [ ] `Arc::strong_count == 1` is a safe free test here: the old version left the `ArcSwap`, and `rcu` turned every debt into a real count before it returned. No reader can gain a new reference.
- [ ] `rcu` allocates a new `Arc`: keep `RoutingWriter` on a housekeeping core.
- [ ] A reader that stops calling `load` holds its old version. The retired list fills, and the update returns `RetiredListFull`.

```toml
[dependencies]
arc-swap = { version = "1.9.5", default-features = false }
thiserror = { version = "2.0.21", default-features = false }
```

```rust
use arc_swap::{ArcSwap, Cache};
use std::sync::Arc;

#[derive(Debug, Clone, Copy)]
pub struct ExchangeRoutingTable {
    pub primary_gateway_ip: u32,
    pub secondary_gateway_ip: u32,
    pub latency_tolerance_us: f64,
    pub is_circuit_breaker_active: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    #[error("retired list full: readers still hold {held} old routing tables")]
    RetiredListFull { held: usize },
}

pub enum Route { Halt, Gateway(u32) }

pub struct LockFreeConfigurationManager {
    global_routing_state: ArcSwap<ExchangeRoutingTable>,
}

/// One per reader thread. `Cache::load` takes `&mut self`, so it cannot be shared.
pub struct RoutingReader<'m> {
    cache: Cache<&'m ArcSwap<ExchangeRoutingTable>, Arc<ExchangeRoutingTable>>,
}

/// Housekeeping core only. Holds old versions so they are freed here, not on a reader.
pub struct RoutingWriter<'m> {
    manager: &'m LockFreeConfigurationManager,
    retired: Vec<Arc<ExchangeRoutingTable>>,
}

impl RoutingReader<'_> {
    /// Hotpath: one relaxed load and a compare while the table has not changed.
    pub fn evaluate_routing_path(&mut self) -> Route {
        let active_table = self.cache.load();
        if active_table.is_circuit_breaker_active {
            Route::Halt
        } else {
            Route::Gateway(active_table.primary_gateway_ip)
        }
    }
}

impl RoutingWriter<'_> {
    /// Copy, edit, publish. `rcu` retries the copy if another writer won the race.
    pub fn update_circuit_breaker_status(&mut self, active: bool) -> Result<(), UpdateError> {
        if self.retired.len() == self.retired.capacity() && self.reclaim() == self.retired.capacity() {
            return Err(UpdateError::RetiredListFull { held: self.retired.len() });
        }
        let old = self.manager.global_routing_state.rcu(|current| ExchangeRoutingTable {
            is_circuit_breaker_active: active,
            ..**current
        });
        self.retired.push(old);
        Ok(())
    }

    /// Grace period by reference count: drop a version once no reader still holds it.
    pub fn reclaim(&mut self) -> usize {
        self.retired.retain(|table| Arc::strong_count(table) > 1);
        self.retired.len()
    }
}
```
