---
name: rex-code-latency
description: >
  Reference for Rust hotpath latency of nanoseconds to tens of microseconds,
  on a Linux host you control: measure, then cause, proof, fix and numbers for
  jitter, caches, allocation, atomics, queues, network and timing. Not for
  millisecond services.
when_to_use: >
  Use when a Rust hotpath has a latency problem: "p99", "tail latency",
  "jitter", "lock-free", "false sharing", "kernel bypass", "busy poll", "time
  this function".
disable-model-invocation: false
user-invocable: true
---

# Code latency

Routes one Rust hotpath latency problem to its topic file.

## Scope

- For: hotpath work of ns to tens of µs, on a Linux host you control.
- IF median is in ms, or p99 is in ms from a database, RPC or web call, THEN stop: this skill does not apply. Find slow call first: sampling profiler, `tracing` spans, `tokio-console`.
- A µs hotpath with rare ms spikes is in scope.

## Use

1. Measure first: read `.rex/skills/rex-code-latency/measure.md` up to `## Check measurement itself`. Read rest IF numbers look wrong or a row sends you there.
2. Match one Router row. Read its first file.
3. Read its second file only IF first file sends you there or does not answer. Max 2 topic files.
4. Fix only a cause that `## Confirm` of topic file proved.
5. Open a `-code.md` file only when you write its code.

## Router

| You see or ask | Read first | Then |
| --- | --- | --- |
| Spike pattern not known yet: p99 or max many times median. Good in benchmark, bad on live host. | `.rex/skills/rex-code-latency/os/jitter.md` | `.rex/skills/rex-code-latency/os/cpu-isolation.md` |
| Known pattern on pinned thread: spikes every 1–10 ms, random multi-µs, or after quiet period. How to pin? | `.rex/skills/rex-code-latency/os/cpu-isolation.md` | `.rex/skills/rex-code-latency/os/huge-pages.md` |
| First events slow (warm-up). Spike on first touch of new memory. Rare stall of hundreds of µs. | `.rex/skills/rex-code-latency/os/huge-pages.md` | `.rex/skills/rex-code-latency/memory/zero-alloc.md` |
| Random reads of large table stall, high dTLB misses. | `.rex/skills/rex-code-latency/os/huge-pages.md` | `.rex/skills/rex-code-latency/cpu/cache-hierarchy.md` |
| Spikes in `Box::new`, `Vec::push`, `format!`, or when last `Arc` drops. Does hotpath allocate? | `.rex/skills/rex-code-latency/memory/zero-alloc.md` | `.rex/skills/rex-code-latency/os/huge-pages.md` |
| Hot loop slower as data grows, IPC below 1.0. Linked list, `BTreeMap`, `Box` per entity, hash lookup. | `.rex/skills/rex-code-latency/cpu/cache-hierarchy.md` | `.rex/skills/rex-code-latency/os/huge-pages.md` |
| Two threads each write own variable, both slower. Hot struct over 64 bytes. `CachePadded` or `align(64)`? | `.rex/skills/rex-code-latency/cpu/cache-line-layout.md` | `.rex/skills/rex-code-latency/threads/atomics.md` |
| Slow on real data, fast on test data, high `branch-misses`. `dyn Trait`. Make `if` branchless? `likely`? | `.rex/skills/rex-code-latency/cpu/branching.md` | `.rex/skills/rex-code-latency/cpu/cache-hierarchy.md` |
| Messages through `Mutex<VecDeque>`, `mpsc` or blocking channel. One writer, many readers of numbers and flags: torn read (half old, half new), or `RwLock` slow. | `.rex/skills/rex-code-latency/threads/ring-buffers.md` | `.rex/skills/rex-code-latency/threads/atomics.md` |
| Lock-free code fails on aarch64 only (Graviton, Ampere, Apple M). Which `Ordering`? CAS loop slows with more cores. | `.rex/skills/rex-code-latency/threads/atomics.md` | `.rex/skills/rex-code-latency/threads/ring-buffers.md` |
| Lock-free stack corrupt after CAS succeeds. Reader reads freed node. Many readers of state with `Vec`, `String`, `Box` or map. | `.rex/skills/rex-code-latency/memory/reclamation.md` | `.rex/skills/rex-code-latency/threads/atomics.md` |
| Receive waits in `tokio`, `mio`, `epoll` or `std::net`: µs per packet, slow after quiet gap. | `.rex/skills/rex-code-latency/network/sockets.md` | `.rex/skills/rex-code-latency/os/cpu-isolation.md` |
| Move feed to DPDK, AF_XDP, `ef_vi` or OpenOnload? Bypass sends corrupt frames. | `.rex/skills/rex-code-latency/network/kernel-bypass.md` | `.rex/skills/rex-code-latency/network/sockets.md` |
| Cause unknown, or no row fits: cache, branch, kernel or allocation? | `.rex/skills/rex-code-latency/measure.md` | `.rex/skills/rex-code-latency/measurement/counters.md` |
| Time a few-ns function: `Instant::now()` costs more, reads 0, coarse on aarch64. | `.rex/skills/rex-code-latency/measurement/counters.md` | `.rex/skills/rex-code-latency/measurement/logging.md` |
| p99 rises with logging on. `log`, `tracing` or `println!` on hotpath. Keep every sample. | `.rex/skills/rex-code-latency/measurement/logging.md` | `.rex/skills/rex-code-latency/threads/ring-buffers.md` |
| After a change: benchmark faster, production p99 same. | `.rex/skills/rex-code-latency/measure.md` | `.rex/skills/rex-code-latency/measurement/logging.md` |
| New host or process: boot line, IRQs, CPU frequency, start order. | `.rex/skills/rex-code-latency/setup.md` | `.rex/skills/rex-code-latency/os/cpu-isolation.md` |

## Shared files

| File | Read when |
| --- | --- |
| `.rex/skills/rex-code-latency/measure-code.md` | You build timing harness. |
| `.rex/skills/rex-code-latency/setup.md` | New host, start-up order. |
| `.rex/skills/rex-code-latency/crates.md` | You add a crate. |
| `.rex/skills/rex-code-latency/glossary.md` | Term not clear. |

## Rules for hotpath and measurement

- [ ] Judge hotpath by p99, p99.9, max. Never by mean: it hides spikes.
- [ ] Hotpath: no syscall, lock, allocation, I/O, `format!`, `log` or `tracing`. Slow path can use them.
- [ ] Hotpath never waits for slow path. Bridge = SPSC ring buffer (`rtrb`). Ring full: drop event, count drop.
- [ ] Pay hotpath costs at start: pin thread, then allocate, pre-fault, build rings, prove core quiet.
- [ ] Change one thing, then measure again.
- [ ] Measure release builds: `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`.
