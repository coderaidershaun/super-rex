# Setup

Work done once, in order. Each step names the file with the detail. Failure modes across topics at end.

## Host

CAUTION: Test each boot line, sysctl, IRQ and NIC change on a host that is not in production. A wrong boot line can stop the host from booting, and a wrong NIC bind can cut the host off the network.

1. Firmware: disable SMT, limit C-states. IF firmware is locked, THEN write `off` to `/sys/devices/system/cpu/smt/control`. `.claude/skills/rex-code-latency/os/cpu-isolation.md`
2. x86_64 with 1 GB pages: confirm `pdpe1gb` in `/proc/cpuinfo`. `.claude/skills/rex-code-latency/os/huge-pages.md`
3. Boot line, one CPU list in each parameter, then reboot: `isolcpus=4-7 nohz_full=4-7 rcu_nocbs=4-7 processor.max_cstate=1 intel_idle.max_cstate=0 nmi_watchdog=0 default_hugepagesz=1G hugepages=16 transparent_hugepage=never`. `.claude/skills/rex-code-latency/os/cpu-isolation.md`, `.claude/skills/rex-code-latency/os/huge-pages.md`
4. IRQs: write housekeeping mask to `/proc/irq/*/smp_affinity`. Stop `irqbalance`, or ban isolated CPUs in it. NIC IRQs go to housekeeping cores on NUMA node of NIC. `.claude/skills/rex-code-latency/os/cpu-isolation.md`
5. Permissions: `CAP_IPC_LOCK` or large `RLIMIT_MEMLOCK` for `mlockall`. `CAP_SYS_NICE` or `RLIMIT_RTPRIO` for `SCHED_FIFO`. `.claude/skills/rex-code-latency/os/huge-pages.md`
6. Counters: `kernel.perf_event_paranoid` ≤ 2 or `CAP_PERFMON`. Keep `/sys/bus/event_source/devices/cpu/rdpmc` at 1 or 2. `.claude/skills/rex-code-latency/measurement/counters.md`
7. Kernel sockets: `CAP_NET_ADMIN` IF `SO_BUSY_POLL` exceeds `net.core.busy_read`. `net.core.rmem_max` caps receive buffer. Test `ethtool -C <iface> rx-usecs 0`. `.claude/skills/rex-code-latency/network/sockets.md`
8. Kernel bypass: Flow Steering rule before XDP program. Never bind management NIC to DPDK. `.claude/skills/rex-code-latency/network/kernel-bypass.md`
9. Wall clock: IF wall time must be accurate, THEN run `phc2sys` so `CLOCK_REALTIME` follows PTP card. `.claude/skills/rex-code-latency/measurement/logging.md`

Build:

1. `[profile.release]`: `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`. `.claude/skills/rex-code-latency/measurement/counters.md`
2. Pre-push check runs `cargo build`: `cargo check` skips `const` asserts in generic code. `.claude/skills/rex-code-latency/cpu/branching.md`
3. No crate turns on `crossbeam-utils/std` (`cargo tree -e features -i crossbeam-utils`). Else `Backoff::snooze()` yields to OS. `.claude/skills/rex-code-latency/threads/atomics.md`
4. Test and debug binaries: counting or `assert_no_alloc` global allocator. `.claude/skills/rex-code-latency/memory/zero-alloc.md`

## Process start

In this order, before first packet:

1. Top of `main`: `nix::sys::prctl::set_thp_disable(true)`. `.claude/skills/rex-code-latency/os/huge-pages.md`
2. Kernel audit: read isolation settings back. Compare `HugePages_Free` (`procfs`) with arena page count. IF mismatch, THEN exit. `.claude/skills/rex-code-latency/os/cpu-isolation-code.md`
3. `quanta::Clock::new()` once. IF TSC is not invariant (`raw-cpuid`), THEN exit. `.claude/skills/rex-code-latency/measurement/counters.md`
4. Spawn each hotpath thread, ring producer and ring consumer included. Pin as first statement of its closure, core id from config or `/sys/devices/system/cpu/isolated`. Send `Result` to parent on `sync_channel(1)`. IF `Err`, THEN exit. `.claude/skills/rex-code-latency/os/cpu-isolation.md`
5. On pinned thread: map huge page arena, `mlockall(MCL_CURRENT | MCL_FUTURE)`, pre-fault, carve typed slices. `.claude/skills/rex-code-latency/os/huge-pages.md`
6. Build pools, arenas, hazard slots, retired lists. Reserve every container. Fill each `Vec` once, then `clear()`. `.claude/skills/rex-code-latency/memory/zero-alloc.md`
7. Build rings. Pre-fault slots: push and pop `capacity` items once. Clone each reader handle before its thread spawns. `.claude/skills/rex-code-latency/threads/ring-buffers.md`
8. On each reader thread: create `HazardPointer`, `ReadHandle`, `Cache`. Do first epoch pin. `.claude/skills/rex-code-latency/memory/reclamation.md`
9. Open perf counters with `pinned(true)`. Measure probe cost: median of 1,000,000 empty regions. `.claude/skills/rex-code-latency/measurement/counters.md`
10. Run jitter detector on each pinned core. IF max > threshold, THEN exit. `.claude/skills/rex-code-latency/os/cpu-isolation-code.md`
11. Spawn slow path thread. It pins itself to a housekeeping core first. IF pin fails, THEN start fails. Writers, reclaimers: housekeeping cores too. `.claude/skills/rex-code-latency/measurement/logging.md`
12. Write start anchor: `SystemTime::now()` next to bounded probe. `.claude/skills/rex-code-latency/measurement/logging.md`
13. NIC: open DPDK, AF_XDP or `ef_vi`. Or set `socket2` options before `bind`. `.claude/skills/rex-code-latency/network/kernel-bypass.md`, `.claude/skills/rex-code-latency/network/sockets.md`
14. Connect. Start poll loop.

