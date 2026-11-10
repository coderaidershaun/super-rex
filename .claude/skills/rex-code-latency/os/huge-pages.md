# Huge pages

Remove the virtual memory cost from the hotpath: TLB misses, page table walks, page faults and swap.

## Symptom

- Steady stalls of about 400 ns on random reads of a large hot state.
- Spikes on first touch: first events after start, after `Vec` growth, after a new mapping.
- A large spike after a quiet period.
- A rare stall of hundreds of µs with no other cause.

## Confirm

Method: `.claude/skills/rex-code-latency/measure.md`. Counters: `.claude/skills/rex-code-latency/measurement/counters.md`.

- [ ] Count minor faults, major faults and `dTLB-load-misses` over the hotpath with `perf-event2`. Live run: 0 faults. Huge pages must cut TLB misses.
- [ ] Read `/proc/meminfo`. `HugePages_Free` = reserved pages left. `AnonHugePages` above 0 = THP is active.
- [ ] Read arena entry in `/proc/<pid>/smaps`. `KernelPageSize` must be `1048576 kB` or `2048 kB`.
- [ ] IF hot state fits TLB reach (entries × page size), THEN huge pages give nothing.

## Rules

- [ ] Make hot data contiguous first: one `Vec` per field, no `Box` chains. One 4 KB page holds 512 `f64`: 1 TLB miss, then 511 hits.
- [ ] IF contiguous hot state still exceeds TLB reach, THEN use huge pages.

Host, once:

CAUTION: Test a new boot line on a host that is not in production. A wrong boot line can stop the host from booting.

- [ ] Reserve huge pages on boot line: `default_hugepagesz=1G hugepages=16`. After days of uptime RAM is fragmented, kernel finds no contiguous 1 GB, and map fails.
- [ ] x86_64: 1 GB pages need `pdpe1gb` flag in `/proc/cpuinfo`.
- [ ] Disable THP: `transparent_hugepage=never` on boot line, or `never` in `/sys/kernel/mm/transparent_hugepage/enabled`. Its merge stalls threads for hundreds of µs.

Process start, before network connect. Run steps 3–7 on pinned hotpath thread:

1. `nix::sys::prctl::set_thp_disable(true)` at top of `main`. Covers process and children.
2. IF `HugePages_Free` (`procfs` 0.18.0) is below arena page count, THEN fail with clear message.
3. Map arena: `MmapOptions::new().len(n).huge(Some(30)).map_anon()`. `Some(30)` = 1 GB, `Some(21)` = 2 MB, `None` = boot default size.
4. Lock: `mlockall(MlockAllFlags::MCL_CURRENT | MlockAllFlags::MCL_FUTURE)`. Covers stack, heap, code and later maps until exit. Needs `CAP_IPC_LOCK` or large `RLIMIT_MEMLOCK`.
5. Pre-fault: write 1 byte per 4096 bytes, then `black_box` the block. Or add `.populate()` (`MAP_POPULATE`) to map.
6. View arena as typed slices: `bytemuck::try_cast_slice_mut`.
7. Pre-fault every other hotpath buffer: `resize(capacity, 0)`, then `clear()`. `clear()` keeps capacity and pages.

CAUTION: After `MCL_FUTURE`, an allocation fails when locked memory reaches `RLIMIT_MEMLOCK`. The process then aborts.

- [ ] Never grow `Vec` on hotpath. Growth = new block + copy + one fault per new page.

## Numbers

Typical figures. Measure on your own hardware.

| Item | Value |
| --- | --- |
| Page table walk, 4 memory reads of about 100 ns | nearly 400 ns, about 500 ns with data fetch |
| TLB entries | 64 to 200 |
| TLB reach, 100 entries × 4 KB | 400 KB |
| 2 MB page, 1 GB page | 512×, 262,144× a 4 KB page |
| Minor page fault | thousands to tens of thousands of ns |
| Major page fault (disk read) | can reach tens of ms |
| THP merge stall | hundreds of µs |
| Pre-fault of 1 GB arena | a few ms |

## Rust tools

| Item | Use here | Note |
| --- | --- | --- |
| `memmap2` 0.9.12, `default-features = false` | Anonymous map | `huge(Some(bits))`: bits = log2 of page size. `Drop` unmaps with stored length. |
| `nix` 0.31.3, `default-features = false`, `features = ["mman", "process"]` | `mlockall`, `prctl::set_thp_disable` | |
| `bytemuck` 1.25.2, `default-features = false` | Typed view on arena bytes | `try_cast_slice_mut` tests size and alignment. `Pod` types only. |
| `mimalloc` 0.1.52 | Heap on huge pages, IF heap is hot | `MIMALLOC_RESERVE_HUGE_OS_PAGES=N`. Name can change per version. |

## Traps

- [ ] `Box`, `Vec`, `String` and every std collection use global allocator: `System` = libc `malloc` = 4 KB pages.
- [ ] `Vec::with_capacity` reserves only. `vec![0; n]` gets lazy zero pages: first write to each page still faults. Clippy `slow_vector_initialization` suggests `vec![0; n]`. For pre-fault, `#[expect]` that lint.
- [ ] Off Linux, memmap2 defines `MAP_HUGETLB` as 0. `huge()` compiles and gives 4 KB pages silently. Gate module with `#[cfg(target_os = "linux")]`.
- [ ] `MmapMut::lock()` locks one map. Not stack, heap or later maps.
- [ ] `mlockall(MCL_CURRENT)` wires every mapped page, also large reservations never touched.
- [ ] Rust has no `volatile` type, and pre-fault needs no `ptr::write_volatile` (`unsafe fn`). Stores through owned `&mut [u8]` read later are not dead. `black_box` is best effort. `mlockall` and `populate()` are the guarantee.
- [ ] CAUTION: Do not cast arena bytes to a type with invariants (`bool`, `enum`, reference, `Vec`). That is undefined behavior.
- [ ] Many cores have few TLB slots for 1 GB pages. Use one huge page region per hot area.

## Code

IF you map the arena, THEN read `.claude/skills/rex-code-latency/os/huge-pages-code.md`:

- Map, lock and pre-fault 1 GB arena: startup memory code.
- Carve arena into struct-of-arrays slices with no `unsafe`.

## See also

- IF hotpath thread is not pinned yet, THEN read `.claude/skills/rex-code-latency/os/cpu-isolation.md` first.
- IF hot state layout is the problem, THEN read `.claude/skills/rex-code-latency/cpu/cache-line-layout.md`.
- IF hotpath allocates, THEN read `.claude/skills/rex-code-latency/memory/zero-alloc.md`.
