# Memory reclamation

Free or reuse memory that another thread can still read in lock-free code.

## Symptom

- CAS succeeds, but a lock-free stack is corrupt: one node handed out twice.
- Reader crashes or reads garbage after a writer frees a node.
- Config read per packet, changed every few minutes.
- Read latency rises as more cores read one shared value.
- Memory grows while one reader stalls.

## Confirm

Measure as `.claude/skills/rex-code-latency/measure.md` says.

- ABA: replay test. Read head and next, pop A, pop B, push A, stale CAS. IF it succeeds, THEN ABA is real.
- `unsafe` code: `cargo +nightly miri test`. Hand-written atomics: `loom` + real-thread stress test on aarch64.
- Reader cost: read the x86_64 assembly of the read path. Each `lock` op, `xchg` or `mfence` = shared write or full fence per read.
- Memory growth: log retired-list length. Steady growth = a reader never leaves its read section.

## Rules

- [ ] First choice: no shared ownership on the hotpath. One owner thread, changes over an SPSC ring buffer.
- [ ] CAUTION: Use an audited crate, not hand-written `unsafe` reclamation. Types do not test memory orders, and one wrong order gives a use-after-free.
- [ ] CAUTION: Unlink a node first, and free or recycle it only after every reader dropped its reference. A reader that still holds the pointer or index reads freed or reused memory.
- [ ] CAUTION: Do not trust CAS to detect a recycled node. CAS compares only the address or index, so a node that came back passes.

| IF | THEN |
| --- | --- |
| Lock-free stack over a fixed pool | 32-bit tag + 32-bit index in one `AtomicU64`. No 128-bit CAS: many ARM CPUs lack it |
| One hot pointer, frequent updates, tight memory | `haphazard` |
| Read-mostly, updates minutes apart | `arc-swap` + per-thread `Cache` |
| Frequent writes, bounded memory | `left-right` |
| Shared lock-free map | `crossbeam-skiplist` `base::SkipList`, one guard per batch |
| Millions of updates per second | Not RCU: pools run out, grace periods never end |

Hazard pointers:

- [ ] CAUTION: Put `fence(SeqCst)` between the hazard publish and the validation load. Without it, the store can wait in the store buffer while the writer frees the node.
- [ ] CAUTION: Put a matching `fence(SeqCst)` on the writer between the unlink and the scan. x86_64 hides a missing writer fence, but aarch64 does not.
- [ ] CAUTION: Call `haphazard` `retire` only one time, and only on a pointer that no other `AtomicPtr` holds. Otherwise a reader can load freed memory, or the crate frees it twice.
- [ ] `haphazard` reader: `AtomicPtr::safe_load(&mut hp)`, then `hp.reset_protection()`. Writer: `swap(Box::new(next))`, then `unsafe { old.retire() }`.

RCU style:

- [ ] Never edit the live version. `ArcSwap::rcu(|cur| T { field: v, ..**cur })` copies, edits, publishes, retries.
- [ ] Keep each old `Arc` from `rcu` in a pre-sized retired `Vec` on the writer. Drop it at `Arc::strong_count == 1`: the free runs on the housekeeping core.
- [ ] IF the retired list is full and nothing frees, THEN return a typed error.
- [ ] Size pools for dozens of old versions alive at once.

Threads:

- [ ] Create `HazardPointer`, `ReadHandle`, `Cache` and first `pin()` per thread at startup. First use can allocate.
- [ ] Run `retire`, `reclaim`, `publish`, epoch flush on a housekeeping core only.
- [ ] Never call `Arc::clone`, `ArcSwap::load_full` or `ArcSwap::load` (`SeqCst` swap into a debt slot) on the hotpath. Use `Cache::load`.

## Numbers

Typical figures. Measure on your own hardware.

| Item | Value |
| --- | --- |
| Hazard slot write | about 1 ns |
| RCU fit | tens of millions of reads per second, update every few minutes |

## Rust tools

- `crossbeam-epoch` 0.9.21, `std`: `Shared::deref` and `Guard::defer_destroy` are `unsafe`.
- `crossbeam-skiplist` 0.1.3, `std`: call `release(&guard)` on the entry from `insert`, or the node leaks.
- `haphazard` 0.1.8, `std`: last release December 2023. Allocates on first `HazardPointer::new` and each `retire`. `retire` scans inline at 1000 retired or every 2 s.
- `arc-swap` 1.9.5, `left-right` 0.11.8: no default features. Only safe write paths.
- `left-right`: `enter()` writes its own counter + `fence(SeqCst)`. `publish` waits for the slowest reader: use `try_publish`.
- `seize` 0.5.1, `papaya` 0.2.5 (map): memory stays bounded under a stalled reader.
- `portable-atomic` 1.15.0: real 128-bit CAS. Without `cmpxchg16b` or LSE it takes a lock.
- `loom` 0.7.3: misses load-buffering bugs.

## Traps

- [ ] Ownership does not stop ABA. With pool indices, safe Rust hands out one node twice, with no UB. LIFO pools (`slab` too) make it more likely.
- [ ] A reader that stops calling `Cache::load` keeps its old version forever.
- [ ] Epoch garbage grows while any thread holds an epoch pin. Every 128th epoch pin runs destructors on that reader.

## Code

`.claude/skills/rex-code-latency/memory/reclamation-code.md`:

- Tagged head: IF you write a lock-free stack over a fixed pool.
- Hazard domain over pool indices: IF you write hazard protection yourself.
- `arc-swap` config: IF you share rarely changed data.

## See also

- IF you write a CAS loop or pick memory orders, THEN read `.claude/skills/rex-code-latency/threads/atomics.md`.
- IF data can have one owner, THEN read `.claude/skills/rex-code-latency/threads/ring-buffers.md`.
- IF you build the node pool, THEN read `.claude/skills/rex-code-latency/memory/zero-alloc.md`.
