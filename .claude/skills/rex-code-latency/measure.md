# Measure

Measure latency without changing it. Read before any code change.

## Rule

1. Measure. Change one thing. Measure again: same profile, host, load.
2. Judge by tail latency: p99, p99.9, max. Never by mean: it hides spikes.
3. Collect min 100 samples beyond each percentile: 10,000 for p99, 100,000 for p99.9.
4. Effect is real only IF more than 2 × noise AND same sign in 3 runs.
5. Report sample count + dropped count with each result.

## Not on measured path

- [ ] No `Instant::now()` per event: 15 to over 50 ns. More than a sub-10 ns hotpath.
- [ ] No `format!`, `{:?}`, `log`, `tracing`: about 20 divisions per large number. Evicts hotpath from L1i.
- [ ] No file, socket, `println!`, syscall, lock: thousands of ns per syscall, hundreds of µs per disk write.
- [ ] No growing `Vec::push`: allocator call + page faults.
- [ ] No sampling profiler (`perf record`, `samply`) as proof of tail: thousands of interrupts per second, each evicts L1.
- [ ] No noisy core: one tick or IRQ = thousands of cycles.
- [ ] No debug build: no inlining, no relation to release.

## Method

```
pseudocode
# 1. Start, hotpath thread. Rust: .claude/skills/rex-code-latency/os/cpu-isolation.md
pin thread to isolated core                       # IF pin fails, THEN exit
# Rust: .claude/skills/rex-code-latency/measurement/counters-code.md
clock = Clock::new()                              # calibrates, once
IF TSC not invariant, THEN exit                   # quanta falls back to OS clock
probe_cost = median of 1e6 × (probe(); probe())   # empty region
# Rust: .claude/skills/rex-code-latency/measurement/logging-code.md
(producer, consumer) = SpscRing::new(65_536)      # allocate all now
fill ring once, drain ring once                   # pre-faults every slot
dropped = 0
spawn slow path thread(consumer) on housekeeping core
push a0 = Anchor { wall_ns: SystemTime::now(), ticks: probe(), probe_cost }

# 2. Hotpath, per event. Rust: .claude/skills/rex-code-latency/measurement/counters-code.md
start = probe()                                   # lfence; rdtsc; lfence
out = work(black_box(input))
end = probe()
black_box(out)
# Rust: .claude/skills/rex-code-latency/measurement/logging-code.md
IF producer.push(Sample { id, start, end }) is Full, THEN dropped += 1   # cold fn, no wait

# 3. Slow path thread. Rust: .claude/skills/rex-code-latency/measurement/logging-code.md
DO
    abandoned = producer dropped                  # read before count
    n = consumer.slots()
    write raw bytes of n records to BufWriter
UNTIL abandoned AND n == 0
sync file

# 4. Shutdown, hotpath thread
push a1 = Anchor { wall_ns: SystemTime::now(), ticks: probe() }   # IF full, THEN wait
drop producer, join slow path thread, report dropped

# 5. Offline. Rust: .claude/skills/rex-code-latency/measurement/logging-code.md
hz = (a1.ticks - a0.ticks) * 1e9 / (a1.wall_ns - a0.wall_ns)        # u128
FOR EACH Sample s with s.end > s.start:
    record ((s.end - s.start).saturating_sub(probe_cost) * 1e9 / hz)  # u128
report p50, p99, p99.9, max, count, dropped
```

IF no archive needed, THEN slow path thread records `clock.delta_as_nanos(start, end)`.

Linux x86_64 calls:

- Pin: `core_affinity::set_for_current(CoreId { id })` (0.8.3), `id` from `/sys/devices/system/cpu/isolated`. IF `false`, THEN exit.
- Clock: `quanta::Clock::new()`, `clock.raw()` (0.13.0).
- Invariant TSC: `raw_cpuid::CpuId::new().get_advanced_power_mgmt_info()` → `has_invariant_tsc()` (11.6.0).
- Probe: `lfence(); clock.raw(); lfence()`. `lfence()` = `#[inline(always)]` fn with `unsafe { core::arch::x86_64::_mm_lfence() }`. SAFETY: sse2 is x86_64 baseline.
- Ring: `rtrb::RingBuffer::new(n)` (0.4.1). `push` → `Err(PushError::Full(_))`. Drain: `read_chunk(slots())`, `commit_all()`, `is_abandoned()`.
- Percentiles: `hdrhistogram::Histogram::<u64>::new_with_bounds(1, 10_000_000, 3)` (7.6.0), `saturating_record`, `value_at_quantile`.

## Check measurement itself

- [ ] Subtract probe cost. IF probe cost > 1/10 of work, THEN time N calls, divide by N.
- [ ] Noise = largest change of one percentile between two runs of one build.
- [ ] Prove core quiet first: listing `Jitter detector` in `.claude/skills/rex-code-latency/os/cpu-isolation-code.md`.
- [ ] IF dropped > 0, THEN tail is under-reported: drops cluster where spikes are. Enlarge ring.
- [ ] IF `end < start`, THEN thread moved core. Pin it.
- [ ] IF delta near 0, THEN work left window: `black_box` input and output.

## Other platforms

- aarch64: no TSC. Probe = `isb; mrs cntvct_el0; isb`. Rate = `CNTFRQ_EL0`: 1 GHz from Armv8.6-A, 24 MHz on Apple M1 (41.67 ns per tick). Linux counters: `Group::read()` per batch (`read_user()` is x86 only).
- macOS: no pinning (`set_for_current` returns `false`), no `perf_event_open`. Development only.

## Which counter answers which question

With no code change: `perf stat -e cycles,instructions,L1-dcache-load-misses,branch-misses ./bin`.

| Question | Measure | Read next |
| --- | --- | --- |
| Cache misses? | Cycles up, IPC below 1.0, L1D misses up = eviction. | `.claude/skills/rex-code-latency/cpu/cache-hierarchy.md` |
| Two cores on one line? | L1D misses rise only while other thread writes. | `.claude/skills/rex-code-latency/cpu/cache-line-layout.md` |
| TLB? | `dTLB-load-misses` per event. | `.claude/skills/rex-code-latency/os/huge-pages.md` |
| Branch misses? | `branch-misses` per event. | `.claude/skills/rex-code-latency/cpu/branching.md` |
| Kernel? | `perf stat -e context-switches,cpu-migrations,page-faults`. Spikes at fixed period. | `.claude/skills/rex-code-latency/os/jitter.md` |
| Allocation? | `assert_no_alloc` around hotpath in tests. | `.claude/skills/rex-code-latency/memory/zero-alloc.md` |
| Queue? | Ticks at push vs at pop. | `.claude/skills/rex-code-latency/threads/ring-buffers.md` |