Shutdown: write end anchor, drop `Producer`, join slow path thread, report written and dropped counts. `.claude/skills/rex-code-latency/measurement/logging.md`

## Failure modes

| You see | Likely cause | Read |
| --- | --- | --- |
| Thread on housekeeping core, pin "worked" | `set_for_current` returned `false` (always on macOS), or `Err` dropped | `.claude/skills/rex-code-latency/os/cpu-isolation.md` |
| No isolated core in `get_core_ids()` | It lists current affinity mask only | `.claude/skills/rex-code-latency/os/cpu-isolation.md` |
| Slow after idle | Deep C-state, or thread slept | `.claude/skills/rex-code-latency/os/cpu-isolation.md` |
| First-touch spikes in live run | `vec![0; n]`, `with_capacity`: lazy pages | `.claude/skills/rex-code-latency/os/huge-pages.md` |
| 4 KB pages despite `huge()` | No-op off Linux, or no pages reserved | `.claude/skills/rex-code-latency/os/huge-pages.md` |
| Abort on allocation after start | `MCL_FUTURE` hit `RLIMIT_MEMLOCK` | `.claude/skills/rex-code-latency/os/huge-pages.md` |
| Stack overflow when long list drops | Recursive `Drop` of `Box` chain | `.claude/skills/rex-code-latency/cpu/cache-hierarchy.md` |
| Pool full, but `slab` allocates | `Slab::insert` grows when full | `.claude/skills/rex-code-latency/memory/zero-alloc.md` |
| Size assert passes, bytes still padded | `align(64)` pads the tail | `.claude/skills/rex-code-latency/cpu/cache-line-layout.md` |
| `cargo check` passes, `cargo build` fails | `const` assert in generic code | `.claude/skills/rex-code-latency/cpu/branching.md` |
| `AtomicCell` slower than expected | No native atomic: lock fallback | `.claude/skills/rex-code-latency/threads/ring-buffers.md` |
| Reader core writes shared memory | `ArcSwap::load` debt slot | `.claude/skills/rex-code-latency/memory/reclamation.md` |
| loom passes, bug remains | loom does not model load buffering | `.claude/skills/rex-code-latency/threads/atomics.md` |
| First `submit()` after quiet gap is slow | SQPOLL thread went to sleep | `.claude/skills/rex-code-latency/network/sockets.md` |
| `set_busy_poll` gives `EPERM` | Above `net.core.busy_read`, no `CAP_NET_ADMIN` | `.claude/skills/rex-code-latency/network/sockets.md` |
| Host lost SSH after XDP load | Default program takes whole queue | `.claude/skills/rex-code-latency/network/kernel-bypass.md` |
| TX corrupts sent packet | Buffer reused before TX completion | `.claude/skills/rex-code-latency/network/kernel-bypass.md` |
| Doorbell seen before payload | `fence(Release)` does not order MMIO | `.claude/skills/rex-code-latency/network/kernel-bypass.md` |
| `perf_event_open` gives `EACCES` | `perf_event_paranoid` > 2, no `CAP_PERFMON` | `.claude/skills/rex-code-latency/measurement/counters.md` |
| Detector threshold means nothing | No invariant TSC: `quanta` uses OS clock | `.claude/skills/rex-code-latency/measurement/counters.md` |
| Delta negative | Thread moved core | `.claude/skills/rex-code-latency/measurement/counters.md` |
| Dropped count > 0 | Ring too small: tail under-reported | `.claude/skills/rex-code-latency/measure.md` |
| Records lost at shutdown | Stop flag, not `Producer` drop | `.claude/skills/rex-code-latency/measurement/logging.md` |
| `SIGBUS` in offline reader | Mapped file truncated | `.claude/skills/rex-code-latency/measurement/logging.md` |
