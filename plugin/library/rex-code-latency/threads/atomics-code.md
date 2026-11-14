# Atomics code

Loom test set-up and a lock-free stack with a compare-and-swap (CAS) loop and backoff.

## Loom shim: one import point for atomics

Compile-checked: cargo check + clippy, x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu and aarch64-apple-darwin, with and without `--cfg loom`.

`Cargo.toml` lines:

```toml
[dependencies]
crossbeam-utils = { version = "0.8.23", default-features = false }
thiserror = { version = "2.0.21", default-features = false }

[target.'cfg(loom)'.dependencies]
loom = { version = "0.7.3", default-features = false }

[lints.rust]
unexpected_cfgs = { level = "warn", check-cfg = ["cfg(loom)"] }
```

`src/sync.rs`:

```rust
//! One import point for atomics, so `--cfg loom` swaps std for the loom model checker.
#[cfg(loom)]
pub(crate) use loom::{
    hint::spin_loop,
    sync::Arc,
    sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
};
#[cfg(not(loom))]
pub(crate) use std::{
    hint::spin_loop,
    sync::Arc,
    sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
};
```

Commands:

- loom: `RUSTFLAGS="--cfg loom" cargo test --release --test <loom test file>`. IF a model runs too long, THEN set `LOOM_MAX_PREEMPTIONS=2` or `3`.
- Miri: `cargo +nightly miri test`. Add `-Zmiri-many-seeds` to `MIRIFLAGS` to try many schedules. Cut loop counts under `cfg!(miri)`. Code under test stays stable Rust.
- Miri cannot run `asm!`. Gate such code with `#[cfg(not(miri))]` in modules that Miri tests.

Notes:

- [ ] Every atomic, `Arc` and `spin_loop` in the crate comes from `crate::sync`. One import from `std` hides that access from loom.
- [ ] Remove each name the crate does not use from both lists. With the stack below as the only user, `AtomicBool`, `Arc` and `spin_loop` give an unused-import warning.
- [ ] `crossbeam_utils::Backoff` calls `std::hint::spin_loop`, not the loom one. OK in a CAS retry loop: each retry follows a success of another thread. Not OK in a loop that waits for another thread: loom does not end.

## Lock-free stack over a node pool: CAS loop with backoff

Compile-checked: cargo check and clippy, aarch64-apple-darwin, x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu. Tested with 4 real threads and with loom.

```rust
//! Lock-free stack over a node pool built at startup: tagged head, CAS loop, exponential spin backoff.
use crate::sync::{
    AtomicU32, AtomicU64,
    Ordering::{Acquire, Relaxed, Release},
};
use crossbeam_utils::{Backoff, CachePadded};
use thiserror::Error;

const EMPTY: u32 = u32::MAX;

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum StackError {
    #[error("node index {index} is outside the pool of {capacity} nodes")]
    BadIndex { index: u32, capacity: usize },
}

struct OrderStackNode {
    order_id: AtomicU64,
    next_node: AtomicU32,
}

pub struct OrderStack {
    // High 32 bits: tag, bumped on every change. Low 32 bits: index of the top node.
    head: CachePadded<AtomicU64>,
    nodes: Box<[OrderStackNode]>,
}

const fn pack(tag: u32, index: u32) -> u64 {
    ((tag as u64) << 32) | index as u64
}
const fn tag_of(head: u64) -> u32 {
    (head >> 32) as u32
}
const fn index_of(head: u64) -> u32 {
    head as u32
}

impl OrderStack {
    /// Allocates every node at startup. The hotpath never allocates.
    pub fn with_capacity(capacity: u32) -> Self {
        let nodes = (0..capacity)
            .map(|_| OrderStackNode {
                order_id: AtomicU64::new(0),
                next_node: AtomicU32::new(EMPTY),
            })
            .collect();
        Self {
            head: CachePadded::new(AtomicU64::new(pack(0, EMPTY))),
            nodes,
        }
    }

    /// The caller must own pool slot `index`: it is not on the stack now.
    pub fn push(&self, index: u32, order_id: u64) -> Result<(), StackError> {
        let capacity = self.nodes.len();
        let node = self
            .nodes
            .get(index as usize)
            .ok_or(StackError::BadIndex { index, capacity })?;
        // Fill the node before it is shared, so the window between load and swap stays short.
        node.order_id.store(order_id, Relaxed);

        let backoff = Backoff::new();
        let mut expected = self.head.load(Relaxed);
        loop {
            // Inside the loop: each retry links to the head that the failed CAS returned.
            node.next_node.store(index_of(expected), Relaxed);
            let desired = pack(tag_of(expected).wrapping_add(1), index);
            match self
                .head
                .compare_exchange_weak(expected, desired, Release, Relaxed)
            {
                Ok(_) => return Ok(()),
                Err(found) => {
                    expected = found;
                    backoff.spin();
                }
            }
        }
    }

    /// Returns the pool slot index and its order id. The caller then owns that slot.
    pub fn pop(&self) -> Option<(u32, u64)> {
        let backoff = Backoff::new();
        let mut expected = self.head.load(Acquire);
        loop {
            let index = index_of(expected);
            // EMPTY is never a valid index, so an empty stack returns None here.
            let node = self.nodes.get(index as usize)?;
            let desired = pack(
                tag_of(expected).wrapping_add(1),
                node.next_node.load(Relaxed),
            );
            match self
                .head
                .compare_exchange_weak(expected, desired, Acquire, Acquire)
            {
                Ok(_) => return Some((index, node.order_id.load(Relaxed))),
                Err(found) => {
                    expected = found;
                    backoff.spin();
                }
            }
        }
    }
}
```

Notes:

- [ ] Tag + index in one `AtomicU64`: the tag changes on each push and pop, so a stale head never matches (ABA). It wraps after 2^32 changes. ABA then needs one thread to stall across 2^32 operations.
- [ ] Pool indices, not `AtomicPtr`: no `unsafe`, no reclamation scheme.
- [ ] Pointer + tag in 16 bytes: `std` `AtomicU128` is nightly only. `portable_atomic::AtomicU128` is lock-free on every aarch64 CPU. On x86_64 it detects `cmpxchg16b` at run time and takes a lock without it. `-C target-feature=+cmpxchg16b` removes the check.
- [ ] `pop` uses `Acquire` on its load and on both CAS orders: it reads `next_node` and `order_id` that another core wrote.
- [ ] CAUTION: Do not push a slot that is already on the stack. It corrupts the list. That is a logic bug, not undefined behavior, so Miri does not report it. The caller must own slot bookkeeping.
- [ ] `Backoff::spin()` runs `1 << step.min(6)` calls of `spin_loop()`. Hand-written form: `for _ in 0..1u32 << failures.min(6) { std::hint::spin_loop(); }`.

## Loom model test shape

```
pseudocode
// tests/loom_stack.rs, starts with #![cfg(loom)]
#[test] fn concurrent_pop_sees_the_pushed_order_id() {
    loom::model(|| {
        stack = loom::sync::Arc::new(OrderStack::with_capacity(2)); other = clone of stack
        pusher = loom::thread::spawn(move || { other.push(0, 10); other.push(1, 11) })
        IF let Some((index, order_id)) = stack.pop() THEN assert order_id == 10 + index
        pusher.join()
    })
}
```

Keep each model small: two or three threads, two or three operations each. loom runs the closure once for each legal interleaving.
