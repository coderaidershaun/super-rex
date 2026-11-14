# Jitter

Where operating system and hardware jitter comes from, and how to judge latency by its tail.

## Symptom

- p99 or max is many times the median. Mean looks fine.
- Spikes of microseconds on a hotpath that takes tens of nanoseconds.
- Profiler or flame graph shows no hot spot, but the tail is bad.
- Latency is good in a benchmark and bad on the live host.

## Confirm

Method for every number: `rex-code-latency/measure.md`.

- [ ] Record every sample. Report p50, p99, p99.9 and max. Never the mean.
- [ ] Classify the spike. Then read the file for its cause (See also).

| Spike pattern | Likely cause |
| --- | --- |
| Periodic, every 1–10 ms | Scheduling tick, RCU callback, timer IRQ |
| Random, multi-µs, any time | IRQ, NMI, context switch |
| First touch, after `Vec` growth, after idle period | Page fault |
| Steady, on random reads of large state | TLB miss. Proof: dTLB miss count |
| Rare, hundreds of µs, no other cause | THP merge |

- [ ] Count events of the hotpath thread over one run. Live run must add 0 of each:
  - Context switches: `voluntary_ctxt_switches` and `nonvoluntary_ctxt_switches` in `/proc/<pid>/task/<tid>/status`, before and after.
  - Minor and major page faults: `perf-event2` software events, read with `Counter::read()`: `rex-code-latency/measurement/counters.md`.
- [ ] Do not apply a fix for a cause you did not confirm.

## Rules

- [ ] Split hotpath from slow path first. Hotpath = stimulus to reaction (packet in, order out). Slow path = logging, telemetry, disk, config, connections.
- [ ] Hotpath does not enter the kernel, allocate, lock, or do I/O:
  - No syscall: no `std::fs`, `std::net`, `thread::sleep`, `thread::yield_now` (= `sched_yield`). Poll with `std::hint::spin_loop()`.
  - No `println!`, `eprintln!`, `dbg!`. Each takes the stdout lock, formats, and can call `write`.
  - No `Mutex::lock`, `RwLock`, `Condvar`, blocking `mpsc::Receiver::recv`.
  - No `Box::new`, `String`, `format!`, `collect` into new `Vec`, `Vec::push` past capacity.
  - No `Arc` in hotpath crate. Each clone or drop is a locked read-modify-write and a possible free.
- [ ] Enforce the list in a hotpath crate of its own. `clippy.toml`:

  ```toml
  disallowed-types = ["std::sync::Mutex", "std::sync::Arc"]
  disallowed-methods = ["std::time::Instant::now"]
  disallowed-macros = ["std::println"]
  ```

  These only warn. To fail the build, add to that crate's `Cargo.toml`:

  ```toml
  [lints.clippy]
  disallowed_types = "deny"
  disallowed_methods = "deny"
  disallowed_macros = "deny"
  ```

- [ ] An `Arc` handle made at start, only dereferenced on hotpath, is fine. Lint fires on `use`, type, `Arc::new`, `Arc::clone`: put `#![expect(clippy::disallowed_types, reason = "...")]` on the module that builds it. Holders of its wrapper type need no allow. Lint misses `.clone()` on a field.
- [ ] Send events to slow path through SPSC ring buffer (`rtrb`). Push fixed-size struct. IF ring is full, THEN drop event and count drop.
- [ ] Run slow path on another physical core. Its work evicts hotpath data from L1.
- [ ] Never take live latency numbers with sampling profiler (`perf record`, `cargo flamegraph`, `samply`). Each sample = interrupt + L1 eviction. Use them offline, on copy of system.
- [ ] No `tracing` span and no `#[instrument]` on hotpath fn. Each enter and exit = call + often timestamp read.
- [ ] No `Instant::now()` on hotpath. Read counter with `quanta::Clock::raw()`. Convert and build histogram on slow path.
- [ ] IF probe costs as much as code it times, THEN result is probe cost.
- [ ] Fix design first. `opt-level = 3`, LTO and `codegen-units = 1` cannot remove context switch or page fault.

## Numbers

Typical figures. Measure on your own hardware.

| Event | Cost |
| --- | --- |
| Main memory read (L3 miss) | about 60 to 100 ns |
| Syscall | hundreds to thousands of ns |
| Context switch, overhead | 500–1000 ns |
| Cache refill of a dozen variables after switch | over 1 µs |
| Minor page fault | thousands to tens of thousands of ns |
| Major page fault (disk read) | can reach tens of ms |
| Clock read through vDSO | dozens to hundreds of instructions |

## Rust tools

| Item | Use here | Note |
| --- | --- | --- |
| `rtrb` 0.4.1, `default-features = false` | Bridge to slow path | Halves are not `Clone`: one producer, one consumer is a type fact. `push` returns `Err(PushError::Full(event))`. |
| `hdrhistogram` 7.6.0, `default-features = false` | p99, p99.9, max on slow path | `Histogram::<u64>::new_with_bounds(1, 10_000_000_000, 3)` allocates once. `saturating_record` does not. |
| `quanta` 0.13.0, `default-features = false` | Hotpath timestamp | `Clock::new()` calibrates: call once at startup. `raw()` reads TSC with no `unsafe`. |

## Traps

- [ ] `Instant::now()` on Linux = `clock_gettime(CLOCK_MONOTONIC)` through vDSO: no kernel entry, but many instructions. IF clock source has no vDSO support, THEN each call is a syscall.
- [ ] Context switch cost is mostly cache refill, not register save. Perfect hotpath code on a shared core still pays it.

## See also

- IF cause is tick, RCU, IRQ, NMI or context switch, THEN read `rex-code-latency/os/cpu-isolation.md`.
- IF cause is page fault, TLB miss or THP, THEN read `rex-code-latency/os/huge-pages.md`.
- IF you build bridge to slow path, THEN read `rex-code-latency/threads/ring-buffers.md`.
- IF hotpath logs, THEN read `rex-code-latency/measurement/logging.md`.
- IF hotpath allocates, THEN read `rex-code-latency/memory/zero-alloc.md`.
