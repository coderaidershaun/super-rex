# Counters

Read time and hardware performance counters on a hotpath at low cost.

## Symptom

- `Instant::now()` around a path of a few ns costs more than the work.
- Duration reads 0, same value each time, or end before start.
- Region is slow, cause unknown: memory, branches, pipeline?
- aarch64 or M1 numbers differ from x86_64.

## Confirm

- [ ] Function of a few ns: bench it first (`criterion`, `divan`): a warm-loop average, never the tail. Then probe it in place.
- [ ] Time an empty region with your probe. IF probe cost > 1/10 of work, THEN time a batch of N calls and divide.
- [ ] Chain each output of a batch into the next input: independent calls overlap and give throughput, not latency. Vary inputs, or predictor and caches learn one.
- [ ] IF you build the timing harness, THEN read `.rex/skills/rex-code-latency/measure-code.md`.

## Rules

Time:

- [ ] Read with `quanta::Clock::raw()`. Build `Clock::new()` once at start: it calibrates.
- [ ] At start, fail IF TSC is not invariant (`raw-cpuid` `has_invariant_tsc()`): `quanta` then uses OS clock with no signal.
- [ ] Bounded probe, x86_64: `lfence(); clock.raw(); lfence();`. Leading fence: earlier work completes first. Trailing: later work waits.
- [ ] Never fence with `cpuid` or `fence(SeqCst)` (`mfence`, no order point for `rdtsc`).
- [ ] Unfenced probe (bare `clock.raw()`) only around tight vectorizable loops: fences stop vectorization. Delta is valid over the whole loop, not per iteration.
- [ ] `std::hint::black_box` input and output of work: fences do not hold register math.
- [ ] Probe `#[inline(always)]`, few live values across it: spills add stores. Pass work as `impl FnOnce() -> R`, not `dyn Fn`.
- [ ] Store raw tick pairs on hotpath, convert on slow path: `clock.delta_as_nanos(start, end)`.
- [ ] Subtract with `checked_sub`: `-` panics in debug, wraps in release. IF end < start, THEN thread moved core: pin it.
- [ ] Release: `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`. `Clock::raw()` has no `#[inline]`: needs LTO.

Hardware counters:

- [ ] Open on the measured thread, since it counts only the calling thread: `perf_event::Builder::new(event).pinned(true).enabled(true).build()?.sampled(0)?`.
- [ ] Read with `Sampler::read_user().count()`, all reads inside one `lfence` pair: user-space `rdpmc`.
- [ ] `pinned(true)`: no time-sharing. IF `count()` is `None`, THEN error.
- [ ] Several counters as one snapshot: `Group::read()`. One syscall: per batch only.
- [ ] Events: `Hardware::{CPU_CYCLES, INSTRUCTIONS, BRANCH_MISSES}`, `Cache { which: CacheId::L1D, operation: CacheOp::READ, result: CacheResult::MISS }`. TLB: `CacheId::DTLB`.
- [ ] IPC = Δ instructions / Δ cycles. IF cycles up, IPC < 1.0 AND L1D misses up, THEN eviction. Polluters: slow path, SMT sibling, other thread, DDIO.
- [ ] Faults, switches: `Software::{PAGE_FAULTS_MIN, PAGE_FAULTS_MAJ, CONTEXT_SWITCHES}`. No `read_user()` for these: `Counter::read()`, a syscall, per window.
- [ ] Default: user mode only (`exclude_kernel`, `exclude_hv`).

CAUTION: Do not run a sampling profiler (`perf record`, `samply`) on isolated production cores. Its interrupts add the jitter that you hunt.

## Numbers

Typical figures. Measure on your own hardware.

| Item | Cost |
| --- | --- |
| `Instant::now()` (vDSO) | 15 to over 50 ns. M1: about 18 ns |
| `lfence; rdtsc; lfence`, `rdpmc` | tens of cycles: measure probe cost, subtract it |
| `cpuid` | over 200 cycles. In a VM: a VM exit, thousands |
| `isb; mrs cntvct_el0; isb`, M1 | about 19.7 ns |
| `CNTVCT_EL0` tick, M1 | 41.67 ns (24 MHz) |
| Peak IPC, server core | 4 to 6 |

## Rust tools

- `quanta` 0.13.0, `default-features = false`: counter read.
- `perf-event2` 0.7.4, `default-features = false`, Linux: import as `perf_event`. `read_user()` on x86 and x86_64 only.
- `raw-cpuid` 11.6.0: invariant TSC check.
- Pin: as `.rex/skills/rex-code-latency/os/cpu-isolation.md` says.
- `criterion` 0.8.2, `divan` 0.1.21 (`DIVAN_TIMER=tsc`, `AllocProfiler`), `default-features = false`, each `[[bench]]` with `harness = false`. Use `std::hint::black_box`: `criterion::black_box` is deprecated.

## Traps

- `_rdtsc`, `__rdtscp` are `unsafe fn`. `__rdtscp` raises SIGILL without RDTSCP, and does not stop later instructions: add trailing `lfence`.
- `lfence` does not drain stores. IF region ends in stores that must be visible, THEN `mfence; lfence` before end read. AMD: `lfence` orders `rdtsc` only IF it is dispatch-serializing: Linux sets that, a hypervisor can leave it off. Check on your host.
- TSC ticks at a fixed rate, not core clock: use `CPU_CYCLES` for cycles.
- `_mm_lfence` has `#[target_feature(enable = "sse2")]`: a call from a plain fn is error E0133, sse2 baseline or not. Wrap it in one `#[inline(always)]` helper with one `unsafe` block.
- `rdpmc` has no intrinsic: let `perf-event2` run it.
- `EACCES` from `perf_event_open`: `kernel.perf_event_paranoid` > 2 and no `CAP_PERFMON`.
- User `rdpmc`: `/sys/bus/event_source/devices/cpu/rdpmc` 1 (default) = processes with a mapped event, 2 = all, 0 = off.
- aarch64: no TSC. `CNTVCT_EL0` ticks at `CNTFRQ_EL0`: 1 GHz from Armv8.6-A, 24 MHz on M1. There a sub-10 ns path reads 0 or 1 tick: time a batch, or count `CPU_CYCLES`. `CNTVCTSS_EL0` (Armv8.6-A) needs no `isb`.
- macOS: no `perf_event_open`, no pinning: development numbers only.

## Code

In `.rex/skills/rex-code-latency/measurement/counters-code.md`: TSC probe (x86_64), ordered `CNTVCT_EL0` read (aarch64), counter snapshot for IPC and L1D misses.

## See also

- IF you keep samples or report percentiles, THEN read `.rex/skills/rex-code-latency/measurement/logging.md`.
- IF IPC is low and L1D misses rise, THEN read `.rex/skills/rex-code-latency/cpu/cache-hierarchy.md`.
- IF an empty region spikes, THEN run listing `Gap scan` in `.rex/skills/rex-code-latency/os/cpu-isolation-code.md`.
