# Kernel bypass code

Three listings for `.claude/skills/rex-code-latency/network/kernel-bypass.md`.
The AF_XDP loop imports the in-place patch of the second listing as `crate::turnaround`.

```toml
[dependencies]
arrayvec = { version = "0.7.8", default-features = false }
thiserror = { version = "2.0.21", default-features = false }
zerocopy = { version = "0.8.63", default-features = false, features = ["derive"] }

[target.'cfg(target_os = "linux")'.dependencies]
xsk-rs = "0.11.0" # no default features. libxdp-sys builds libxdp and libbpf from C: needs clang, libelf, zlib

[build-dependencies]
bindgen = { version = "0.73.2", default-features = false, features = ["runtime"] }
cc = "1.11.1"
pkg-config = "0.3.34"

[profile.release]
panic = "abort"
```

## AF_XDP Run to Completion loop with `xsk-rs`

Compile-checked: cargo check + clippy, x86_64-unknown-linux-gnu, against real `xsk-rs` 0.11.0, with check-only stubs for the C builds of `libbpf-sys` and `libxdp-sys`. Not run.

- Pin thread to isolated core before call. Loop does not pin itself.
- `Umem::new(.., true)` maps frames in huge pages. Reserve them first.
- `FRAME_COUNT` = default size of each ring, so `produce` cannot fall short. IF you change a ring size, THEN keep each ring at `FRAME_COUNT` or more.
- `XDP_ZEROCOPY` fails at bind IF driver has no zero-copy mode. Without flag: copy mode, slower.
- `produce_and_wakeup` makes a `sendto` syscall when kernel asks for wakeup. `XDP_USE_NEED_WAKEUP` and `SO_PREFER_BUSY_POLL` make it rare.
- 6 `unsafe` blocks: `xsk-rs` cannot check frame aliasing. Each frame is in one place only: RX batch, `send`, `recycle`, or one ring.
- Completion ring to fill ring = TX completion. A sent frame goes back to RX only after kernel finished its read.

