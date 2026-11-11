# Cache hierarchy

Cost of each cache level, access patterns the prefetcher follows, and data layout that keeps a hot loop in cache.

## Symptom

- Hot loop is slower than its instruction count predicts, and gets slower as the data set grows.
- Hotpath walks a linked list, a `BTreeMap`, or one `Box`, `Rc` or `Arc` per entity.
- Scan reads one or two fields of a large struct for each element.
- Random lookup (hash map by instrument ID) stalls the core on each packet.

## Confirm

Method: `.rex/skills/rex-code-latency/measure.md`. Counters: `.rex/skills/rex-code-latency/measurement/counters.md`.

1. Count L1D misses and LLC misses per element around the loop.
2. Expected misses per element = bytes read per element / 64. Forward scan of `u32`: about 1 miss per 16 elements.
3. IF misses are near that value or below, THEN layout is not the cause. Stop.
4. IF misses are far above it AND IPC is below 1.0, THEN the loop waits for memory. Apply Rules.
5. Compare two layouts on a data block larger than L3, with `std::hint::black_box` on input and result.

## Rules

Access pattern:

- [ ] Keep hotpath collections in one contiguous `Vec` or slice. Iterate forward. Prefetcher confirms a constant stride by the third access, then loads ahead.
- [ ] In hot loops, use iterators (`iter`, `zip`, `chunks_exact`), not `v[i]`. They go forward and remove bounds checks.
- [ ] No `LinkedList`, `Option<Box<Node>>` chain, tree walk, or `Box`/`Rc`/`Arc` per entity on the hotpath. Each hop is a dependent load that the prefetcher cannot predict.
- [ ] IF nodes must link, THEN keep them in one `Vec` or `slab::Slab` and link by `u32` index.
- [ ] Price-ordered book: levels inline in one `Vec`, indexed by `(price - base) / tick`, or kept sorted with best price at the end. Not `BTreeMap<Price, Box<Level>>`.
- [ ] IF a lookup must be random (hash by ID), THEN software prefetch. Order: compute address, issue prefetch, do independent work (checksum, field parse), read.
- [ ] IF packets arrive in a burst, THEN prefetch state for packet `i + k` while you process packet `i`. Tune `k` with a benchmark.
- [ ] Never software prefetch a sequential scan. It only costs cycles.
- [ ] IF a prefetch gives no gain, THEN time the independent work. Too late = stall anyway. Too early = line evicted before use.

Layout:

- [ ] IF a hot loop reads few fields of each entity, THEN use SoA: one private `Vec` per field, one `push` that writes all.
- [ ] IF the loop reads every field of each entity, THEN keep AoS. SoA gains nothing there.
- [ ] Object-style access over SoA: a proxy with one `&` or `&mut` per field, bounds checked once. Or `soa_derive`.
- [ ] IF the flag stream is the hot stream, THEN pack flags with `bitvec`: 512 per line, not 64.

## Numbers

Typical figures. Measure on your own hardware.

| Item | Figure |
| --- | --- |
| L1 hit | about 1 ns (3 to 4 cycles). Private to core |
| L2 hit | about 4 to 12 ns. Private to core |
| L3 hit | about 15 to 40 ns, variable. Shared by cores on one die |
| Main memory | about 100 ns. Over 300 instructions lost |
| Cache line | 64 bytes. 128 on Apple M |
| AoS 24-byte order, volume sum | 5 of 24 bytes used: about 20% of each line |
| SoA, same sum | 100% used: 4.8 times less memory traffic |

## Rust tools

- `soa_derive` 0.14.0, `default-features = false`: `#[derive(StructOfArray)]` gives `OrderVec`, `OrderRef`, `OrderRefMut`. `soa_zip!(book, [live, volume])` scans some fields. Wrap `OrderVec` in a private field: its fields are `pub`.
- `bitvec` 1.1.1: bit-packed flags.
- `slab` 0.4.12: nodes in one allocation, `usize` keys.
- `foldhash` 0.2.0: fast hasher for an ID map. Not for keys an attacker controls.
- `core::arch::x86_64::_mm_prefetch::<_MM_HINT_T0>`: software prefetch, read, high locality. `_MM_HINT_ET0` = write form.
- `divan` 0.1.21 or `criterion` 0.8.2: compare two layouts.

## Traps

- `_mm_prefetch` has `#[target_feature(enable = "sse")]`: a call needs `unsafe`, although SSE is in the x86_64 baseline. A prefetch never faults.
- `core::hint::prefetch_read` and `core::arch::aarch64::_prefetch` are nightly only. On stable, make the helper a no-op off x86_64.
- CAUTION: Do not keep the default `Drop` on a long `Option<Box<Node>>` chain. It recurses once per node and can overflow the stack. Write an iterative `Drop` that `take`s each `next`.
- A list built in one burst sits close together and looks fast in a benchmark. Production inserts and removes scatter it.
- `zip` stops at the shorter slice with no error.
- LLVM vectorizes a sequential `iter().sum()` but not a `step_by` loop. A layout benchmark then mixes a compute gap with the memory gap.

## Code

- Software prefetch before a random lookup, x86_64 with a no-op elsewhere. Read before you write a prefetch helper: `.rex/skills/rex-code-latency/cpu/cache-hierarchy-code.md`.

## See also

- IF several cores write the data, or a hot struct spills past one line, THEN read `.rex/skills/rex-code-latency/cpu/cache-line-layout.md`.
- IF random reads over a large region show high dTLB misses, THEN read `.rex/skills/rex-code-latency/os/huge-pages.md`.
- IF nodes come from `Box::new` on the hotpath, THEN read `.rex/skills/rex-code-latency/memory/zero-alloc.md`.
- IF L3 latency varies from run to run, THEN read `.rex/skills/rex-code-latency/os/cpu-isolation.md`.
