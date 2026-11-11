# Atomics

Orderings, fences, compare-and-swap (CAS) loops, data races between threads, and their tests.

## Symptom

- Lock-free code passes on x86_64, fails or hangs on aarch64 (Graviton, Ampere, Apple M).
- Shared counter or flag costs far more than its instructions: `lock` or `xchg` in assembly, or `bl __aarch64_*` calls.
- Code writes `SeqCst` "to be safe", or nobody knows which `Ordering` fits.
- CAS loop gets slower as cores are added.

## Confirm

Measure as `.rex/skills/rex-code-latency/measure.md` says.

1. Read hot fn assembly for both targets (`cargo asm`). x86_64 `lock` or `xchg` = read-modify-write or `SeqCst` store.
2. Time it with a bounded probe. Compare with Numbers.
3. IF CAS throughput falls as cores are added, THEN cause = CAS target line bouncing.
4. Correctness: loom, Miri (`cargo +nightly miri test`), real-thread stress test on aarch64 hardware: QEMU on x86_64 hides its reordering.

## Rules

| Access | Who | Ordering |
| --- | --- | --- |
| Store of flag or index that publishes data | writer | `Release` |
| Load of that flag or index, before data reads | reader | `Acquire` |
| Data fields under that flag | both | `Relaxed` atomics |
| Own index | owner | `Relaxed` |
| Counter, one writer (own padded per-core slot too) | owner | `store(load(Relaxed).wrapping_add(1), Relaxed)`: no locked instruction |
| Counter, more than one writer | each | `fetch_add(1, Relaxed)` |
| CAS that publishes a node | pusher | success `Release`, failure `Relaxed` |
| CAS, then read of node another core wrote | popper | load, success, failure: `Acquire` |
| Read-modify-write that reads and publishes | any | `AcqRel` |
| Store to own slot, then load of other thread's slot, both atomic | both | `fence(SeqCst)` between, both sides |

Stream handoff on one flag:

CAUTION: Reset the flag with `Release`, and make the producer wait for the reset. Else the producer overwrites a payload that the consumer still reads.

1. Producer: spin until `flag.load(Acquire)` is `false`. Store payload, `Relaxed`. `flag.store(true, Release)`.
2. Consumer: spin until `flag.load(Acquire)` is `true`. Load payload, `Relaxed`. `flag.store(false, Release)`.

- [ ] `SeqCst` only where several threads must agree on one order of several atomics.
- [ ] `Relaxed` only when the value gates no other data.
- [ ] `compiler_fence` = compiler only, 0 instructions. `fence` = compiler + CPU, and orders only through an atomic that one side stores and the other loads.
- [ ] CAS: `compare_exchange_weak` in own `loop`. On `Err(found)`: `expected = found`, back off, retry. Not `fetch_update`: no place for backoff.
- [ ] Build new state before the loop. Redo link stores inside it.
- [ ] Back off with `crossbeam_utils::Backoff::spin()`. Never `thread::sleep`, `thread::yield_now` or `Backoff::snooze()` on hotpath.
- [ ] A loop that waits for another thread (CAS retry, flag, ring) calls `std::hint::spin_loop()`. Exception: a busy-poll loop on an isolated core, SMT off, that must react in fewest cycles can drop it: `pause` costs about 100 cycles on recent Intel. Check on your host.
- [ ] ABA: pool index + 32-bit tag in one `AtomicU64`. Tag + 1 on each change.
- [ ] CAUTION: Make every shared access atomic, or prove exclusive access in a `// SAFETY:` comment. A data race (one location, at least one write, at least one non-atomic access, no happens-before) is undefined behavior, even when code discards the value.
- [ ] CAUTION: Do not view one `AtomicU64` as two `AtomicU32`. Racing atomic accesses of different sizes are undefined behavior.

## Numbers

Typical figures. Measure on your own hardware.

| Operation | Cost |
| --- | --- |
| `SeqCst` store, contended read-modify-write, `SeqCst` fence | over 100 cycles |
| Relaxed store, one-writer counter | about 1 ns |
| `fence(Release)`, `fence(SeqCst)` | x86_64: none, `mfence`. aarch64: `dmb ish` |

## Rust tools

| Crate | Use | Not obvious |
| --- | --- | --- |
| `crossbeam-utils` 0.8.23, `default-features = false` | `Backoff`, `CachePadded` (128 bytes) | `snooze()` yields to OS IF any crate enables `std` (`crossbeam-epoch` with `std` does). |
| `portable-atomic` 1.15.0 | `AtomicU128` (std: nightly only), `AtomicF64` (feature `float`) | x86_64: `cmpxchg16b`, detected at run time, else a lock. aarch64: always lock-free. Without LSE2, a 128-bit load writes the line. |
| `loom` 0.7.3 | model checker | Under `[target.'cfg(loom)'.dependencies]`. |

## Traps

- x86_64 reorders only a load before an earlier store. aarch64 reorders more: wrong orderings fail there.
- Panic at run time: `load(Release)`, `store(Acquire)`, `AcqRel` on either, `fence(Relaxed)`, `compiler_fence(Relaxed)`, CAS failure `Release` or `AcqRel`.
- No `Consume`: use `Acquire`. No atomic float: `f64::to_bits` into `AtomicU64`.
- `read_volatile`, `write_volatile`: not atomic, no thread order, device registers only. `fence(Release)` does not order device memory.
- loom: `SeqCst` loads and stores act as `AcqRel` (false alarms). `fence(SeqCst)` is modelled, load buffering is not. It passed three `Relaxed` bugs: flag reset, consumer flag load, push CAS.
- aarch64 Linux: each read-modify-write is a call to a `__aarch64_*` helper. `-C target-cpu=neoverse-n1` (or `+lse`) inlines it, and the binary then needs LSE. Check on your host.

## Code

In `.rex/skills/rex-code-latency/threads/atomics-code.md`: "Loom shim", "Lock-free stack" (CAS loop), "Loom model test shape".

## See also

- IF you pass messages or share state between cores, THEN read `.rex/skills/rex-code-latency/threads/ring-buffers.md`.
- IF lock-free nodes get freed, THEN read `.rex/skills/rex-code-latency/memory/reclamation.md`.
- IF cores write nearby atomics, THEN read `.rex/skills/rex-code-latency/cpu/cache-line-layout.md`.