```rust
//! Run to Completion turnaround over an AF_XDP socket with xsk-rs.
#![cfg(target_os = "linux")]
use crate::turnaround::{ExchangeExecutionOrder, process_and_turnaround_packet};
use core::num::NonZeroU32;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use xsk_rs::config::{BindFlags, SocketConfig, UmemConfig};
use xsk_rs::socket::SocketCreateError;
use xsk_rs::umem::UmemCreateError;
use xsk_rs::{Socket, Umem};

/// Equal to the default size of every ring, so each ring can hold every frame.
const FRAME_COUNT: u32 = 2048;
const BATCH: usize = 32;

#[derive(Debug, thiserror::Error)]
pub enum XdpError {
    #[error("interface name contains a NUL byte")]
    BadInterface,
    #[error(transparent)]
    Umem(#[from] UmemCreateError),
    #[error(transparent)]
    Socket(#[from] SocketCreateError),
    #[error("socket came without a fill queue and a completion queue")]
    NoFillQueue,
    #[error("TX wakeup failed: {0}")]
    Wakeup(#[from] std::io::Error),
}

pub fn run_xdp_turnaround(
    ifname: &str,
    queue_id: u32,
    session_active: &AtomicBool,
    mut evaluate: impl FnMut(&[u8]) -> Option<ExchangeExecutionOrder>,
) -> Result<(), XdpError> {
    let frames = NonZeroU32::new(FRAME_COUNT).expect("FRAME_COUNT is not zero");
    let (umem, descs) = Umem::new(UmemConfig::default(), frames, true)?;
    let ifname = ifname.parse().map_err(|_| XdpError::BadInterface)?;
    let config = SocketConfig::builder().bind_flags(BindFlags::XDP_ZEROCOPY).build();
    // SAFETY: this UMEM is not shared with another socket, so the shared-UMEM rule does not apply.
    let (mut tx_q, mut rx_q, rings) = unsafe { Socket::new(config, &umem, &ifname, queue_id) }?;
    let (mut fill_q, mut comp_q) = rings.ok_or(XdpError::NoFillQueue)?;
    // SAFETY: every descriptor belongs to this UMEM and none is in a ring yet.
    unsafe { fill_q.produce(&descs) };

    // Consume overwrites these scratch descriptors, so the loop does not allocate.
    let mut rx = descs[..BATCH].to_vec();
    let mut done = descs[..BATCH].to_vec();
    let (mut send, mut recycle) = (Vec::with_capacity(BATCH), Vec::with_capacity(BATCH));

    while session_active.load(Relaxed) {
        // SAFETY: the descriptors belong to this UMEM.
        let got = unsafe { rx_q.consume(&mut rx) };
        for desc in &mut rx[..got] {
            // SAFETY: the kernel gave this frame to us on the RX ring. Nothing else reads or
            // writes it until we put it on the TX ring or the fill ring.
            let mut data = unsafe { umem.data_mut(desc) };
            let new_len = evaluate(data.contents())
                .and_then(|order| process_and_turnaround_packet(data.contents_mut(), &order).ok());
            match new_len {
                Some(len) => {
                    data.cursor().set_pos(len);
                    send.push(*desc);
                }
                None => recycle.push(*desc),
            }
        }
        // SAFETY: each frame is in exactly one of `send` and `recycle`, and we own it. Every ring
        // holds FRAME_COUNT entries, so a produce call cannot fall short.
        unsafe {
            if !send.is_empty() {
                tx_q.produce_and_wakeup(&send)?;
            }
            fill_q.produce(&recycle);
        }
        send.clear();
        recycle.clear();
        // TX completion: only frames that the kernel finished reading go back to the fill ring.
        // SAFETY: the descriptors belong to this UMEM; completed frames are ours again.
        unsafe {
            let finished = comp_q.consume(&mut done);
            fill_q.produce(&done[..finished]);
        }
    }
    Ok(())
}
```

## In-place header patch with `zerocopy`

Compile-checked: cargo check + clippy, aarch64-apple-darwin and x86_64-unknown-linux-gnu. A unit test on a built TCP frame passes. It asserts that both checksums are valid and that seq and ack moved.

- No `unsafe`. Each header split is checked. All checks run before first write, so a rejected frame is unchanged and can go back to RX.
- Byte-order fields have alignment 1. Structs have no padding and need no `repr(packed)`.
- Reads IHL and TCP data offset. IP and TCP options work.
- Swap of addresses alone does not change either checksum: ones' complement sum is commutative. New payload and new length do. Recompute both, or offload them (DPDK notes below, `ef_vi` notes at end).
- `ExchangeExecutionOrder` is an example. Use your venue's wire format. Venues send fixed-point integer prices, not `f64`.
- Frame must already hold the order. Buffer does not grow.
- For real orders, patch one pre-built frame per session. Market data comes as UDP multicast. Orders go on TCP to another gateway. A reflected market data frame is rarely a valid order frame.

