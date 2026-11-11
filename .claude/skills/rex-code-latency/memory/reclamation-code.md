# Memory reclamation: code

## Hazard domain over pool indices

Compile-checked: cargo check and clippy, aarch64-apple-darwin, x86_64-unknown-linux-gnu and aarch64-unknown-linux-gnu. Stress test on aarch64-apple-darwin, 20 runs: 3 reader threads protect and check, 1 writer swaps, retires and recycles 300,000 times. No reader saw a recycled slot. Not run under loom or Miri.

- [ ] Slots hold pool indices, so the domain needs no `unsafe`. Over raw addresses it hands back a `*mut T`, and `&T` from that needs `unsafe`: use `haphazard` there.
- [ ] One slot per core, each `CachePadded` (128 bytes on x86_64 and aarch64): a reader writes only its own line. Build the domain at startup.
- [ ] CAUTION: Read the node only between `protect` and `release_hazard_protection`. After the release, the writer can recycle the slot under the reader.
- [ ] Run `reclaim` in batches on a housekeeping core. Reserve `retired` at startup. The scan is O(retired × cores): for hundreds of retired indices, copy the slots into a fixed array and sort it first.
- [ ] Reader is lock-free, not wait-free: the loop retries while a fast writer keeps swapping.
- [ ] `fence(SeqCst)` = `mfence` on x86_64, `dmb ish` on aarch64.
- [ ] `CachePadded` needs no `std` feature.
- [ ] CAUTION: Do not call `crossbeam_utils::Backoff::snooze()` on the hotpath. With the `std` feature, which `crossbeam-epoch` turns on, it yields to the operating system. Use `spin()`.

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

Compile-checked: cargo check and clippy, aarch64-apple-darwin, x86_64-unknown-linux-gnu and aarch64-unknown-linux-gnu. Tests ran on aarch64-apple-darwin: an old version stays alive while a reader cache holds it and is freed after that reader loads again. A full retired list returns `RetiredListFull`.

- [ ] Reader: one relaxed load and a compare while the table is unchanged. After a swap, each reader pays once: one count increment and one drop of its old `Arc`.
- [ ] `Cache::load` takes `&mut self`: one `RoutingReader` per thread.
- [ ] Put this module in a crate without the `Arc` ban of `.claude/skills/rex-code-latency/os/jitter.md`. The hotpath crate holds a `RoutingReader` and never names `Arc`, so its ban stays whole.
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

impl LockFreeConfigurationManager {
    pub fn new(initial: ExchangeRoutingTable) -> Self {
        Self { global_routing_state: ArcSwap::from_pointee(initial) }
    }

    /// Call at startup, once for each reader thread.
    pub fn reader(&self) -> RoutingReader<'_> {
        RoutingReader { cache: Cache::new(&self.global_routing_state) }
    }

    /// Call at startup, on the housekeeping core.
    pub fn writer(&self, retired_capacity: usize) -> RoutingWriter<'_> {
        RoutingWriter { manager: self, retired: Vec::with_capacity(retired_capacity) }
    }
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
