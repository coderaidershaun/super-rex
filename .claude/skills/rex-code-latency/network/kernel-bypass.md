# Kernel bypass

User-space packet path: when worth it, which route, hot loop, buffer ownership.

## Symptom

- Socket ladder is done, but kernel stamp gap still exceeds budget.
- Questions: "move feed to DPDK", "AF_XDP from Rust", "OpenOnload or `ef_vi`".
- Bypass code sends corrupt frames, or jitters when poll core feeds workers.

## Confirm

Propose bypass only after these results. Measure as `.claude/skills/rex-code-latency/measure.md` says.

1. Measure kernel stamp gap and handler time, do socket ladder, measure again: `.claude/skills/rex-code-latency/network/sockets.md`.
2. IF gap is still most of p99 AND above budget, THEN bypass is a candidate. ELSE fix handler.
3. IF budget is tens of µs, THEN stop: bypass removes about 3–10 µs.
4. Write down what bypass takes away. Get agreement before work starts:
   - Kernel TCP/IP stack: raw routes give frames. You own ARP, checksums, TCP state.
   - Ordinary tools: `ip`, `ethtool`, `tcpdump`, firewall lose a DPDK NIC. XDP-redirected packets skip `tcpdump` and firewall.
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

CAUTION: Steer traffic before you load an XDP program. The default `libxdp` program sends every packet of the bound queue, ARP and SSH included, to the socket. Put only the feed on that queue with a Flow Steering rule.

Loop:

- [ ] Run to Completion on one isolated core: RX burst (up to 32), patch in place, TX burst, recycle completed TX buffers. Never sleep, yield or hand off to other cores.
- [ ] No branch hint on empty-poll test. A hint for empty case moves packet path out of line.
- [ ] Process each packet at once, before it leaves L3.
- [ ] One RX ring, TX ring, event queue per core. Flow Steering maps each flow to one core.
- [ ] Pin AF_XDP thread yourself. DPDK EAL pins lcores (`-c <core mask>`).

Ownership:

- [ ] Never touch, free or repost a TX buffer before TX completion.
- [ ] Let driver ring doorbell. IF you write one, THEN `fence(Release)`, device store barrier (x86_64 `_mm_sfence()`, aarch64 `asm!("dmb oshst")`), `write_volatile`. Fence alone does not order MMIO.
- [ ] Put all `unsafe` for one C library in one module, with owner types that free in `Drop`. Set `panic = "abort"` in release profile.

In place:

- [ ] View headers through `zerocopy` structs of byte-order fields (alignment 1): `ref_from_prefix`, `mut_from_prefix`. Not `read_from_prefix`: it copies.
- [ ] IF payload or length changes, THEN recompute IPv4 + TCP checksums, or offload them.

## Numbers

Typical figures. Measure on your own hardware.

| Item | Value |
| --- | --- |
| First packet read: L3 with DDIO, RAM without | about 15 ns, about 100 ns |
| DDIO share of L3 | max 2 ways, about 10% |
| NIC DMA read of TX buffer | several hundred ns |
| Cross-core buffer handoff | hundreds of ns jitter |

## Rust tools

- `xsk-rs` 0.11.0 (Linux target): AF_XDP with default XDP program. Builds `libxdp` + `libbpf` from C.
- `xdpilone` 1.3.0, `xdp` 0.8.0: AF_XDP, no C build. Load XDP program yourself (`aya` 0.16.0).
- `bindgen` 0.73.2 (`default-features = false`, `features = ["runtime"]`), `cc` 1.11.1, `pkg-config` 0.3.34: DPDK `sys` crate.
- `zerocopy` 0.8.63 (`default-features = false`, `features = ["derive"]`): checked in-place views.
- `smoltcp` 0.14.0: user-space TCP/IP for raw frames.

## Traps

- OpenOnload intercepts libc calls only. `rustix` makes raw syscalls by default on Linux and skips it.
- `#[repr(packed)]` field reference is error E0793. `#[repr(align(64))]` header at slot offset 4 is misaligned. Use byte-order fields + `Unaligned`.
- Hand-written NIC ring: `Acquire` load of ownership flag, read payload from `UnsafeCell` (DMA writes it), end borrow, `Release` store.
- Owner type with raw pointer (`Mbuf`, VI handle) is `!Send`. Create it on polling core.
- `ef_vi` and AF_XDP buffer ids come from NIC at run time. No type proves an id returns once: keep rule in `// SAFETY:` comments.

## Code

In `.claude/skills/rex-code-latency/network/kernel-bypass-code.md`:

- AF_XDP Run to Completion loop (`xsk-rs`). Read before you write AF_XDP code.
- In-place patch of Ethernet, IPv4, TCP headers. Read before you parse or edit frames.
- DPDK owner types, `build.rs`, shim (not compile-checked against DPDK). Then `ef_vi` and doorbell notes. Read before FFI or doorbell code.

## See also

- IF you pin poll thread to isolated core, THEN read `.claude/skills/rex-code-latency/os/cpu-isolation.md`.
- IF you put packet buffers in huge pages, THEN read `.claude/skills/rex-code-latency/os/huge-pages.md`.
- IF you write NIC ring or Pipeline model handoff, THEN read `.claude/skills/rex-code-latency/threads/ring-buffers.md`.
