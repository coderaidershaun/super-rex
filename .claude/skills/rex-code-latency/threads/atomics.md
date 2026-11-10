# Atomics

Orderings, fences, compare-and-swap (CAS) loops, data races between threads, and their tests.

## Symptom

- Lock-free code passes on x86_64, fails or hangs on aarch64.
- Shared counter or flag costs far more than its instructions: `lock` or `xchg` in assembly.
- Code writes `SeqCst` "to be safe", or nobody knows which `Ordering` to write.
- CAS loop gets slower as cores are added.

## Confirm

Method: `.claude/skills/rex-code-latency/measure.md`.

1. Read hot fn assembly for both targets (`cargo asm`). x86_64 `lock` or `xchg` = read-modify-write or `SeqCst` store.
2. Time it with a bounded probe. Expect the costs in Numbers.
3. IF CAS throughput falls as cores are added, THEN cause = cache line bouncing on the CAS target.
4. Correctness: loom, Miri (`cargo +nightly miri test`), real-thread stress test on aarch64.

## Rules

| Access | Who | Ordering |
| --- | --- | --- |
| Store of flag or index that publishes data | writer | `Release` |
| Load of that flag or index, before data reads | reader | `Acquire` |
| Data fields under that flag | both | `Relaxed` atomics |
| Own index, own counter | owner | `Relaxed` |
| Counter, exactly one writer | owner | `store(load(Relaxed).wrapping_add(1), Relaxed)` |
| Counter, many writers | each | `fetch_add(1, Relaxed)` in own `CachePadded` |
| CAS that publishes a node | pusher | success `Release`, failure `Relaxed` |
| CAS, then read of node another core wrote | popper | load, success, failure: `Acquire` |
| Read-modify-write that reads and publishes | any | `AcqRel` |
| Store to own slot, then load of other thread's slot | both | `fence(SeqCst)` between, both sides |

Stream handoff on one flag, in this order.

CAUTION: Reset the flag with `Release`, and make the producer wait for the reset. Else the producer overwrites a payload that the consumer still reads.

1. Producer: spin until `flag.load(Acquire)` is `false`. Store payload, `Relaxed`. `flag.store(true, Release)`.
2. Consumer: spin until `flag.load(Acquire)` is `true`. Load payload, `Relaxed`. `flag.store(false, Release)`.

- [ ] `SeqCst` only where several threads must agree on one order of several atomics. In review, name that order for each `SeqCst`.
- [ ] `Relaxed` only when the value gates no other data.
- [ ] `compiler_fence` = compiler only, 0 instructions. `fence` = compiler + CPU. A fence orders only through an atomic that one side stores and the other loads.
- [ ] CAS: `compare_exchange_weak` in own `loop`. On `Err(found)`: `expected = found`, back off, retry. Not `fetch_update`: it has no place for backoff.
- [ ] Build new state before the loop. Redo link stores inside it: each retry links to the new head.
- [ ] Back off with `crossbeam_utils::Backoff::spin()`. Never `thread::sleep`, `thread::yield_now` (`sched_yield` syscall) or `Backoff::snooze()` on hotpath.
- [ ] Every spin loop calls `std::hint::spin_loop()`.
- [ ] ABA: pool index + 32-bit tag in one `AtomicU64`. Tag + 1 on each change.
- [ ] CAUTION: Make every shared access atomic, or prove exclusive access in a `// SAFETY:` comment. A data race (one location, at least one write, at least one non-atomic access, no happens-before) is undefined behavior, even when code discards the value.
- [ ] CAUTION: Do not view one `AtomicU64` as two `AtomicU32`. Racing atomic accesses of different sizes are undefined behavior.

## Numbers

Typical figures. Measure on your own hardware.

| Operation | Cost |
| --- | --- |
| `SeqCst` store, contended read-modify-write, `SeqCst` fence | over 100 cycles |
| Relaxed store, one-writer counter | about 1 ns |
| `fetch_add(1, Relaxed)`, x86_64 | `lock incq`, full barrier |
| `fence(Release)`, `fence(SeqCst)` | x86_64: none, `mfence`. aarch64: `dmb ish` |
| `spin_loop()` | x86_64: `pause`. aarch64: `isb sy` |

## Rust tools

| Crate | Use | Not obvious |
| --- | --- | --- |
| `crossbeam-utils` 0.8.23, `default-features = false` | `Backoff`, `CachePadded` (128 bytes) | Defaults off: `snooze()` never yields. |
| `portable-atomic` 1.15.0 | `AtomicU128` (std: nightly only), `AtomicF64` (feature `float`) | Needs `cmpxchg16b` or LSE, else a lock. |
| `loom` 0.7.3 | model checker | Under `[target.'cfg(loom)'.dependencies]`. |

## Traps

- x86_64 reorders only a load before an earlier store. aarch64 reorders more: wrong orderings fail there.
- Panic at run time: `load(Release)`, `store(Acquire)`, `AcqRel` on either, `fence(Relaxed)`, `compiler_fence(Relaxed)`, CAS failure `Release` or `AcqRel`.
- No default ordering. No `Consume`: use `Acquire`. No atomic float: `f64::to_bits` into `AtomicU64`.
- `read_volatile`, `write_volatile`: not atomic, no thread order, device registers only. `fence(Release)` does not order device memory.
- IF another crate turns on `crossbeam-utils/std`, THEN feature unification makes `snooze()` call `yield_now`.
- loom treats `SeqCst` as `AcqRel` and skips load buffering. It passed three `Relaxed` bugs: flag reset, consumer flag load, push CAS.
- Under loom, spin with `loom::hint::spin_loop`, or the model does not end.

## Code

- IF you set up loom, THEN read "Loom shim" in `.claude/skills/rex-code-latency/threads/atomics-code.md`.
- IF you write a CAS loop, THEN read "Lock-free stack" in the same file.
- IF you write a loom test, THEN read "Loom model test shape" in the same file.

## See also

- IF you pass messages or share state between cores, THEN read `.claude/skills/rex-code-latency/threads/ring-buffers.md`.
- IF lock-free nodes get freed, THEN read `.claude/skills/rex-code-latency/memory/reclamation.md`.
- IF cores write nearby atomics, THEN read `.claude/skills/rex-code-latency/cpu/cache-line-layout.md`.
- IF you write a NIC doorbell, THEN read `.claude/skills/rex-code-latency/network/kernel-bypass.md`.
