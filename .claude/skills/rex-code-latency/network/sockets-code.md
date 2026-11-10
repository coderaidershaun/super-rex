# Kernel socket code

Two listings for steps 1 to 3 of the socket ladder in `.claude/skills/rex-code-latency/network/sockets.md`.
Both call `evaluate_trading_strategy(&[u8])`. Replace it with your handler.

```toml
[dependencies]
socket2 = { version = "0.6.5", default-features = false, features = ["all"] } # "all" gives set_busy_poll
thiserror = { version = "2.0.21", default-features = false }

[target.'cfg(target_os = "linux")'.dependencies]
io-uring = { version = "0.7.16", default-features = false }
```

## Busy-poll UDP socket with `SO_BUSY_POLL`

Compile-checked: cargo check, x86_64-unknown-linux-gnu. Not run.

- Removes passive wait. Each `recv` still pays syscall, kernel stack and copy.
- `busy_poll_us` above `net.core.busy_read` needs `CAP_NET_ADMIN`. Without it, `set_busy_poll` returns `EPERM`.
- Kernel caps the `8 << 20` receive buffer at `net.core.rmem_max` with no error. Read value back with `recv_buffer_size()`.
- No `unsafe`.

```rust
//! Busy-poll receive on a kernel UDP socket.
#![cfg(target_os = "linux")]
use std::hint::spin_loop;
use std::io;
use std::net::{SocketAddr, UdpSocket};
use socket2::{Domain, Protocol, Socket, Type};

#[derive(Debug, thiserror::Error)]
pub enum BusyPollError {
    #[error("socket setup failed")]
    Setup(#[source] io::Error),
    #[error("receive failed")]
    Recv(#[source] io::Error),
}

pub fn open_busy_poll_socket(bind: SocketAddr, busy_poll_us: u32) -> Result<UdpSocket, BusyPollError> {
    let socket = Socket::new(Domain::for_address(bind), Type::DGRAM, Some(Protocol::UDP))
        .map_err(BusyPollError::Setup)?;
    socket.set_reuse_address(true).map_err(BusyPollError::Setup)?;
    socket.set_recv_buffer_size(8 << 20).map_err(BusyPollError::Setup)?;
    // The kernel polls the NIC queue inside recv, so no packet waits for an interrupt.
    // A value above net.core.busy_read needs CAP_NET_ADMIN.
    socket.set_busy_poll(busy_poll_us).map_err(BusyPollError::Setup)?;
    socket.set_nonblocking(true).map_err(BusyPollError::Setup)?;
    socket.bind(&bind.into()).map_err(BusyPollError::Setup)?;
    Ok(socket.into())
}

/// Run this on an isolated, pinned core. It never sleeps, and each miss costs one syscall.
pub fn spin_receive(socket: &UdpSocket) -> Result<(), BusyPollError> {
    let mut buffer = [0u8; 2048];
    loop {
        match socket.recv(&mut buffer) {
            Ok(bytes_read) => evaluate_trading_strategy(&buffer[..bytes_read]),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => spin_loop(),
            Err(e) => return Err(BusyPollError::Recv(e)),
        }
    }
}
```

## Receive with `io_uring` SQPOLL

Compile-checked: cargo check, x86_64-unknown-linux-gnu. Not run.

- Kernel poll thread on `kernel_poll_cpu` reads each submission. Hotpath thread makes no syscall while poll thread is awake.
- `setup_sqpoll(2_000)`: poll thread sleeps after 2 s with no work. Then `submit()` makes a syscall to wake it.
- Kernel stack and copy into `buffer` still run, on poll thread core.
- Linux 5.11+. Without privileges: Linux 5.13+.
- One `unsafe` block: kernel writes `buffer` after `push`, outside borrow checker.

```rust
//! Receive on a kernel UDP socket with no syscall on the hot thread.
#![cfg(target_os = "linux")]
use std::hint::spin_loop;
use std::io;
use std::net::UdpSocket;
use std::os::fd::AsRawFd;
use io_uring::{opcode, types, IoUring};

#[derive(Debug, thiserror::Error)]
pub enum UringError {
    #[error("io_uring setup failed")]
    Setup(#[source] io::Error),
    #[error("submission queue is full")]
    QueueFull,
    #[error("submit failed")]
    Submit(#[source] io::Error),
    #[error("receive failed")]
    Recv(#[source] io::Error),
}

pub fn build_ring(kernel_poll_cpu: u32) -> Result<IoUring, UringError> {
    IoUring::builder()
        .setup_sqpoll(2_000) // kernel thread spins 2 s with no work, then sleeps
        .setup_sqpoll_cpu(kernel_poll_cpu) // pin it to its own isolated core
        .build(256)
        .map_err(UringError::Setup)
}

pub fn spin_receive(ring: &mut IoUring, socket: &UdpSocket) -> Result<(), UringError> {
    let mut buffer = [0u8; 2048];
    loop {
        let recv = opcode::Recv::new(
            types::Fd(socket.as_raw_fd()),
            buffer.as_mut_ptr(),
            buffer.len() as u32,
        )
        .build()
        .user_data(1);
        // SAFETY: `buffer` lives on this frame for the whole loop, and no code
        // reads or writes it until the completion for this entry is taken below.
        unsafe { ring.submission().push(&recv) }.map_err(|_| UringError::QueueFull)?;
        // With SQPOLL this enters the kernel only if the poll thread went to sleep.
        ring.submit().map_err(UringError::Submit)?;

        let result = loop {
            if let Some(cqe) = ring.completion().next() {
                break cqe.result();
            }
            spin_loop();
        };
        let bytes_read = usize::try_from(result)
            .map_err(|_| UringError::Recv(io::Error::from_raw_os_error(-result)))?;
        evaluate_trading_strategy(&buffer[..bytes_read]);
    }
}
```
