# Measure: code

Read when you build the timing harness that `rex-code-latency/measure.md` asks for: a harness that times each event and does not change its latency.

## Where the Rust is

The method below is pseudocode. Each step has real Rust in another file:

| Step | Rust |
| --- | --- |
| 1. Pin | Pin as `rex-code-latency/os/cpu-isolation.md` says: `nix` `sched_setaffinity` |
| 1. Clock, probe, probe cost | Listing `Bounded and unfenced TSC probe, x86_64` in `rex-code-latency/measurement/counters-code.md` |
| 1. Quiet core | Listing `Gap scan` in `rex-code-latency/os/cpu-isolation-code.md` |
| 1 to 4. Ring, slow path, anchors | Listing `Event block and hand-off to a pinned slow path` in `rex-code-latency/measurement/logging-code.md` |
| 5. Percentiles | Listing `Percentiles from tick pairs` in the same file |

## Method

```
pseudocode
# 1. Start, on hotpath thread, before first event
pin this thread to its isolated core               # first statement. IF Err, THEN start fails
clock = Clock::new()                               # calibrates: once
IF x86_64 AND TSC not invariant, THEN start fails  # else quanta reads OS clock
(producer, consumer) = RingBuffer::new(65_536)     # allocate all now
push and pop capacity items once                   # pre-faults every slot
spawn slow path thread(consumer)                   # it pins itself to housekeeping core first
probe_cost = median of 1e6 × (probe(); probe())    # empty region, in ticks
scan_for_noise(clock, 10 s, threshold)             # IF Err, THEN start fails: core not quiet
dropped = 0
push Anchor { wall_ns: SystemTime::now(), ticks: probe(), probe_cost }

# 2. Hotpath, per event
start = probe()                                    # x86_64: lfence; rdtsc; lfence
out = black_box(work(black_box(input)))            # before end probe: keeps work inside window
end = probe()
IF producer.push(Sample { start, end }) is Full, THEN dropped += 1   # cold fn, never wait

# 3. Slow path thread
DO
    abandoned = consumer.is_abandoned()            # read before slots()
    n = consumer.slots()
    IF n > 0, THEN write raw bytes of n records to BufWriter, commit n
    ELSE spin_loop()                               # sleep some µs IF core is shared
UNTIL abandoned AND n == 0
flush BufWriter, sync file

# 4. Shutdown, hotpath thread
push Anchor { wall_ns: SystemTime::now(), ticks: probe() }   # IF Full, THEN retry: hotpath is done
drop producer, join slow path thread, report written and dropped

# 5. Offline
hz = (a1.ticks - a0.ticks) * 1e9 / (a1.wall_ns - a0.wall_ns)         # u128
h = Histogram::new_with_bounds(1, 10_000_000_000, 3)                 # 1 ns to 10 s
FOR EACH Sample s, in archive order:
    IF s.end < s.start, THEN backward += 1, skip s                   # > 0 = pin failed
    ns = (s.end - s.start).saturating_sub(probe_cost) * 1e9 / hz     # u128
    IF s is in first WARM_UP samples, THEN warm.record(ns), ELSE h.record(ns)
report p50, p99, p99.9, max, count, dropped, backward. Report warm apart.
```

- `WARM_UP`: pick it from a run. Use the count of samples before latency settles.
- IF no archive is needed, THEN slow path thread does step 5 itself. Convert with `clock.delta_as_nanos(start, end)`, probe cost too.
- IF dropped > 0, THEN tail is under-reported. Enlarge ring before you trust p99.

## Linux x86_64 calls

- Pin: `nix` `sched_setaffinity`, as `rex-code-latency/os/cpu-isolation.md` says. Core id from config or `/sys/devices/system/cpu/isolated`. IF it returns `Err`, THEN start fails.
- Clock: `quanta::Clock::new()`, `clock.raw()` (0.13.0). `raw()` has no `#[inline]`: build with LTO.
- Invariant TSC: `raw_cpuid::CpuId::new().get_advanced_power_mgmt_info()` → `has_invariant_tsc()` (11.6.0). `None` = not invariant.
- Quiet core: `scan_for_noise(&clock, Duration::from_secs(10), threshold_nanos)`.
- Probe: `lfence(); clock.raw(); lfence()`. `lfence()` = `#[inline(always)]` fn with `unsafe { core::arch::x86_64::_mm_lfence() }`. SAFETY: sse2 is x86_64 baseline.
- Ring: `rtrb::RingBuffer::new(n)` (0.4.1). `push` → `Err(PushError::Full(_))`. Drain: `read_chunk(slots())`, `commit_all()`, `is_abandoned()`.
- Percentiles: `hdrhistogram::Histogram::<u64>::new_with_bounds(1, 10_000_000_000, 3)` (7.6.0): 1 ns to 10 s, so a major page fault still records. `saturating_record`, `value_at_quantile`.
- Hardware counters, IF needed: open them on the measured thread, in step 1. Listing `Counter snapshot for IPC and L1D misses, Linux x86_64` in `rex-code-latency/measurement/counters-code.md`.

## Other platforms

- aarch64: no TSC. Probe = `isb; mrs cntvct_el0; isb`: listing "Ordered `CNTVCT_EL0` read, aarch64" in `rex-code-latency/measurement/counters-code.md`. Rate = `CNTFRQ_EL0`: 1 GHz from Armv8.6-A, 24 MHz on Apple M1 (41.67 ns per tick). IF region lasts few ticks, THEN time a batch. Linux counters: `Group::read()` per batch (`read_user()` is x86 only).
- macOS: no pinning, no `perf_event_open`: `rex-code-latency/os/cpu-isolation.md`. Development numbers only.
