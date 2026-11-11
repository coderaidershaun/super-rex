# Ring buffers code

A sequence lock with no data race in Rust, and the layout of an SPSC ring for the rare case that no crate fits.

## Sequence lock with atomic payload words

Compile-checked: cargo check + clippy, x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu and aarch64-apple-darwin. Run with 4 reader threads and 2,000,000 writes on aarch64-apple-darwin: no torn read seen. A loom model of these orderings passes, and each of four weaker orderings fails with a torn read: no writer fence, no reader fence, `Relaxed` first load, `Relaxed` even store. Not run under Miri.

`Cargo.toml` lines:

```toml
[dependencies]
crossbeam-utils = { version = "0.8.23", default-features = false }
```

```rust
//! Sequence lock: one writer publishes risk limits to many readers that never write shared memory.
use std::hint::spin_loop;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering::{Acquire, Relaxed, Release}, fence};

use crossbeam_utils::CachePadded;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RiskLimitPayload {
    pub maximum_financial_exposure: f64,
    pub restricted_instrument_identifier: u32,
    pub is_trading_globally_halted: bool,
}

struct SeqLockCell {
    sequence_counter: AtomicU64,
    words: [AtomicU64; 2],
}

// Counter and payload share one line, so a reader fetches both in one miss.
const _: () = assert!(size_of::<SeqLockCell>() <= 64);
const _: () = assert!(align_of::<CachePadded<SeqLockCell>>() >= 64);

/// The only handle that writes. It is not Clone, so a second writer cannot exist.
pub struct RiskLimitWriter(Arc<CachePadded<SeqLockCell>>);

#[derive(Clone)]
pub struct RiskLimitReader(Arc<CachePadded<SeqLockCell>>);

pub fn risk_limits() -> (RiskLimitWriter, RiskLimitReader) {
    let start = RiskLimitPayload {
        maximum_financial_exposure: 0.0,
        restricted_instrument_identifier: 0,
        is_trading_globally_halted: true,
    };
    let [w0, w1] = encode(start);
    let cell = Arc::new(CachePadded::new(SeqLockCell {
        sequence_counter: AtomicU64::new(0),
        words: [AtomicU64::new(w0), AtomicU64::new(w1)],
    }));
    (RiskLimitWriter(Arc::clone(&cell)), RiskLimitReader(cell))
}

impl RiskLimitWriter {
    pub fn update_limits(&mut self, new: RiskLimitPayload) {
        let cell = &**self.0;
        let even = cell.sequence_counter.load(Relaxed);
        cell.sequence_counter.store(even + 1, Relaxed);
        // Keeps the payload stores below from moving above the odd store.
        fence(Release);
        let [w0, w1] = encode(new);
        cell.words[0].store(w0, Relaxed);
        cell.words[1].store(w1, Relaxed);
        cell.sequence_counter.store(even + 2, Release);
    }
}

impl RiskLimitReader {
    pub fn read_limits(&self) -> RiskLimitPayload {
        let cell = &**self.0;
        loop {
            let initial_sequence = cell.sequence_counter.load(Acquire);
            if initial_sequence & 1 != 0 {
                spin_loop();
                continue;
            }
            let copy = [cell.words[0].load(Relaxed), cell.words[1].load(Relaxed)];
            fence(Acquire);
            let validation_sequence = cell.sequence_counter.load(Relaxed);
            if initial_sequence == validation_sequence {
                return decode(copy);
            }
            spin_loop();
        }
    }
}

fn encode(p: RiskLimitPayload) -> [u64; 2] {
    let flags = u64::from(p.restricted_instrument_identifier)
        | (u64::from(p.is_trading_globally_halted) << 32);
    [p.maximum_financial_exposure.to_bits(), flags]
}

fn decode([w0, w1]: [u64; 2]) -> RiskLimitPayload {
    RiskLimitPayload {
        maximum_financial_exposure: f64::from_bits(w0),
        restricted_instrument_identifier: w1 as u32,
        is_trading_globally_halted: (w1 >> 32) & 1 == 1,
    }
}
```

Notes:

- [ ] No `unsafe`: every payload access is a `Relaxed` atomic. On x86_64 and aarch64 that is the same `mov`, `ldr` or `str` as a plain copy.
- [ ] Word 0 = `f64::to_bits`. Word 1 = instrument in the low 32 bits, halt flag in bit 32.
- [ ] Halt flag decodes from one bit test. A torn byte that is not 0 or 1 is undefined behavior as a `bool`. A bit test cannot make an invalid `bool`.
- [ ] Counter is `AtomicU64`. At one write each nanosecond it wraps after about 290 years.
- [ ] Start state is the safe state (`is_trading_globally_halted: true`). A reader that runs before the first update does not trade.
- [ ] Clone `RiskLimitReader` once per reader thread at startup. The clone is one atomic increment. `read_limits(&self)` writes nothing shared.
- [ ] Each handle holds an `Arc` that is made at start and only dereferenced on the hotpath. IF the hotpath crate bans `Arc` with clippy `disallowed-types`, THEN allow it as `.rex/skills/rex-code-latency/os/jitter.md` says: `#![expect(clippy::disallowed_types, reason = "...")]` on the module that holds this listing. Code that holds only `RiskLimitReader` needs no allow. The narrower form also passes: the same `#[expect]` on the `use std::sync::Arc` line, on both handle fields and on `fn risk_limits`.
- [ ] Generic payload: bound `T: bytemuck::Pod` and copy with `atomic-memcpy` 0.2.3 `atomic_load` / `atomic_store`. Their API is `unsafe` (raw pointers), but the copy is not a data race.
- [ ] Two writer threads: make them hand updates to one writer thread. Do not share `RiskLimitWriter`.

## Hand-written SPSC ring: layout and orderings

Use only when `rtrb` and `ringbuf` do not fit, for example a ring across processes in shared memory. Rules and pseudocode only: no checked listing.

- [ ] Capacity = const generic `N`. `const { assert!(N.is_power_of_two()) }`. Wrap with `& (N - 1)`. No `%` by a run-time value: it is an integer division. `%` by a const power of two compiles to an AND.
- [ ] Producer alone writes the write index. Consumer alone writes the read index. No CAS, no backoff.
- [ ] Orderings. A loom model of this table (N = 2, three values) passes, and each of three weaker orderings fails: `Relaxed` on either index store, or on the load of the other side's index.

| Access | Who | Ordering |
| --- | --- | --- |
| Load of own index | owner | `Relaxed`, or none: keep it in a handle field |
| Load of other side's index | producer, consumer | `Acquire` |
| Store of write index, after the slot write | producer | `Release` |
| Store of read index, after the slot read | consumer | `Release` |

- [ ] Cached index: plain `usize` copy of the other side's index, in the owner's handle. Refresh it with one `Acquire` load only when it says full (producer) or empty (consumer). Test again after the refresh. Indices only move forward, so a stale copy only underestimates space or data.
- [ ] Each shared index in its own `CachePadded` (128 bytes on x86_64 and aarch64). Cached copies stay in the handles. A cached copy on an index line brings back false sharing.
- [ ] One `Producer`, one `Consumer`, neither `Clone`. `push` and `pop` take `&mut self`, so the cached copies need no atomic.
- [ ] Slots: `Box<[UnsafeCell<MaybeUninit<T>>; N]>`, indexed with `i & (N - 1)`, so the compiler drops the bounds check. Not `get_unchecked`. Build once at startup: collect a `Box<[_]>`, then `try_into()`, so no array passes through the stack. Touch each page once: `MaybeUninit::uninit()` touches no page, so the first write faults.
- [ ] CAUTION: Write the slot rule in the `// SAFETY:` comment of `unsafe impl<T: Send> Sync`: the producer writes only slots outside `[read, write)`, and the consumer reads only slots inside it. Any other slot access is a data race.
- [ ] `T: Copy`, or drop the items left in `[read, write)` when the ring drops.
- [ ] Full returns `Err(Full(value))`, so the caller keeps the value.
- [ ] Masked stored indices leave one slot unused: full = `next_write == read`. Positions in `0..2 * N` use every slot.
- [ ] Test with Miri and loom before first use.

```
pseudocode
push(&mut self, value) -> Result<(), Full<T>>:
    next = (self.write + 1) & (N - 1)              // self.write: own index, a handle field
    IF next == self.cached_read THEN
        self.cached_read = shared.read_index.load(Acquire)
        IF next == self.cached_read THEN return Err(Full(value))
    unsafe { slot[self.write] = value }            // SAFETY: slot is outside [read, write)
    shared.write_index.store(next, Release)
    self.write = next; return Ok(())
pop has the same shape: cached_write, refresh on empty, read slot, read_index.store(next, Release)
```