```rust
//! In-place turnaround: the inbound TCP frame becomes the outbound order.
use core::mem::{size_of, swap};
use zerocopy::byteorder::{little_endian, network_endian};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

#[derive(FromBytes, IntoBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct EthernetHeader {
    pub destination_mac: [u8; 6],
    pub source_mac: [u8; 6],
    pub ether_type: network_endian::U16,
}

#[derive(FromBytes, IntoBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct Ipv4Header {
    pub version_ihl: u8,
    pub dscp_ecn: u8,
    pub total_length: network_endian::U16,
    pub identification: network_endian::U16,
    pub flags_fragment: network_endian::U16,
    pub ttl: u8,
    pub protocol: u8,
    pub checksum: network_endian::U16,
    pub source: network_endian::U32,
    pub destination: network_endian::U32,
}

#[derive(FromBytes, IntoBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct TcpHeader {
    pub source_port: network_endian::U16,
    pub destination_port: network_endian::U16,
    pub sequence: network_endian::U32,
    pub acknowledgment: network_endian::U32,
    pub offset_flags: network_endian::U16,
    pub window: network_endian::U16,
    pub checksum: network_endian::U16,
    pub urgent: network_endian::U16,
}

#[derive(Clone, Copy, FromBytes, IntoBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct ExchangeExecutionOrder {
    pub instrument_identifier: little_endian::U32,
    pub order_volume: little_endian::U32,
    pub execution_price: little_endian::F64,
    pub order_direction: u8,
}

const _: () = assert!(size_of::<EthernetHeader>() == 14);
const _: () = assert!(size_of::<Ipv4Header>() == 20);
const _: () = assert!(size_of::<TcpHeader>() == 20);
const _: () = assert!(size_of::<ExchangeExecutionOrder>() == 17);

const ETHER_TYPE_IPV4: u16 = 0x0800;
const PROTOCOL_TCP: u8 = 6;
const ORDER_LEN: usize = size_of::<ExchangeExecutionOrder>();

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum TurnaroundError {
    #[error("frame is shorter than its headers say")]
    Truncated,
    #[error("frame is not IPv4")]
    NotIpv4,
    #[error("packet is not TCP")]
    NotTcp,
}

/// Rewrites `frame` in place into an order back to the sender.
/// Returns the new frame length.
pub fn process_and_turnaround_packet(
    frame: &mut [u8],
    order: &ExchangeExecutionOrder,
) -> Result<usize, TurnaroundError> {
    use TurnaroundError::{NotIpv4, NotTcp, Truncated};

    let (eth, rest) = EthernetHeader::mut_from_prefix(frame).map_err(|_| Truncated)?;
    if eth.ether_type.get() != ETHER_TYPE_IPV4 {
        return Err(NotIpv4);
    }
    let (ip, rest) = Ipv4Header::mut_from_prefix(rest).map_err(|_| Truncated)?;
    let ip_len = usize::from(ip.version_ihl & 0x0f) * 4;
    if ip.version_ihl >> 4 != 4 || ip_len < size_of::<Ipv4Header>() {
        return Err(NotIpv4);
    }
    if ip.protocol != PROTOCOL_TCP {
        return Err(NotTcp);
    }
    let (ip_options, rest) = rest.split_at_mut_checked(ip_len - 20).ok_or(Truncated)?;
    let (tcp, rest) = TcpHeader::mut_from_prefix(rest).map_err(|_| Truncated)?;
    let tcp_len = usize::from(tcp.offset_flags.get() >> 12) * 4;
    let tcp_options_len = tcp_len.checked_sub(20).ok_or(Truncated)?;
    let (tcp_options, payload) = rest.split_at_mut_checked(tcp_options_len).ok_or(Truncated)?;
    let inbound_payload = usize::from(ip.total_length.get())
        .checked_sub(ip_len + tcp_len)
        .ok_or(Truncated)?;
    let (out, _) = ExchangeExecutionOrder::mut_from_prefix(payload).map_err(|_| Truncated)?;

    swap(&mut eth.destination_mac, &mut eth.source_mac);
    swap(&mut ip.source, &mut ip.destination);
    swap(&mut tcp.source_port, &mut tcp.destination_port);
    let next_ack = tcp.sequence.get().wrapping_add(inbound_payload as u32);
    tcp.sequence = tcp.acknowledgment;
    tcp.acknowledgment.set(next_ack);
    *out = *order;

    let segment_len = tcp_len + ORDER_LEN;
    ip.total_length.set((ip_len + segment_len) as u16);
    ip.ttl = 64;
    ip.checksum.set(0);
    ip.checksum.set(fold(sum(ip.as_bytes()) + sum(ip_options)));

    // The TCP checksum covers a pseudo header of addresses, protocol, and segment length.
    let pseudo = sum(ip.source.as_bytes())
        + sum(ip.destination.as_bytes())
        + u32::from(PROTOCOL_TCP)
        + segment_len as u32;
    tcp.checksum.set(0);
    let segment = sum(tcp.as_bytes()) + sum(tcp_options) + sum(out.as_bytes());
    tcp.checksum.set(fold(pseudo + segment));

    Ok(size_of::<EthernetHeader>() + ip_len + segment_len)
}

/// Sum of big-endian 16-bit words. An odd last byte is padded with zero.
fn sum(bytes: &[u8]) -> u32 {
    let mut words = bytes.chunks_exact(2);
    let mut acc: u32 = words
        .by_ref()
        .map(|w| u32::from(u16::from_be_bytes([w[0], w[1]])))
        .sum();
    if let [last] = words.remainder() {
        acc += u32::from(*last) << 8;
    }
    acc
}

fn fold(mut acc: u32) -> u16 {
    while acc > 0xffff {
        acc = (acc & 0xffff) + (acc >> 16);
    }
    !(acc as u16)
}
```

