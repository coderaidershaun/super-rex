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

- [ ] Run jitter detector on pinned core. Healthy core: max near median. Disturbed core: max of thousands of cycles.
- [ ] Read `/proc/interrupts` before and after a run. Column of isolated CPU must not rise. On x86_64: `LOC` = local timer, `NMI` = NMI, `RES` = reschedule.
- [ ] Call `nix::sched::sched_getcpu()` on hotpath thread. Result must be isolated CPU.
- [ ] Run kernel audit at startup: `isolcpus`, `nohz_full`, `nmi_watchdog`, SMT siblings.

## Rules

Host, once (root, reboot):

- [ ] Keep housekeeping cores for OS, SSH, IRQs, disk, slow path. Example: 8 cores, 0–3 housekeeping, 4–7 isolated.

CAUTION: Test a new boot line on a host that is not in production. A wrong boot line can stop the host from booting.

- [ ] Put same CPU list in each parameter of `GRUB_CMDLINE_LINUX_DEFAULT`:

  ```
  isolcpus=4-7 nohz_full=4-7 rcu_nocbs=4-7 processor.max_cstate=1 intel_idle.max_cstate=0 nmi_watchdog=0
  ```

  - `isolcpus`: no general task on CPU. Tick still runs.
  - `nohz_full`: stops scheduling tick only while exactly one runnable thread is on CPU. Small residual tick stays. Needs one CPU outside list.
  - `rcu_nocbs`: RCU callbacks run on housekeeping cores.
  - `processor.max_cstate=1`, `intel_idle.max_cstate=0`: no deep C-state, no wake-up latency.
  - `nmi_watchdog=0`: no lockup-detector NMI. Cost: kernel no longer detects frozen core.
- [ ] Route IRQs to housekeeping cores. Write housekeeping mask to `/proc/irq/*/smp_affinity`. Stop `irqbalance`, or ban isolated CPUs in it. Boot line does not do this.
- [ ] Disable SMT in BIOS. IF firmware is locked, THEN write `off` to `/sys/devices/system/cpu/smt/control`, or `0` to `/sys/devices/system/cpu/cpuN/online` of each sibling. Sibling numbers vary: read `thread_siblings_list`.

Process:

- [ ] One hotpath thread per isolated core. Keep `tokio`, `rayon` and other pools off it. Second runnable thread brings tick back.
- [ ] Take isolated CPU ids from config or `/sys/devices/system/cpu/isolated`.
- [ ] Pin as first statement of spawn closure: `CpuSet::new()`, `set.set(cpu)?`, `sched_setaffinity(Pid::from_raw(0), &set)?`. Pid 0 = calling thread.
- [ ] Allocate and touch hotpath data after pin, inside closure. Linux puts page on NUMA node of CPU that first writes it.
- [ ] Treat failed pin as fatal. Send `Result` to parent on `mpsc::sync_channel(1)`. Parent returns `Err` to `main`. Never fall back to housekeeping core.
- [ ] Pin slow path thread to housekeeping core.
- [ ] Run jitter detector at each start, on each pinned core, before network connect. IF max > threshold, THEN refuse to start.
- [ ] No sampling profiler on live host. It floods isolated cores with NMIs.
- [ ] CAUTION: A spinning `SCHED_FIFO` thread can starve the kernel threads of its CPU. That can stall other work on the host.
- [ ] IF housekeeping work must never preempt hotpath thread, THEN set `SCHED_FIFO` (`thread-priority` 3.1.1). It does not remove residual tick.

## Numbers

Typical figures. Measure on your own hardware.

| Item | Value |
| --- | --- |
| Scheduling tick | 100 to 1000 per second |
| NMI | control returns microseconds later |
| Detector threshold | 150 TSC cycles (example, set per host) |
| Spike on disturbed core | thousands of cycles |
| Cache for each busy SMT sibling | cut in half |

## Rust tools

| Item | Use here | Note |
| --- | --- | --- |
| `nix` 0.31.3, `default-features = false`, `features = ["sched"]` | Pin, read placement | Linux and Android only. `set.set(cpu)` returns `Err(EINVAL)` at or above `CPU_SETSIZE`. |
| `core_affinity` 0.8.3 | One-call pin for tools | `set_for_current` returns `bool` and drops errno. Test the `bool`. |

## Traps

- [ ] macOS has no pin call. `set_for_current` returns `false` there. Gate pin code with `#[cfg(target_os = "linux")]`.
- [ ] `core_affinity::get_core_ids()` and `available_parallelism()` see current affinity mask only. On `isolcpus` host, that is housekeeping CPUs.
- [ ] `move` closure that captures parent-built `Vec` brings memory from before pin, maybe on remote NUMA node.
- [ ] Dropped `JoinHandle` detaches thread. Keep handle + `Arc<AtomicBool>` stop flag. `Drop` stores `true` with `Release`, then joins.
- [ ] `compiler_fence(SeqCst)` emits no CPU instruction. `rdtsc` is not serializing. Neither matters for detector, because spike is thousands of cycles.
- [ ] 150-cycle threshold is in x86_64 TSC ticks. aarch64 `CNTVCT_EL0` ticks far slower. Without invariant TSC, `quanta` falls back to OS clock in ns. IF code runs on both, THEN compare `delta_as_nanos(start, end)`.
- [ ] `spin_loop()` (`pause` on x86_64, `isb sy` on aarch64) only adds delay with SMT off. IF loop must react in fewest cycles, THEN remove it.

## Code

IF you write startup checks, THEN read `.claude/skills/rex-code-latency/os/cpu-isolation-code.md`:

- Jitter detector: proves pinned core is quiet.
- Kernel audit: reads isolation settings back, so engine refuses a wrongly booted host.

## See also

- IF spike remains on quiet core, THEN read `.claude/skills/rex-code-latency/os/huge-pages.md`.
- IF you need full host order, THEN read `.claude/skills/rex-code-latency/setup.md`.
