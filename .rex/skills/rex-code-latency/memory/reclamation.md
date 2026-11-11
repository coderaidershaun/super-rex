# Memory reclamation

Free or reuse memory that another thread can still read in lock-free code.

## Symptom

- CAS succeeds, but a lock-free stack is corrupt: one node handed out twice.
- Reader crashes or reads garbage after a writer frees a node.
- Config read per packet, changed every few minutes.
- Read latency rises as more cores read one shared value.
- Memory grows while one reader stalls.

## Confirm

Measure as `.rex/skills/rex-code-latency/measure.md` says.

- ABA: replay test. Read head and next, pop A, pop B, push A, stale CAS. IF it succeeds, THEN ABA is real.
- `unsafe` code: `cargo +nightly miri test`. Hand-written atomics: `loom` + real-thread stress test on aarch64.
- Reader cost: time reads with 1, 2, 4, 8 reader cores. Flat = no shared write. IF not flat, THEN find `lock`, `xchg` or `mfence` in x86_64 assembly of read path.
- Memory growth: log retired-list length. Steady growth = reader stays in its read section.

## Rules

- [ ] First choice: no shared ownership on hotpath. One owner thread, changes over an SPSC ring buffer.
- [ ] CAUTION: Use an audited crate, not hand-written `unsafe` reclamation. Types do not test memory orders, and one wrong order gives a use-after-free.
- [ ] CAUTION: Unlink a node first, and free or recycle it only after every reader dropped its reference. A reader that still holds the pointer or index reads freed or reused memory.
- [ ] CAUTION: Do not trust CAS to detect a recycled node. CAS compares only the address or index, so a node that came back passes.

| IF | THEN |
| --- | --- |
| Shared free list of pool indices | `crossbeam_queue::ArrayQueue<u32>` (0.3.14, `alloc`): no ABA. FIFO, so next slot is cold in L1 |
| Lock-free stack over a fixed pool, by hand | 32-bit tag + 32-bit index in one `AtomicU64`. Not 128-bit CAS: std `AtomicU128` is nightly-only, `portable-atomic` locks on x86_64 without `cmpxchg16b` |
| One hot pointer, frequent updates, tight memory | `haphazard` |
| Read-mostly, updates minutes apart | `arc-swap` + per-thread `Cache` |
| Frequent writes, bounded memory | `left-right` |
| Shared lock-free map | `crossbeam-skiplist` `base::SkipList`, one guard per batch |
| Millions of updates per second | Not RCU: grace periods never end |

Hazard pointers:

- [ ] CAUTION: Put `fence(SeqCst)` between the hazard publish and the validation load. Without it, the store can wait in the store buffer while the writer frees the node.
- [ ] CAUTION: Put a matching `fence(SeqCst)` on the writer between the unlink and the scan. The Rust memory model needs both fences, whatever the CPU.
- [ ] CAUTION: Call `haphazard` `retire` only one time, and only on a pointer that no other `AtomicPtr` holds. Otherwise a reader can load freed memory, or the crate frees it twice.
- [ ] `haphazard` reader: `AtomicPtr::safe_load(&mut hp)`, then `hp.reset_protection()`. Writer: `swap(Box::new(next))`, then `unsafe { old.retire() }`.

RCU style:

- [ ] Never edit the live version. `ArcSwap::rcu(|cur| T { field: v, ..**cur })` copies, edits, publishes, retries.
- [ ] Keep each old `Arc` from `rcu` in a pre-sized retired `Vec` on writer. Drop it at `Arc::strong_count == 1`, on housekeeping core.
- [ ] Size pools for dozens of live old versions.

Threads:

- [ ] Create `HazardPointer`, `ReadHandle`, `Cache` and first `pin()` per thread at startup. First use can allocate.
- [ ] Run `retire`, `reclaim`, `publish`, epoch flush on housekeeping core only.
- [ ] Never call `Arc::clone`, `ArcSwap::load_full` or `ArcSwap::load` (`SeqCst` swap into a debt slot) on hotpath. Use `Cache::load`.

## Numbers

Typical figures. Measure on your own hardware.

| Item | Value |
| --- | --- |
| Hazard slot write | about 1 ns |
| Reader fence per `protect` | tens of cycles |

## Rust tools

- `crossbeam-epoch` 0.9.21, `std`: `Shared::deref` and `Guard::defer_destroy` are `unsafe`.
- `crossbeam-skiplist` 0.1.3, `std`: call `release(&guard)` on the entry from `insert`, or the node leaks.
- `haphazard` 0.1.8, `std`: last release December 2023. Allocates on first `HazardPointer::new` and each `retire`. `retire` scans inline at 1000 retired or every 2 s.
- `arc-swap` 1.9.5, `left-right` 0.11.8: no default features. Their write APIs need no `unsafe`.
- `left-right`: `enter()` writes its own counter + `fence(SeqCst)`. `publish` waits for the slowest reader: use `try_publish`.
- `seize` 0.5.1, `papaya` 0.2.5 (map): the thread that drops the last reference to a retired batch frees it. That can be a hotpath reader.
- `portable-atomic` 1.15.0: 128-bit CAS, lock-free on aarch64. x86_64: `cmpxchg16b`, detected at run time, else a lock.
- `loom` 0.7.3: misses load-buffering bugs.

## Traps

- [ ] Ownership does not stop ABA: with pool indices, safe Rust hands out one node twice, with no UB. LIFO pools (`slab` too) make it likelier.
- [ ] Epoch garbage grows while any thread holds an epoch pin. Every 128th pin runs destructors on that reader.

## Code

`.rex/skills/rex-code-latency/memory/reclamation-code.md`:

- Hazard domain over pool indices: IF you write hazard protection yourself.
- `arc-swap` config: IF you share rarely changed data.

IF you write a lock-free stack over a fixed pool, THEN read `.rex/skills/rex-code-latency/threads/atomics-code.md`.

## See also

- IF you write a CAS loop or pick memory orders, THEN read `.rex/skills/rex-code-latency/threads/atomics.md`.
- IF data can have one owner, THEN read `.rex/skills/rex-code-latency/threads/ring-buffers.md`.
- IF you build the node pool, THEN read `.rex/skills/rex-code-latency/memory/zero-alloc.md`.