## DPDK owner types

Compile-checked: cargo check + clippy, x86_64-unknown-linux-gnu, against a hand-written `sys` stand-in with the same names and C types as the `bindgen` output.
Not compile-checked against real `bindgen` output or against the C shim: needs DPDK. Not run.

Declare module as `#[cfg(target_os = "linux")] mod dpdk;`. All DPDK `unsafe` stays in it. `sys` comes from `build.rs`:

1. `pkg_config::probe_library("libdpdk")` prints link lines. Pass output of `pkg-config --cflags libdpdk` to `bindgen` and to `cc`.
2. `bindgen` reads `wrapper.h` (`rte_eal.h`, `rte_errno.h`, `rte_ethdev.h`, `rte_mbuf.h`, shim header) with `.allowlist_function("rte_(eal|eth|pktmbuf|mempool)_.*|shim_.*")`, `.allowlist_type("rte_eth_conf")`, `.derive_default(true)`, `.wrap_static_fns(true)`.
3. Give `bindgen` an absolute header path. Generated `static_fns.c` includes it from `OUT_DIR`.
4. `cc` compiles `static_fns.c` and a C shim. `bindgen` binds no function-like macro, no thread-local, and gives anonymous-union fields `__bindgen_anon_N` names that change with DPDK version. Shim:
   - `int shim_errno(void)`: returns `rte_errno`.
   - `uint8_t *shim_mbuf_data(struct rte_mbuf *m, uint16_t *len)`: sets `*len` from `rte_pktmbuf_data_len`, returns `rte_pktmbuf_mtod(m, uint8_t *)`.
   - `int shim_mbuf_set_len(struct rte_mbuf *m, uint16_t len)`: IF `nb_segs != 1` or `len > buf_len - data_off`, THEN returns -1. ELSE sets `data_len` and `pkt_len`.

What the listing encodes:

- `Mbuf` is `#[repr(transparent)]` over `NonNull<rte_mbuf>`. `ArrayVec<Mbuf, 32>` has layout of the C pointer array that `rte_eth_rx_burst` fills.
- `tx_burst` calls `mem::forget` on each mbuf that driver accepted. PMD frees it after TX completion. Use after send and double free do not compile.
- `PhantomData<&'p Mempool>` ties each mbuf and port to pool. Pool freed before port does not compile.
- `rte_eal_init` runs once per process and can write its argv: static `AtomicBool`, owned writable buffers.
- Zeroed `rte_eth_conf`. Current DPDK rejects a null config with `-EINVAL`.
- Pool 16383 = 2^14 − 1. Ring under the pool has power-of-two size and keeps one slot empty.
- Per-lcore cache 512: alloc and free are local pushes. Global ring takes only bulk moves.
- Data room 2048 includes 128 bytes of default headroom: 1920 frame bytes. Private area 0 keeps payload cache-line aligned. 1024 descriptors per queue.
- `Mbuf` is `!Send`: Run to Completion compiles as is. Pipeline model needs `unsafe impl Send for Mbuf<'_>`.

