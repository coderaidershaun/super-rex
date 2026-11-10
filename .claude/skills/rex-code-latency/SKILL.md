---
name: rex-code-latency
description: >
  Reference for latency in Rust: measure first, then cause, proof, fix and
  numbers for OS jitter, CPU caches, allocation, atomics, queues, network
  receive and timing.
when_to_use: >
  Use when Rust code has a latency problem: "latency", "p99", "tail latency",
  "jitter", "spikes", "hot path", "low latency", "lock-free", "false sharing",
  "kernel bypass", "busy poll", "time this function".
disable-model-invocation: false
user-invocable: true
---

# Code latency

Find and fix one latency problem in Rust code. This file routes. Topic files hold rules.

## Use

1. Measure first: `.claude/skills/rex-code-latency/measure.md`. Get p99, p99.9, max, and counter that names cause.
2. Find Router row that matches what you see. Read its first file.
3. Read its second file only IF first file sends you there or does not answer. Max 2 topic files per problem.
4. Apply fix only for cause that `## Confirm` of topic file proved.
5. Open a `-code.md` file only when you write that code. `## Code` of topic file names it.

Never read whole skill.

## Router

| You see or ask | Read first | Then |
| --- | --- | --- |
| p99 or max many times median, mean fine. Fast in benchmark, slow on live host. | `.claude/skills/rex-code-latency/os/jitter.md` | `.claude/skills/rex-code-latency/os/cpu-isolation.md` |
| Spikes at fixed period (1–10 ms), random multi-µs, or after quiet period, also on pinned core. How to pin, isolate core. | `.claude/skills/rex-code-latency/os/cpu-isolation.md` | `.claude/skills/rex-code-latency/setup.md` |
| First events after start slow, then fine. Spike when `Vec` grows or buffer maps. | `.claude/skills/rex-code-latency/os/huge-pages.md` | `.claude/skills/rex-code-latency/memory/zero-alloc.md` |
| Steady stalls of about 400 ns on random reads of large table. Rare stall of hundreds of µs. | `.claude/skills/rex-code-latency/os/huge-pages.md` | `.claude/skills/rex-code-latency/cpu/cache-hierarchy.md` |
| Spikes in `Box::new`, `Vec::push`, `format!`, worse with more threads. Does hotpath allocate? Arena, pool. | `.claude/skills/rex-code-latency/memory/zero-alloc.md` | `.claude/skills/rex-code-latency/os/huge-pages.md` |
| Hot loop slower as data grows, IPC below 1.0. Linked list, `BTreeMap`, `Box` per entity, random hash lookup. | `.claude/skills/rex-code-latency/cpu/cache-hierarchy.md` | `.claude/skills/rex-code-latency/os/huge-pages.md` |
| Two threads each write own counter: throughput drops. Hot struct over 64 bytes. `CachePadded` or `align(64)`? | `.claude/skills/rex-code-latency/cpu/cache-line-layout.md` | `.claude/skills/rex-code-latency/threads/atomics.md` |
| Fast on test data, slow on real data, high `branch-misses`. `dyn Trait`, bounds checks, cold error path. | `.claude/skills/rex-code-latency/cpu/branching.md` | `.claude/skills/rex-code-latency/cpu/cache-hierarchy.md` |
| Messages through `Mutex<VecDeque>`, `mpsc` or blocking channel. Which queue crate? | `.claude/skills/rex-code-latency/threads/ring-buffers.md` | `.claude/skills/rex-code-latency/threads/atomics.md` |
| Reader sees struct half old, half new. Many readers of state one thread writes, `RwLock` slow. | `.claude/skills/rex-code-latency/threads/ring-buffers.md` | `.claude/skills/rex-code-latency/threads/atomics.md` |
| Lock-free code fails on aarch64 only. Which `Ordering`? `lock` in assembly. CAS loop slows with more cores. | `.claude/skills/rex-code-latency/threads/atomics.md` | `.claude/skills/rex-code-latency/cpu/cache-line-layout.md` |
| Lock-free stack corrupt after CAS succeeds. Reader reads freed node. Memory grows while reader stalls. | `.claude/skills/rex-code-latency/memory/reclamation.md` | `.claude/skills/rex-code-latency/threads/atomics.md` |
| Config read per packet, rarely changed. `Arc::clone` or `ArcSwap::load` cost rises with reader cores. | `.claude/skills/rex-code-latency/memory/reclamation.md` | `.claude/skills/rex-code-latency/threads/ring-buffers.md` |
| Receive takes µs through `tokio`, `mio`, `epoll` or `std::net`. Busy poll? `io_uring`? | `.claude/skills/rex-code-latency/network/sockets.md` | `.claude/skills/rex-code-latency/os/cpu-isolation.md` |
| Socket tuning done, still over budget. DPDK, AF_XDP, `ef_vi`, OpenOnload, NIC doorbell. | `.claude/skills/rex-code-latency/network/kernel-bypass.md` | `.claude/skills/rex-code-latency/network/sockets.md` |
| Cause unknown: cache, branch, kernel or allocation? | `.claude/skills/rex-code-latency/measure.md` | `.claude/skills/rex-code-latency/measurement/counters.md` |
| Time a few-ns function: `Instant::now()` costs more, reads 0, or too coarse on aarch64. | `.claude/skills/rex-code-latency/measurement/counters.md` | `.claude/skills/rex-code-latency/measure.md` |
| p99 rises with logging on. `log` or `tracing` on hotpath. Keep every sample. | `.claude/skills/rex-code-latency/measurement/logging.md` | `.claude/skills/rex-code-latency/threads/ring-buffers.md` |
| Is my change faster? Benchmark faster, production p99 same. | `.claude/skills/rex-code-latency/measure.md` | `.claude/skills/rex-code-latency/measurement/logging.md` |
| New host or process: boot line, IRQs, huge pages, start order. No row fits. | `.claude/skills/rex-code-latency/setup.md` | `.claude/skills/rex-code-latency/os/cpu-isolation.md` |

## Shared files

| File | Read when |
| --- | --- |
| `.claude/skills/rex-code-latency/measure.md` | First, always. Before and after each change. |
| `.claude/skills/rex-code-latency/setup.md` | New host, start order, or failure with no Router row. |
| `.claude/skills/rex-code-latency/crates.md` | Before you add a crate: version, features. |
| `.claude/skills/rex-code-latency/glossary.md` | A term is not clear. |

## Rules for every latency job

- [ ] Judge by tail latency: p99, p99.9, max. Never mean: it hides spikes.
- [ ] Hotpath: no syscall, lock, allocation, I/O, `format!`, `log` or `tracing`.
- [ ] Hotpath never waits for slow path. Bridge = SPSC ring buffer (`rtrb`). Ring full: drop event, count drop.
- [ ] Pay every cost at start: pin thread, then allocate, pre-fault, build rings, prove core quiet.
- [ ] Change one thing, then measure again. No fix for a cause you did not confirm.
- [ ] Measure release builds only: `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`.
