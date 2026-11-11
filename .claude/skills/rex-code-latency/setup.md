# Setup

Work done once, in order. Each step names the file with the detail.

## Host

CAUTION: Test each boot line, sysctl, IRQ and NIC change on a host that is not in production. A wrong boot line can stop the host from booting, and a wrong NIC bind can cut the host off the network.

1. Firmware: disable SMT, limit C-states, pick the maximum performance power profile. IF firmware is locked, THEN write `off` to `/sys/devices/system/cpu/smt/control`. `.claude/skills/rex-code-latency/os/cpu-isolation.md`
2. x86_64 with 1 GB pages: confirm `pdpe1gb` in `/proc/cpuinfo`. `.claude/skills/rex-code-latency/os/huge-pages.md`
3. Boot line, one CPU list in each parameter, then reboot: `isolcpus=4-7 nohz_full=4-7 rcu_nocbs=4-7 processor.max_cstate=1 intel_idle.max_cstate=0 nmi_watchdog=0 default_hugepagesz=1G hugepagesz=1G hugepages=16 transparent_hugepage=never`. IF DPDK uses `vfio-pci`, THEN add `intel_iommu=on iommu=pt`. Check on your host. `.claude/skills/rex-code-latency/os/cpu-isolation.md`, `.claude/skills/rex-code-latency/os/huge-pages.md`
4. Clock source: `/sys/devices/system/clocksource/clocksource0/current_clocksource` must be `tsc` on bare metal. IF `hpet` or `acpi_pm`, THEN each `Instant::now()` can be a syscall: `dmesg` says why kernel left TSC. Check on your host. `.claude/skills/rex-code-latency/os/jitter.md`
5. CPU frequency: governor `performance` in `/sys/devices/system/cpu/cpu*/cpufreq/scaling_governor`. Fix one turbo policy: `/sys/devices/system/cpu/intel_pstate/no_turbo` = 1 (AMD: `cpufreq/boost` = 0) gives a steady clock. Turbo: faster, but clock moves with heat and other cores' load. Measure both. Check on your host.
6. IRQs: write housekeeping mask to `/proc/irq/*/smp_affinity` and `/proc/irq/default_smp_affinity`. Stop `irqbalance`, or ban isolated CPUs in it. NIC IRQs go to housekeeping cores on NUMA node of NIC. Kernel work queues: housekeeping mask in `/sys/devices/virtual/workqueue/cpumask`. Check on your host. `.claude/skills/rex-code-latency/os/cpu-isolation.md`
7. Sysctls: `kernel.numa_balancing=0`. IF a `SCHED_FIFO` thread spins, THEN `kernel.sched_rt_runtime_us=-1`. Swap off: `swapoff -a`, and remove swap from `/etc/fstab`. `mlockall` protects only its own process. Check on your host. `.claude/skills/rex-code-latency/os/cpu-isolation.md`
8. Permissions: `CAP_IPC_LOCK` or `RLIMIT_MEMLOCK` unlimited for `mlockall`. `CAP_SYS_NICE` or `RLIMIT_RTPRIO` for `SCHED_FIFO`. `.claude/skills/rex-code-latency/os/huge-pages.md`
9. Counters: `kernel.perf_event_paranoid` ≤ 2 or `CAP_PERFMON`. Keep `/sys/bus/event_source/devices/cpu/rdpmc` at 1 or 2. `.claude/skills/rex-code-latency/measurement/counters.md`
10. Kernel sockets: `CAP_NET_ADMIN` IF `SO_BUSY_POLL` exceeds `net.core.busy_read`. `net.core.rmem_max` caps receive buffer. Test `ethtool -C <iface> rx-usecs 0`. `.claude/skills/rex-code-latency/network/sockets.md`
11. Kernel bypass: Flow Steering rule before XDP program. Never bind management NIC to DPDK. DPDK `vfio-pci`: bind NIC with `dpdk-devbind.py --bind=vfio-pci <PCI address>`. Check on your host. `.claude/skills/rex-code-latency/network/kernel-bypass.md`
12. Wall clock: IF wall time must be accurate, THEN run `ptp4l` on the NIC and `phc2sys` so `CLOCK_REALTIME` follows PTP card. `.claude/skills/rex-code-latency/measurement/logging.md`
13. Baseline, before the application runs: measure noise of each isolated CPU with `rtla osnoise top -c 4-7`, or with the gap scan. Keep the result: no application on this host gets below it. Check on your host.

Build:

1. `[profile.release]`: `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`. `.claude/skills/rex-code-latency/measurement/counters.md`
2. Hotpath crate: `clippy.toml` bans plus `[lints.clippy]` at `deny`. `.claude/skills/rex-code-latency/os/jitter.md`
3. Pre-push check runs `cargo build`: `cargo check` skips `const` asserts in generic code. `.claude/skills/rex-code-latency/cpu/branching.md`
4. Test and debug binaries: counting or `assert_no_alloc` global allocator. `.claude/skills/rex-code-latency/memory/zero-alloc.md`