Use:

- EAL arguments: `-c <isolated core mask>`, `-n <memory channels>`, `--no-shconf` (no multi-process shared config, so no file system jitter).
- Loop: `rx_burst`, then `drain(..)` the burst. Drop of an `Mbuf` recycles it. Push turned-around mbufs to a TX `Burst`, call `tx_burst`, then `clear()` it. Mbufs left after a full TX ring drop there: count them as drops.
- Checksum offload: enable `RTE_ETH_TX_OFFLOAD_IPV4_CKSUM | RTE_ETH_TX_OFFLOAD_TCP_CKSUM` in `rte_eth_conf.txmode.offloads`. In the shim, set `RTE_MBUF_F_TX_IPV4 | RTE_MBUF_F_TX_IP_CKSUM | RTE_MBUF_F_TX_TCP_CKSUM`, `l2_len` and `l3_len` on the mbuf. Put the pseudo-header sum (`rte_ipv4_phdr_cksum`) in the TCP checksum field. This listing does not do it.

Limits:

- `wrap_static_fns` costs one call per burst and one per free that Rust cannot inline. Cross-language LTO removes it: build shim with clang of same LLVM version as rustc, pass `-Clinker-plugin-lto`.
- `data_mut` gives first segment only. Chained or jumbo mbufs need a segment walk.
- EAL is never cleaned up. It lives until process exit.
- Sibling lcores: `rte_eal_remote_launch` takes `extern "C" fn(*mut c_void) -> c_int`. Pass closure as boxed raw pointer through a trampoline. That is one more `unsafe` point.

