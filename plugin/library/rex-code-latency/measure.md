# Measure

Measure latency without changing it. Read before any code change.

## Rule

1. Measure. Change one thing. Measure again: same profile, host, load.
2. Judge by tail latency: p99, p99.9, max. Never by mean: it hides spikes.
3. Collect min 100 samples beyond each percentile: 10,000 for p99, 100,000 for p99.9.
4. Effect is real only IF more than 2 × noise AND same sign in 3 runs. Noise = largest change of one percentile between two runs of one build.
5. Report with each result: sample count, dropped count, backward count (end before start). Report first events apart.

## Which counter answers which question

With no code change: `perf stat -e cycles,instructions,L1-dcache-load-misses,dTLB-load-misses,branch-misses ./bin`. It counts every thread. For hotpath thread only, attach to it: `-t <tid>` in place of `./bin`.

| Question | Measure | Read next |
| --- | --- | --- |
| Cache misses? | Cycles up, IPC below 1.0, L1D misses up = eviction. | `rex-code-latency/cpu/cache-hierarchy.md` |
| Two cores on one line? | L1D misses rise only while other thread writes. Proof: `perf c2c`. | `rex-code-latency/cpu/cache-line-layout.md` |
| TLB? | `dTLB-load-misses` per event. | `rex-code-latency/os/huge-pages.md` |
| Branch misses? | `branch-misses` per event. Near 0 = not the cause. | `rex-code-latency/cpu/branching.md` |
| Kernel? | `perf stat -e context-switches,cpu-migrations,page-faults`. Then spike pattern. | `rex-code-latency/os/jitter.md` |
| Allocation? | Heap calls per event: counting allocator in a test. | `rex-code-latency/memory/zero-alloc.md` |
| Lock or blocking channel? | `strace -f -e trace=futex` shows hotpath thread parks. | `rex-code-latency/threads/ring-buffers.md` |
| Queue? | Ticks at push vs at pop. | `rex-code-latency/threads/ring-buffers.md` |
| Network receive? | Kernel stamp gap: kernel receive stamp to handler entry. | `rex-code-latency/network/sockets.md` |

## Not on measured path

- [ ] No `Instant::now()` per event: 15 to over 50 ns. More than a sub-10 ns hotpath.
- [ ] No `format!`, `{:?}`, `log`, `tracing`: about 20 divisions per large number. Evicts hotpath from L1i.
- [ ] No file, socket, `println!`, syscall, lock: thousands of ns per syscall, hundreds of µs per disk write.
- [ ] No growing `Vec::push`: allocator call + page faults.
- [ ] No sampling profiler (`perf record`, `samply`) as proof of tail: thousands of interrupts per second, each evicts L1.
- [ ] No noisy core: one tick or IRQ = thousands of cycles.
- [ ] No debug build: no inlining, no relation to release.

## Check measurement itself

- [ ] Subtract probe cost. IF probe cost > 1/10 of work, THEN time N calls, divide by N.
- [ ] Prove core quiet first: listing `Gap scan` in `rex-code-latency/os/cpu-isolation-code.md`.
- [ ] IF dropped > 0, THEN tail is under-reported: drops cluster where spikes are. Enlarge ring.
- [ ] IF backward count > 0, THEN thread moved core: pin failed. Fix pin.
- [ ] IF delta near 0, THEN work left window: `black_box` input, and output before end probe.
- [ ] IF first events are slow and later fine, THEN cause = warm-up: cold caches, first-touch faults, lazy init. Read `rex-code-latency/os/huge-pages.md`.

## Benchmark faster, production p99 same

- [ ] `criterion` and `divan` time a warm loop: hot caches, trained predictor, no interrupt. They report a typical value, not the tail.
- [ ] Tail samples come from events a benchmark lacks: cold cache, interrupt, page fault, queue behind a burst.
- [ ] Stamp each stage of each event in production. Take samples above p99. Find stage whose time grows in them.
- [ ] IF changed stage does not grow in tail samples, THEN no speed-up of it moves p99. Fix stage that grows.

## Code

IF you build the timing harness, THEN read `rex-code-latency/measure-code.md`: method pseudocode, exact Linux calls, other platforms.
