# Counters

Read time and hardware performance counters on a hotpath for a few cycles.

## Symptom

- `Instant::now()` around a path of a few ns costs more than the work.
- Duration reads 0, same value each time, or end before start.
- Region is slow, cause unknown: memory, branches, pipeline?
- aarch64 or M1 numbers differ from x86_64.

## Confirm

- [ ] Time an empty region with your probe. IF probe cost > 1/10 of work, THEN time a batch of N calls and divide.
- [ ] Price probe on target host (`criterion`, `divan`). Method: `.claude/skills/rex-code-latency/measure.md`.

## Rules

Time:

- [ ] Read with `quanta::Clock::raw()`. Build `Clock::new()` once at start: it calibrates.
- [ ] At start, fail IF TSC is not invariant (`raw-cpuid` `has_invariant_tsc()`): `quanta` then uses OS clock with no signal.
- [ ] Bounded probe, x86_64: `lfence(); clock.raw(); lfence();`. Leading fence: earlier work retires first. Trailing: later work waits for read.
- [ ] Never fence with `cpuid` (over 200 cycles) or `fence(SeqCst)` (`mfence`, no order point for `rdtsc`).
- [ ] Unfenced probe (bare `clock.raw()`) only in tight vectorizable loops: fences stop vectorization. Delta stays valid.
- [ ] `std::hint::black_box` input and output of work: fences do not hold register math.
- [ ] Probe `#[inline(always)]`, few live values across it: spills add stores. Pass work as `impl FnOnce() -> R`, not `dyn Fn`.
- [ ] Store raw tick pairs on hotpath. Convert on slow path: `clock.delta_as_nanos(start, end)`.
- [ ] Subtract with `checked_sub`: `-` panics in debug, wraps in release. IF end < start, THEN thread moved core: pin it.
- [ ] Release: `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`. `Clock::raw()` has no `#[inline]`: needs LTO.

Hardware counters:

- [ ] Open at start: `perf_event::Builder::new(event).pinned(true).enabled(true).build()?.sampled(0)?`.
- [ ] Read with `Sampler::read_user().count()`, all reads inside one `lfence` pair: user-space `rdpmc`, no syscall.
- [ ] `pinned(true)`: no time-sharing. IF `count()` is `None`, THEN error.
- [ ] Several counters as one snapshot: `Group::read()`. One syscall: per batch only.
- [ ] Events: `Hardware::{CPU_CYCLES, INSTRUCTIONS, BRANCH_MISSES}`, `Cache { which: CacheId::L1D, operation: CacheOp::READ, result: CacheResult::MISS }`. TLB: `CacheId::DTLB`.
- [ ] IPC = Δ instructions / Δ cycles. IF cycles up, IPC < 1.0 AND L1D misses up, THEN eviction. Polluter: slow path work, SMT sibling, other thread, DDIO burst.
- [ ] Faults, switches: `Software::{PAGE_FAULTS_MIN, PAGE_FAULTS_MAJ, CONTEXT_SWITCHES}`. No `read_user()` for these: `Counter::read()`, a syscall, per window.
- [ ] Default counts user mode only (`exclude_kernel`, `exclude_hv`).

CAUTION: Do not run a sampling profiler (`perf record`, `samply`) on isolated production cores. Its interrupts add the jitter that you hunt.

Benchmarks:

- [ ] `criterion`: saved baseline. `divan`: `DIVAN_TIMER=tsc` timer, `AllocProfiler`. Each `[[bench]]`: `harness = false`.
- [ ] Bench = warm-loop average: price probes, catch regressions. Never tail.

## Numbers

Typical figures. Measure on your own hardware.

| Item | Cost |
| --- | --- |
| `Instant::now()` (vDSO) | 15 to over 50 ns. M1: about 18 ns |
| `rdtsc` | about 2 to 4 cycles |
| `lfence` | about 10 to 15 cycles |
| `cpuid` | over 200 cycles |
| `rdpmc` | a few cycles |
| `Clock::raw()` unfenced, M1 | about 0.32 ns (reads overlap) |
| `isb; mrs cntvct_el0; isb`, M1 | about 19.7 ns |
| `CNTVCT_EL0` tick, M1 | 41.67 ns (24 MHz) |
| Peak IPC, server core | 4 to 6 |
| Example, healthy / evicted | 100 cycles at IPC 2.5 / 300 at IPC 0.5 |

## Rust tools

- `quanta` 0.13.0, `default-features = false`: counter read.
- `perf-event2` 0.7.4, `default-features = false`, Linux: import as `perf_event`. `read_user()` on x86 and x86_64 only.
- `raw-cpuid` 11.6.0: invariant TSC check.
- `core_affinity` 0.8.3: pin. Returns `bool`, drops errno.
- `minstant` 0.1.7: TSC `Instant` on Linux x86_64. Elsewhere `SystemTime`: jumps.
- `criterion` 0.8.2, `divan` 0.1.21, `default-features = false`. Use `std::hint::black_box`: `criterion::black_box` is deprecated.

## Traps

- `_rdtsc`, `__rdtscp` are `unsafe fn`. `__rdtscp` raises SIGILL without RDTSCP, and does not stop later instructions: add trailing `lfence`.
- `_mm_lfence` = safe fn with `#[target_feature(enable = "sse2")]`. From a plain fn: error E0133, sse2 baseline or not. `#[target_feature]` cannot go with `#[inline(always)]`. So: one `unsafe` block in one `#[inline(always)]` helper.
- `rdpmc`: no intrinsic, `asm!` is always `unsafe`. Let `perf-event2` hold it.
- `EACCES` from `perf_event_open`: `kernel.perf_event_paranoid` > 2 and no `CAP_PERFMON`.
- User `rdpmc` needs CR4.PCE. `/sys/bus/event_source/devices/cpu/rdpmc`: 1 (default) = only processes with a mapped event. 2 = all. 0 = off.
- Recent Intel maps L1D read miss to `L1D.REPLACEMENT`.
- aarch64: no TSC. `CNTVCT_EL0` ticks at `CNTFRQ_EL0`: 1 GHz from Armv8.6-A, 24 MHz on M1. There a sub-10 ns path reads 0 or 1 tick: time a batch, or count `CPU_CYCLES` per batch. Order with `isb` in `asm!` (`__isb` is unstable). `CNTVCTSS_EL0` (Armv8.6-A) needs no `isb`.
- macOS: no `perf_event_open`, no pinning. Development numbers only.

## Code

In `.claude/skills/rex-code-latency/measurement/counters-code.md`:

- Bounded and unfenced TSC probe: IF you time a region on x86_64.
- Ordered `CNTVCT_EL0` read: IF you time a region on aarch64.
- Counter snapshot, IPC + L1D misses: IF you add counters to code.

## See also

- IF you keep samples or report percentiles, THEN read `.claude/skills/rex-code-latency/measurement/logging.md`.
- IF IPC is low and L1D misses rise, THEN read `.claude/skills/rex-code-latency/cpu/cache-hierarchy.md`.
- IF an empty region spikes, THEN run listing `Jitter detector` in `.claude/skills/rex-code-latency/os/cpu-isolation-code.md`.