```rust
//! Safe owner types over DPDK. Every unsafe call into DPDK lives in this module.
use arrayvec::ArrayVec;
use core::ffi::{c_char, c_int};
use core::marker::PhantomData;
use core::ptr::{self, NonNull};
use std::ffi::CString;
use std::sync::atomic::{AtomicBool, Ordering::AcqRel};

#[allow(non_camel_case_types, non_snake_case, non_upper_case_globals, dead_code)]
mod sys {
    include!(concat!(env!("OUT_DIR"), "/dpdk_sys.rs"));
}

pub const BURST: usize = 32;
const TOTAL_POOL_ELEMENTS: u32 = 16_383; // 2^14 - 1
const PER_CORE_CACHE_SIZE: u32 = 512;
const PACKET_DATA_ROOM_SIZE: u16 = 2048;
const HARDWARE_RING_DESCRIPTOR_COUNT: u16 = 1024;
const SOCKET_ZERO: c_int = 0;

#[derive(Debug, thiserror::Error)]
pub enum DpdkError {
    #[error("the EAL is already initialized in this process")]
    AlreadyInitialized,
    #[error("an EAL argument contains a NUL byte")]
    BadArgument,
    #[error("{call} failed with code {code}")]
    Call { call: &'static str, code: i32 },
    #[error("length {0} does not fit in one mbuf segment")]
    TooLong(usize),
}

fn check(call: &'static str, rc: c_int) -> Result<(), DpdkError> {
    if rc < 0 {
        return Err(DpdkError::Call { call, code: rc });
    }
    Ok(())
}

fn last_errno() -> i32 {
    // SAFETY: reads this thread's rte_errno through the shim. No pointer is involved.
    unsafe { sys::shim_errno() }
}

static EAL_STARTED: AtomicBool = AtomicBool::new(false);

/// Proof that the EAL runs. The EAL can start only once per process, and it lives until exit.
pub struct Eal {
    _args: Vec<Vec<u8>>,
    _argv: Vec<*mut c_char>,
}

impl Eal {
    pub fn init(args: &[&str]) -> Result<Self, DpdkError> {
        if EAL_STARTED.swap(true, AcqRel) {
            return Err(DpdkError::AlreadyInitialized);
        }
        // rte_eal_init may write the strings, so each one is an owned, writable buffer.
        let mut args = args
            .iter()
            .map(|a| CString::new(*a).map(CString::into_bytes_with_nul))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| DpdkError::BadArgument)?;
        let mut argv: Vec<*mut c_char> = args.iter_mut().map(|a| a.as_mut_ptr().cast()).collect();
        // SAFETY: argv holds argc writable, NUL-terminated buffers. Both vectors move into
        // Self and the heap buffers do not move, so they outlive the EAL, which can permute
        // and modify them.
        let rc = unsafe { sys::rte_eal_init(argv.len() as c_int, argv.as_mut_ptr()) };
        check("rte_eal_init", rc)?;
        Ok(Self { _args: args, _argv: argv })
    }
}

pub struct Mempool(NonNull<sys::rte_mempool>);

impl Mempool {
    pub fn new(_eal: &Eal) -> Result<Self, DpdkError> {
        // SAFETY: the EAL runs (proved by `&Eal`) and the name is a static C string.
        let raw = unsafe {
            sys::rte_pktmbuf_pool_create(
                c"hft_packet_pool".as_ptr(),
                TOTAL_POOL_ELEMENTS,
                PER_CORE_CACHE_SIZE,
                0, // no private area, so the payload stays cache-line aligned
                PACKET_DATA_ROOM_SIZE,
                SOCKET_ZERO,
            )
        };
        NonNull::new(raw).map(Self).ok_or_else(|| DpdkError::Call {
            call: "rte_pktmbuf_pool_create",
            code: last_errno(),
        })
    }
}

impl Drop for Mempool {
    fn drop(&mut self) {
        // SAFETY: every Port<'p> and Mbuf<'p> borrows this pool, so all of them are gone:
        // no port posts its buffers, and no mbuf is outside the pool.
        unsafe { sys::rte_mempool_free(self.0.as_ptr()) }
    }
}

/// One owned packet buffer. Drop returns it to the lcore cache of its pool.
#[repr(transparent)]
pub struct Mbuf<'p> {
    raw: NonNull<sys::rte_mbuf>,
    _pool: PhantomData<&'p Mempool>,
}

impl Mbuf<'_> {
    #[inline]
    pub fn data_mut(&mut self) -> &mut [u8] {
        let mut len = 0u16;
        // SAFETY: this value owns the mbuf, so no other code and no NIC queue touches its
        // bytes. The shim returns the start of the first segment and its length.
        unsafe {
            let data = sys::shim_mbuf_data(self.raw.as_ptr(), &mut len);
            core::slice::from_raw_parts_mut(data, usize::from(len))
        }
    }

    #[inline]
    pub fn set_len(&mut self, len: usize) -> Result<(), DpdkError> {
        let short = u16::try_from(len).map_err(|_| DpdkError::TooLong(len))?;
        // SAFETY: this value owns the mbuf. The shim checks the length against the buffer room.
        let rc = unsafe { sys::shim_mbuf_set_len(self.raw.as_ptr(), short) };
        if rc < 0 {
            return Err(DpdkError::TooLong(len));
        }
        Ok(())
    }
}

impl Drop for Mbuf<'_> {
    #[inline]
    fn drop(&mut self) {
        // SAFETY: this value owns the mbuf and drop runs once. The free pushes it to the lcore cache.
        unsafe { sys::rte_pktmbuf_free(self.raw.as_ptr()) }
    }
}

/// 32 pointers take 256 bytes, which is four cache lines.
#[repr(C, align(64))]
pub struct Burst<'p>(pub ArrayVec<Mbuf<'p>, BURST>);
const _: () = assert!(size_of::<Burst<'static>>() == 320); // 256 B of pointers + length, padded

impl Burst<'_> {
    pub fn new() -> Self {
        Self(ArrayVec::new())
    }
}

impl Default for Burst<'_> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Port<'p> {
    id: u16,
    _pool: PhantomData<&'p Mempool>,
}

impl<'p> Port<'p> {
    pub fn start(id: u16, pool: &'p Mempool) -> Result<Self, DpdkError> {
        let port = Self { id, _pool: PhantomData };
        // Current DPDK rejects a null config with -EINVAL.
        let conf = sys::rte_eth_conf::default();
        // SAFETY: the EAL runs (the pool exists). `conf` outlives the call. A null queue
        // config selects the driver defaults. On error, Drop of `port` stops and closes it.
        unsafe {
            check("rte_eth_dev_configure", sys::rte_eth_dev_configure(id, 1, 1, &conf))?;
            check(
                "rte_eth_rx_queue_setup",
                sys::rte_eth_rx_queue_setup(
                    id,
                    0,
                    HARDWARE_RING_DESCRIPTOR_COUNT,
                    0,
                    ptr::null(),
                    pool.0.as_ptr(),
                ),
            )?;
            check(
                "rte_eth_tx_queue_setup",
                sys::rte_eth_tx_queue_setup(id, 0, HARDWARE_RING_DESCRIPTOR_COUNT, 0, ptr::null()),
            )?;
            check("rte_eth_dev_start", sys::rte_eth_dev_start(id))?;
        }
        Ok(port)
    }

    /// Fills the free part of `burst` from RX queue 0. Not a system call.
    #[inline]
    pub fn rx_burst(&mut self, burst: &mut Burst<'p>) {
        let len = burst.0.len();
        // SAFETY: Mbuf is repr(transparent) over a non-null pointer, so the spare part of the
        // array has the layout of `*mut rte_mbuf[BURST - len]`. The driver writes at most that
        // many valid pointers and hands their ownership to us.
        unsafe {
            let spare = burst.0.as_mut_ptr().add(len).cast::<*mut sys::rte_mbuf>();
            let got = sys::rte_eth_rx_burst(self.id, 0, spare, (BURST - len) as u16);
            burst.0.set_len(len + usize::from(got));
        }
    }

    /// Posts the mbufs to TX queue 0. Sent mbufs leave `burst`. Unsent ones stay.
    #[inline]
    pub fn tx_burst(&mut self, burst: &mut Burst<'p>) -> usize {
        // SAFETY: same layout argument as rx_burst. The driver takes ownership of the
        // first `sent` mbufs and frees each one itself after the NIC reports TX completion.
        let sent = usize::from(unsafe {
            sys::rte_eth_tx_burst(
                self.id,
                0,
                burst.0.as_mut_ptr().cast::<*mut sys::rte_mbuf>(),
                burst.0.len() as u16,
            )
        });
        // The NIC owns these now. Forget them, so that Drop does not free them a second time.
        burst.0.drain(..sent).for_each(core::mem::forget);
        sent
    }
}

impl Drop for Port<'_> {
    fn drop(&mut self) {
        // SAFETY: this value owns the port id. Stop and close release the ring buffers to the pool.
        unsafe {
            sys::rte_eth_dev_stop(self.id);
            sys::rte_eth_dev_close(self.id);
        }
    }
}
```

