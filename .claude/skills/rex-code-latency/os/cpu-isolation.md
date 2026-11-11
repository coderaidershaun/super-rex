# CPU isolation

Give the hotpath thread a core of its own: no other thread, no scheduling tick, no device interrupt, no sibling thread.

## Symptom

- p99 spikes at a fixed period (every 1–10 ms) on the hotpath thread.
- Random multi-µs spikes at any time, also on a pinned thread.
- Hotpath thread runs on the wrong core, or moves.
- Latency rises when other work on the host is busy.
- First event after a quiet period is slow.

## Confirm

Method: `.claude/skills/rex-code-latency/measure.md`.

- [ ] Run gap scan for 10 s or more on pinned core. Healthy core: no gap above threshold. Gap period names source: 1 or 4 ms = tick.
- [ ] Read `/proc/interrupts` before and after scan. Each rise in isolated CPU column = disturbance. x86_64 rows: `LOC` timer, `RES` reschedule, `CAL` function call, `TLB` shootdown, `NMI`. Check on your kernel.
- [ ] Call `nix::sched::sched_getcpu()` on hotpath thread. Result must be isolated CPU.
- [ ] Run kernel audit at startup: `isolcpus`, `nohz_full`, `nmi_watchdog`, SMT siblings.

## Rules

Host, once (root, reboot):

- [ ] Keep housekeeping cores for OS, SSH, IRQs, disk, slow path.

CAUTION: Test a new boot line on a host that is not in production. A wrong boot line can stop the host from booting.

- [ ] Put same CPU list in each parameter of `GRUB_CMDLINE_LINUX_DEFAULT`:

  ```
  isolcpus=4-7 nohz_full=4-7 rcu_nocbs=4-7 processor.max_cstate=1 intel_idle.max_cstate=0 nmi_watchdog=0
  ```

  - `isolcpus`: no general task on CPU. Tick still runs.
  - `nohz_full`: stops scheduling tick only while exactly one runnable thread is on CPU. Needs one CPU outside list.
  - `rcu_nocbs`: RCU callbacks run on housekeeping cores.
  - `processor.max_cstate=1`, `intel_idle.max_cstate=0`: no deep C-state, no wake-up latency.
  - `nmi_watchdog=0`: no lockup-detector NMI. Cost: kernel no longer detects frozen core.
- [ ] Route IRQs: write housekeeping mask to `/proc/irq/*/smp_affinity`. Stop `irqbalance`, or ban isolated CPUs in it. Boot line does not do this.
- [ ] Disable SMT in BIOS. IF firmware is locked, THEN write `off` to `/sys/devices/system/cpu/smt/control`.

Process:

- [ ] One hotpath thread per isolated core. Keep `tokio`, `rayon` and other pools off it. Second runnable thread brings tick back.
- [ ] Take isolated CPU ids from config or `/sys/devices/system/cpu/isolated`.
- [ ] Pin with `nix` (it returns the error), as first statement of spawn closure: `CpuSet::new()`, `set.set(cpu)?`, `sched_setaffinity(Pid::from_raw(0), &set)?`. Pid 0 = calling thread.
- [ ] Allocate and touch hotpath data after pin, inside closure. Linux puts page on NUMA node of CPU that first writes it.
- [ ] Treat failed pin as fatal: send `Result` to parent (`mpsc::sync_channel(1)`), return `Err` from `main`. Never fall back to housekeeping core.
- [ ] Pin slow path thread to housekeeping core.
- [ ] Gate each start with gap scan on each pinned core, before network connect. IF a gap > threshold, THEN refuse to start.
- [ ] No sampling profiler on isolated cores of live host. It floods them with NMIs.
- [ ] x86_64: another thread of the process that unmaps memory (`munmap`, `madvise`: large `free`, allocator purge) sends TLB shootdown IPI to each core of the process. Fix: slow path keeps its memory (mimalloc: `MIMALLOC_PURGE_DELAY=-1`), `kernel.numa_balancing=0`. Check on your host.
- [ ] CAUTION: A spinning `SCHED_FIFO` thread can starve the kernel threads of its CPU. That can stall other work on the host.
- [ ] IF housekeeping work must never preempt hotpath thread, THEN set `SCHED_FIFO` (`thread-priority` 3.1.1). It removes no tick.
- [ ] IF `SCHED_FIFO` thread spins, THEN read `/proc/sys/kernel/sched_rt_runtime_us`: at 950000 kernel can pause it up to 50 ms each second. `-1` removes pause. Check on your kernel.

## Numbers

Typical figures. Measure on your own hardware.

| Item | Value |
| --- | --- |
| Scheduling tick | 100 to 1000 per second (`CONFIG_HZ`) |
| NMI handler stall | microseconds |
| `pause` (`spin_loop()`), recent Intel | on the order of 100 cycles |
| Cache for each busy SMT sibling | cut in half |

## Rust tools

| Item | Use here | Note |
| --- | --- | --- |
| `nix` 0.31.3, `default-features = false`, `features = ["sched"]` | Pin, read placement | Not on macOS. |
| `core_affinity` 0.8.3 | Pin in tools and tests only | `set_for_current` returns `bool` and drops errno. |

## Traps

- [ ] macOS cannot pin. `core_affinity::set_for_current` returns `false` on Apple silicon, and `true` on Intel macOS for a hint that does not pin. Gate pin code with `#[cfg(target_os = "linux")]`.
- [ ] `core_affinity::get_core_ids()` and `available_parallelism()` see current affinity mask only. On `isolcpus` host, that is housekeeping CPUs.
- [ ] A loop that waits for another thread (CAS retry, flag, ring) calls `std::hint::spin_loop()`. Exception: a busy-poll loop on an isolated core, SMT off, that must react in fewest cycles can drop it. Check on your host.

## Code

IF you write startup checks, THEN read `.claude/skills/rex-code-latency/os/cpu-isolation-code.md`:

- Jitter detector: probe floor, quick check of a few tens of ms. Not a start gate.
- Gap scan: start gate. Logs start of each gap, so period shows.
- Kernel audit: reads isolation settings back, so engine refuses a wrongly booted host.

## See also

- IF spike remains on quiet core, THEN read `.claude/skills/rex-code-latency/os/huge-pages.md`.
- IF you need full host order, THEN read `.claude/skills/rex-code-latency/setup.md`.
