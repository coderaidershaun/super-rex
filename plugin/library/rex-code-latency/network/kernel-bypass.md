# Kernel bypass

User-space packet path: when worth it, which route, hot loop, buffer ownership.

## Symptom

- Socket ladder is done, but kernel stamp gap still exceeds budget.
- Questions: "move feed to DPDK", "AF_XDP from Rust", "OpenOnload or `ef_vi`".
- Bypass code sends corrupt frames, or jitters when poll core feeds workers.

## Confirm

Propose bypass only after these results. Measure as `rex-code-latency/measure.md` says.

1. Measure kernel stamp gap and handler time, do socket ladder, measure again: `rex-code-latency/network/sockets.md`.
2. IF gap is still most of p99 AND above budget, THEN bypass is a candidate. ELSE fix handler.
3. IF budget is tens of µs, THEN stop: bypass removes about 3–10 µs.
4. Write down what bypass takes away. Get agreement first:
   - Kernel TCP/IP stack: raw routes give frames. You own ARP, checksums, TCP state. On DPDK you also send IGMP joins and answer queries.
   - Ordinary tools: `ip`, `ethtool`, `tcpdump`, firewall lose a DPDK NIC, unless the driver is bifurcated (`mlx5`). No tool sees bypassed packets: capture in app or switch.
   - Portability: Linux, listed NICs and drivers, vendor libraries, root, huge pages, one core at 100% per queue.
   - Safe Rust: FFI to C, `unsafe` ring calls.

## Rules

Route:

- [ ] IF Solarflare NIC and socket API must stay, THEN try OpenOnload first: `onload ./app`, no code change.
- [ ] IF Linux driver has AF_XDP zero-copy, THEN use AF_XDP: least hand-written `unsafe`.
- [ ] IF full control on commodity NIC, THEN use DPDK through own `bindgen` `sys` crate: no maintained Rust DPDK crate exists.
- [ ] IF Solarflare NIC and raw frames below 1 µs, THEN use `ef_vi` through C shim: macros defeat `bindgen`, no crate binds it.

CAUTION: Do bypass setup as root on a test host first. A wrong NIC bind or XDP program cuts the host off the network.

CAUTION: Do not bind the NIC that carries SSH or management traffic to DPDK. The kernel loses that interface, and you lose remote access to the host.

CAUTION: Steer traffic before you load an XDP program. The default `libxdp` program sends every packet of the bound queue, ARP and SSH included, to the socket. Put the feed on queue N with a Flow Steering rule, and take queue N out of RSS: `ethtool -X <iface> equal N` keeps other traffic on queues 0 to N−1. Check on your host.

Loop:

- [ ] Run to Completion on one isolated core: RX burst (up to 32), patch in place while the packet is in L3, TX burst, recycle completed TX buffers. Never sleep, yield or hand off to other cores.
- [ ] One RX ring, TX ring, event queue per core. Flow Steering maps each flow to one core.
- [ ] Pin AF_XDP thread yourself. DPDK EAL pins its lcores (`-c <core mask>`) and the thread that calls `rte_eal_init`.
- [ ] AF_XDP: bind with `XDP_USE_NEED_WAKEUP`, and kick each ring when the kernel asks. Else TX waits for a driver poll.

Ownership:

- [ ] Never touch, free or repost a TX buffer before TX completion.
- [ ] Let driver ring doorbell. IF you write one, THEN `fence(Release)`, device store barrier (x86_64 `_mm_sfence()`, aarch64 `asm!("dmb oshst")`), `write_volatile`. Fence alone does not order MMIO.
- [ ] Put all `unsafe` for one C library in one module, with owner types that free in `Drop`. Set `panic = "abort"` in release profile.

In place:

- [ ] View headers through `zerocopy` structs of byte-order fields (alignment 1): `ref_from_prefix`, `mut_from_prefix`. Not `read_from_prefix`: it copies.
- [ ] IF payload or length changes, THEN recompute IPv4 and TCP or UDP checksums, or offload them.

## Numbers

Typical figures. Measure on your own hardware.

| Item | Value |
| --- | --- |
| First packet read: L3 with DDIO, RAM without | about 15 ns, about 100 ns |
| Cross-core buffer handoff | hundreds of ns jitter |

## Rust tools

- `xsk-rs` 0.11.0 (Linux target): AF_XDP with default XDP program. Builds `libxdp` + `libbpf` from C.
- `xdpilone` 1.3.0, `xdp` 0.8.0: AF_XDP, no C build. Load XDP program yourself (`aya` 0.16.0).
- `bindgen` 0.73.2 (`default-features = false`, `features = ["runtime"]`), `cc` 1.11.1, `pkg-config` 0.3.34: DPDK `sys` crate.
- `zerocopy` 0.8.63 (`default-features = false`, `features = ["derive"]`): checked in-place views.

## Traps

- OpenOnload intercepts libc calls only: `rustix` (raw syscalls by default) and `io_uring` skip it.
- `#[repr(packed)]` field reference is error E0793: use alignment-1 byte-order fields.
- Hand-written NIC ring: `Acquire` load of ownership flag, read payload from `UnsafeCell` (DMA writes it), `Release` store. These order threads, not a device. aarch64 also needs `dmb oshld` after the flag load and `dmb oshst` before the flag store (Linux `dma_rmb`, `dma_wmb`). Check on your host.
- Owner type with raw pointer (`Mbuf`, VI handle) is `!Send`. Create it on polling core.
- `ef_vi` and AF_XDP buffer ids come from the NIC at run time: keep the ownership rule in `// SAFETY:` comments.

## Code

Before you write code, read its part of `rex-code-latency/network/kernel-bypass-code.md`: AF_XDP loop, in-place header patch, DPDK owner types (not compile-checked against DPDK), `ef_vi` and doorbell notes.

## See also

- IF you pin poll thread to isolated core, THEN read `rex-code-latency/os/cpu-isolation.md`.
- IF you put packet buffers in huge pages, THEN read `rex-code-latency/os/huge-pages.md`.
- IF you write NIC ring or Pipeline model handoff, THEN read `rex-code-latency/threads/ring-buffers.md`.