## `ef_vi` shim notes

No listing. Facts that a C shim and a Rust owner over `ef_vi` need.

- Headers: `<etherfabric/base.h>` (`ef_driver_open`), `<etherfabric/pd.h>`, `<etherfabric/memreg.h>`, `<etherfabric/vi.h>`.
- Setup order: `ef_driver_open`, `ef_pd_alloc`, `ef_vi_alloc_from_pd`, `mmap` buffers with `MAP_HUGETLB`, `ef_memreg_alloc`, `ef_vi_receive_init` for each buffer, one `ef_vi_receive_push`. Teardown runs in reverse.
- `ef_eventq_poll`, `ef_vi_receive_init`, `ef_vi_receive_push`, `ef_vi_transmit` are macros over `vi->ops`. `ef_event` is a C union read by `EF_EVENT_TYPE`, `EF_EVENT_RX_RQ_ID`, `EF_EVENT_RX_BYTES`. Wrap each in a plain C function. Declare the functions in an `unsafe extern "C"` block (edition 2024 rule).
- RX test: `EF_EVENT_TYPE(ev) == EF_EVENT_TYPE_RX`. `EF_EVENT_RX_BYTES` includes RX prefix: subtract `ef_vi_receive_prefix_len`. Frame starts after prefix.
- TX completion: on `EF_EVENT_TYPE_TX`, `ef_vi_transmit_unbundle` gives request ids. Repost only those buffers. On `EF_EVENT_TYPE_RX_DISCARD`, repost the buffer.
- Make RX ring hold every buffer (ring of 2048 for 1024 buffers), so a repost never finds it full. IF TX ring is full, THEN repost that buffer to RX.
- This model does not handle `EF_EVENT_TYPE_RX_MULTI` (high-throughput mode) or `EF_EVENT_TYPE_RX_REF` (X3 adapters, other buffer model).
- `ef_vi` offloads TX checksums unless VI has `EF_VI_TX_IP_CSUM_DIS` or `EF_VI_TX_TCPUDP_CSUM_DIS`.
- Rust owner: opaque `#[repr(C)]` struct with `_opaque: [u8; 0]` and `PhantomData<(*mut u8, PhantomPinned)>`, so it is not `Send` and not `Unpin`. Create it on the polling core. One VI per core. Free it in `Drop`.
- Buffer ownership cannot be a type rule: ids come from NIC at run time. Keep it in `// SAFETY:` comments. A debug build can assert on a per-id ownership bitmap.