## Process start

In this order, before first packet:

1. Top of `main`: `nix::sys::prctl::set_thp_disable(true)`. `.claude/skills/rex-code-latency/os/huge-pages.md`
2. IF DPDK: call `rte_eal_init` now, with `--main-lcore` on a housekeeping core. It pins its caller and each lcore thread, and maps its own huge pages. Check on your host. `.claude/skills/rex-code-latency/network/kernel-bypass.md`
3. Kernel audit: read isolation settings back. Compare `HugePages_Free` (`procfs`) with arena page count. IF mismatch, THEN exit. `.claude/skills/rex-code-latency/os/cpu-isolation-code.md`
4. `quanta::Clock::new()` once. IF TSC is not invariant (`raw-cpuid`), THEN exit. `.claude/skills/rex-code-latency/measurement/counters.md`
5. Spawn and pin each hotpath thread: pin is first statement of its closure, core id from config or `/sys/devices/system/cpu/isolated`. Send `Result` to parent on `sync_channel(1)`. IF `Err`, THEN exit. `.claude/skills/rex-code-latency/os/cpu-isolation.md`
6. On each pinned thread: map huge page arena, `mlockall(MCL_CURRENT | MCL_FUTURE)`, pre-fault, carve typed slices. `.claude/skills/rex-code-latency/os/huge-pages.md`
7. On each pinned thread: build pools, arenas, hazard pointers, retired lists. Reserve every container. Fill each `Vec` once, then `clear()`. `.claude/skills/rex-code-latency/memory/zero-alloc.md`
8. On each producer's pinned thread: build its ring, then push and pop `capacity` items once. Slots then sit on its NUMA node. Send `Consumer` and each reader handle to its thread over a channel. `.claude/skills/rex-code-latency/threads/ring-buffers.md`
9. Spawn slow path thread with its `Consumer`. It pins itself to a housekeeping core first. IF pin fails, THEN start fails. Writers, reclaimers: housekeeping cores too. `.claude/skills/rex-code-latency/measurement/logging.md`
10. On each reader thread: create `HazardPointer`, `ReadHandle`, `Cache`. Do first epoch pin. `.claude/skills/rex-code-latency/memory/reclamation.md`
11. On each thread you measure: open perf counters there, with `pinned(true)`. Measure probe cost: median of 1,000,000 empty regions. `.claude/skills/rex-code-latency/measurement/counters.md`
12. Run gap scan for 10 s or more on each pinned core. IF a gap > threshold, THEN exit. `.claude/skills/rex-code-latency/os/cpu-isolation-code.md`
13. Write start anchor: `SystemTime::now()` next to bounded probe. `.claude/skills/rex-code-latency/measurement/logging.md`
14. NIC: AF_XDP or `ef_vi`, or `socket2` options before `bind`. `.claude/skills/rex-code-latency/network/kernel-bypass.md`, `.claude/skills/rex-code-latency/network/sockets.md`
15. Connect. Start poll loop.

Shutdown: write end anchor, drop `Producer`, join slow path thread, report written and dropped counts. `.claude/skills/rex-code-latency/measurement/logging.md`

## Start-up failures

| You see | Likely cause | Read |
| --- | --- | --- |
| Thread on housekeeping core, pin "worked" | `core_affinity` returned `false`, or `Err` dropped | `.claude/skills/rex-code-latency/os/cpu-isolation.md` |
| No isolated core in `get_core_ids()` | It lists current affinity mask only | `.claude/skills/rex-code-latency/os/cpu-isolation.md` |
| 4 KB pages despite `huge()` | No-op off Linux, or no pages reserved | `.claude/skills/rex-code-latency/os/huge-pages.md` |
| Abort on allocation after start | `MCL_FUTURE` hit `RLIMIT_MEMLOCK` | `.claude/skills/rex-code-latency/os/huge-pages.md` |
| `set_busy_poll` gives `EPERM` | Above `net.core.busy_read`, no `CAP_NET_ADMIN` | `.claude/skills/rex-code-latency/network/sockets.md` |
| Host lost SSH after XDP load | Default program takes whole queue | `.claude/skills/rex-code-latency/network/kernel-bypass.md` |
| `perf_event_open` gives `EACCES` | `perf_event_paranoid` > 2, no `CAP_PERFMON` | `.claude/skills/rex-code-latency/measurement/counters.md` |
| Counters show another thread's work | Opened on another thread | `.claude/skills/rex-code-latency/measurement/counters.md` |
