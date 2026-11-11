# Huge pages

Remove the virtual memory cost from the hotpath: TLB misses, page table walks, page faults and swap.

## Symptom

- Steady stalls on random reads of a large hot state.
- Spikes on first touch: first events after start, after `Vec` growth, after a new mapping.
- A large spike after a quiet period.
- A rare stall of hundreds of µs with no other cause.

## Confirm

Method: `.claude/skills/rex-code-latency/measure.md`. Counters: `.claude/skills/rex-code-latency/measurement/counters.md`.

- [ ] Count minor faults, major faults and `dTLB-load-misses` over hotpath (`perf-event2`). Live run: 0 faults. Proof of TLB problem = dTLB miss count, not stall time. Huge pages must cut it.
- [ ] Read `/proc/meminfo`. `HugePages_Free` = reserved pages left of default size. `AnonHugePages` above 0 = THP is active.
- [ ] In `/proc/<pid>/smaps`, arena `KernelPageSize` must be `1048576 kB` or `2048 kB`.
- [ ] IF hot state fits TLB reach (entries × page size), THEN huge pages give nothing.
- [ ] IF faults and allocations are 0 and first events stay slow, THEN cause = cold caches, cold branch predictor or lazy init (`OnceLock`, `LazyLock`). Fix: at start, run synthetic events through whole hotpath, send off.

## Rules

- [ ] Make hot data contiguous first: one `Vec` per field, no `Box` chains.
- [ ] IF contiguous hot state still exceeds TLB reach, THEN use huge pages.

Host, once:

CAUTION: Test a new boot line on a host that is not in production. A wrong boot line can stop the host from booting.

- [ ] Reserve huge pages on boot line: `default_hugepagesz=1G hugepagesz=1G hugepages=16`. Keep `hugepagesz=1G`: old kernels need it. After days of uptime, RAM has no contiguous 1 GB.
- [ ] x86_64: 1 GB pages need `pdpe1gb` flag in `/proc/cpuinfo`.
- [ ] Disable THP: `transparent_hugepage=never` on boot line, or `never` in `/sys/kernel/mm/transparent_hugepage/enabled`.

Process start, before network connect. Steps 3–7 on pinned hotpath thread:

1. `nix::sys::prctl::set_thp_disable(true)` at top of `main`. Covers process and children.
2. IF free huge pages (`procfs` 0.18.0) are below arena page count, THEN fail with clear message.
3. Map arena: `MmapOptions::new().len(n).huge(Some(30)).map_anon()`. `Some(30)` = 1 GB, `Some(21)` = 2 MB, `None` = boot default size.
4. Lock: `mlockall(MlockAllFlags::MCL_CURRENT | MlockAllFlags::MCL_FUTURE)`. Covers stack, heap, code and later maps. Huge pages never swap. Needs `CAP_IPC_LOCK`, or `RLIMIT_MEMLOCK` unlimited (systemd `LimitMEMLOCK=infinity`): else `MCL_CURRENT` fails with `ENOMEM` when whole virtual size of process is above limit.
5. Pre-fault: write 1 byte per 4096 bytes, then `black_box` the block. Or add `.populate()` (`MAP_POPULATE`) to map.
6. View arena as typed slices: `bytemuck::try_cast_slice_mut`.
7. Pre-fault every other hotpath buffer: `resize(capacity, 0)`, then `clear()`. `clear()` keeps capacity and pages.

CAUTION: After `MCL_FUTURE`, an allocation fails when locked memory reaches `RLIMIT_MEMLOCK`. The process then aborts.

- [ ] Never grow `Vec` on hotpath: each new page faults.

## Numbers

Typical figures. Measure on your own hardware.

| Item | Value |
| --- | --- |
| Page table walk | up to about 400 ns when every level misses to memory. Tens of cycles when walk hits cache |
| TLB entries | first level 64 to 200. Second level 1,500 to 3,000 on current x86_64: 6 to 12 MB at 4 KB |
| Minor page fault | thousands to tens of thousands of ns |
| Major page fault (disk read) | can reach tens of ms |
| THP merge stall | hundreds of µs |
| Pre-fault of 1 GB arena | tens to hundreds of ms: kernel zeroes it |

## Rust tools

| Item | Use here | Note |
| --- | --- | --- |
| `memmap2` 0.9.12, `default-features = false` | Anonymous map | `huge(Some(bits))`: bits = log2 of page size. |
| `nix` 0.31.3, `default-features = false`, `features = ["mman", "process"]` | `mlockall`, `prctl::set_thp_disable` | |
| `bytemuck` 1.25.2, `default-features = false` | Typed view on arena bytes | `try_cast_slice_mut` tests size and alignment. `Pod` types only. |
| `mimalloc` 0.1.52 | Heap on huge pages, IF heap is hot | `MIMALLOC_RESERVE_HUGE_OS_PAGES=N` (1 GB pages). Name can change per version. |

## Traps

- [ ] `Vec::with_capacity` reserves only. `vec![0; n]` gets lazy zero pages: first write to each page still faults.
- [ ] Off Linux, memmap2 defines `MAP_HUGETLB` as 0. `huge()` compiles and gives 4 KB pages silently. Gate module with `#[cfg(target_os = "linux")]`.
- [ ] `MmapMut::lock()` locks one map. Not stack, heap or later maps.
- [ ] `mlockall(MCL_CURRENT)` wires every mapped page, also large reservations never touched.
- [ ] Pre-fault needs no `ptr::write_volatile`: stores through `&mut [u8]` that are read later are not dead. `mlockall` and `populate()` guarantee it. `black_box` is best effort.
- [ ] CAUTION: Do not cast arena bytes to a type with invariants (`bool`, `enum`, reference, `Vec`). That is undefined behavior.
- [ ] Many cores have few TLB slots for 1 GB pages. Use one huge page region per hot area.

## Code

IF you map the arena, THEN read `.claude/skills/rex-code-latency/os/huge-pages-code.md`:

- Map, lock and pre-fault 1 GB arena.
- Carve arena into struct-of-arrays slices with no `unsafe`.

## See also

- IF hotpath thread is not pinned yet, THEN read `.claude/skills/rex-code-latency/os/cpu-isolation.md` first.
- IF hot state layout is the problem, THEN read `.claude/skills/rex-code-latency/cpu/cache-line-layout.md`.
- IF hotpath allocates, THEN read `.claude/skills/rex-code-latency/memory/zero-alloc.md`.
- IF hot state fits TLB reach and stalls stay, THEN read `.claude/skills/rex-code-latency/cpu/cache-hierarchy.md`.
