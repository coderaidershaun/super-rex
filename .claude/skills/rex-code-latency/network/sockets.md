# Kernel sockets

Cost of a Linux kernel receive, what tuning removes, where tuning stops.

## Symptom

- Several µs from packet arrival to handler, at p50 and p99.
- Hotpath thread waits in `tokio`, `mio`, `epoll_wait` or blocking `std::net` calls.
- First packet after quiet period is slow: thread wakes from sleep.
- Questions: "is `tokio` too slow for the feed", "busy poll a socket", "`io_uring` instead of `recv`".

## Confirm

Measure as `.claude/skills/rex-code-latency/measure.md` says. Tune only after one of these results.

- [ ] Kernel stamp gap: `SO_TIMESTAMPNS` stamp to `CLOCK_REALTIME` at handler entry. Gap = stack + wake-up + copy. Cause confirmed IF gap is µs and handler time is small next to it.
  - Read the stamp with `nix` (`sockopt::ReceiveTimestampns`, `recvmsg`, `ControlMessageOwned::ScmTimestampns`). std and `socket2` cannot.
  - Kernel stamps after the interrupt. It misses NIC interrupt coalescing (`rx-usecs`), which can be most of wire to handler. See it with NIC hardware stamps (`SO_TIMESTAMPING`) or a switch capture.
- [ ] Count per packet on hotpath thread: `perf stat -e context-switches,raw_syscalls:sys_enter -t <tid>`, divide by packets. Cause confirmed IF about 1 context switch or 2+ syscalls per packet.
- [ ] A/B: replace wait with spin (step 1). Cause confirmed IF p99 falls by µs.
- [ ] IF handler time is most of p99, THEN stop. This file does not apply.

## Rules

Ladder. Do one step, measure, stop IF budget is met.

- [ ] Keep async runtimes (`tokio`, `async-std`, `smol`) and `mio` off hotpath. Reactor parks in `epoll_wait`. Readiness is not data: `recv` is a second syscall.
- [ ] Run async for control plane on housekeeping cores. Feed hotpath over SPSC ring buffer (`rtrb`).
- [ ] Step 1, never sleep: `set_nonblocking(true)`, loop on `recv`, `std::hint::spin_loop()` on `WouldBlock`. Removes wake-up context switch.
- [ ] Step 2, `SO_BUSY_POLL`: `socket2::Socket::set_busy_poll(50)` (µs, usual start). Each `recv` runs driver poll of NIC queue in calling thread. No interrupt wait. It polls only the queue that last delivered: keep the feed on one queue.
- [ ] Set every option with `socket2` before `bind`, then `socket.into()` to `std::net::UdpSocket`.

CAUTION: Change sysctl, IRQ affinity and `ethtool` settings as root on a test host first. Each change acts on every socket of the host.

- [ ] Pin hotpath thread to isolated core. Route NIC IRQs to housekeeping cores on NUMA node of NIC.
- [ ] Also test `SO_PREFER_BUSY_POLL` (Linux 5.11+). It does little unless `/sys/class/net/<iface>/napi_defer_hard_irqs` and `gro_flush_timeout` are above 0. Check on your host.
- [ ] Also test lower NIC interrupt coalescing: `ethtool -C <iface> adaptive-rx off rx-usecs 0`.
- [ ] Step 3, `io_uring` SQPOLL (Linux 5.11+, unprivileged 5.13+): kernel thread polls submission ring, hotpath thread makes no syscall. Pin kernel thread to its own isolated core with `setup_sqpoll_cpu`. Cost: 2 cores per feed. A/B it against step 2: the interrupt comes back.
- [ ] IF all 3 steps are on AND kernel stamp gap still exceeds budget, THEN stop tuning and read `.claude/skills/rex-code-latency/network/kernel-bypass.md`.

## Numbers

Typical figures. Measure on your own hardware.

| Item | Cost |
| --- | --- |
| Kernel path, packet at NIC to user code | 3–10 µs |
| Context switch, overhead only | 500–1000 ns |
| One `send`, with cache refill after | about 2000 ns |

What each step still pays per packet:

- `tokio` or `mio` + `recv`: wait, 2+ syscalls, stack, copy.
- Spin + `SO_BUSY_POLL`: 1 syscall, stack, copy.
- `io_uring` SQPOLL: interrupt, stack and copy on other core. 0 syscalls on hotpath thread.
- AF_XDP zero-copy: no stack, no copy. RX: 0 syscalls, NIC interrupt. TX: `sendto` when kernel asks. Busy poll: 1 syscall per batch, no interrupt.
- DPDK, `ef_vi`: none of these.

## Rust tools

- `socket2` 0.6.5, `default-features = false`, `features = ["all"]`: options before `bind`, `set_busy_poll`.
- `io-uring` 0.7.16, `default-features = false`, Linux target only: SQPOLL, multishot, provided buffers.
- `nix` 0.31.3, `default-features = false`, `features = ["socket", "uio"]`: receive stamps.
- `libc` 0.2.191: `SO_PREFER_BUSY_POLL` has no `socket2` setter. Use `unsafe` `libc::setsockopt`.
- `mio` 1.2.4 (`default-features = false`, `features = ["os-poll", "net"]`), `tokio` 1.54.1: control plane only.

## Traps

- `mio` is edge-triggered. After each readiness event, DO `recv` UNTIL `WouldBlock`. One `recv` per event strands packets.
- `tokio`: each wake is an atomic count + run-queue push, plus a futex or `eventfd` write IF the worker is parked. Multi-thread runtime moves tasks to cold cores. Cooperative budget forces yields at points you do not choose.
- `SubmissionQueue::push` is `unsafe`: kernel writes buffer after call. Do not move, drop or read buffer before its completion entry, on any path: abort IF `submit()` fails after a push. `tokio-uring` owns buffers safely but brings a runtime back.
- SQPOLL thread sleeps after `setup_sqpoll(idle_ms)` with no work. Next `submit()` makes a syscall to wake it. Set idle time above longest quiet gap of feed.
- Without `CAP_NET_ADMIN`, a `SO_BUSY_POLL` value above `net.core.busy_read` (default 0) gives `EPERM`. `SO_PREFER_BUSY_POLL` needs it too.
- Kernel caps `set_recv_buffer_size` at `net.core.rmem_max` with no error. Read it back: Linux reports double the size it kept.

## Code

In `.claude/skills/rex-code-latency/network/sockets-code.md`:

- Busy-poll UDP socket: read before steps 1 and 2.
- `io_uring` SQPOLL receive: read before step 3.

## See also

- IF you pin hotpath thread or route IRQs, THEN read `.claude/skills/rex-code-latency/os/cpu-isolation.md`.
- IF control plane hands data to hotpath, THEN read `.claude/skills/rex-code-latency/threads/ring-buffers.md`.