## Doorbell write notes

No listing. `rte_eth_tx_burst`, `ef_vi_transmit` and the AF_XDP kernel driver ring the doorbell themselves. Write one only for a device with no driver.

Order:

1. Copy payload into DMA buffer with `copy_from_slice`. IF payload is larger than buffer, THEN return an error.
2. `core::sync::atomic::fence(Release)`. It orders cacheable stores between cores only. It emits no instruction on x86_64 and `dmb ish` (inner shareable domain) on aarch64.
3. Device store barrier:
   - x86_64: `core::arch::x86_64::_mm_sfence()`. It orders write-combining stores and is also a compiler barrier. It is a `#[target_feature(enable = "sse")]` fn, so a call from a plain fn needs `unsafe` (error E0133), though SSE is in the x86_64 baseline.
   - aarch64: `core::arch::asm!("dmb oshst", options(nostack, preserves_flags))`, in `unsafe`. The device is in the outer shareable domain. Give no `nomem` option, so the barrier also stops compiler reordering. `__dmb` and `__dsb` are nightly only. IF device must see stores complete, not only ordered, THEN use `dsb st`.
4. `write_volatile` of the value to the doorbell register (`NonNull<u32>`), in `unsafe`. It stops the compiler from deleting the write. It is not atomic and gives no order between threads.

Rules:

- On x86_64, `compiler_fence(Release)` + `_mm_sfence` is enough. `fence(Release)` costs nothing there, so keep it for aarch64.
- Make the constructor that takes the register pointer an `unsafe fn`. Caller promises an aligned, mapped 32-bit register that only this queue writes while it lives.
- Map registers with `memmap2::MmapRaw` over the device file. Keep the map in the same struct as the pointer. Check that the offset is 4-byte aligned and inside the map.
- `volatile` 0.6.1: `VolatilePtr` puts the `unsafe` in one constructor, and each later read and write is safe. Choose it when a device has more than one or two registers.
- `NonNull<u32>` is not `Send`. Build the queue on the hotpath thread. No `unsafe impl Send` is necessary.
- Never use `write_volatile` for sync between threads.
