# Crates

Every crate this skill names.

Key: off = `default-features = false`. none = no feature list given. Topic = file name in its area folder.

| Crate | Version | Features | For | Topic |
| --- | --- | --- | --- | --- |
| `allocator-api2` | 0.2.21, not 0.4 | off, `alloc` | Stable `Allocator` trait | zero-alloc |
| `arc-swap`, `left-right` | 1.9.5, 0.11.8 | off | RCU read with `Cache`. Two copies, bounded memory | reclamation |
| `assert_no_alloc` | 1.1.2 | off: stays on in release | Abort on allocation | zero-alloc |
| `atomic-memcpy` | 0.2.3 | none | Generic sequence lock payload | ring-buffers |
| `basedrop` | 0.1.3 | none | Drops on collector thread | zero-alloc |
| `bindgen`, `cc`, `pkg-config` | 0.73.2, 1.11.1, 0.3.34 | `bindgen`: off, `runtime` | DPDK `sys` crate | kernel-bypass |
| `bitfield-struct`, `bitflags` | 0.13.0, 2.13.2 | none | Bit fields, flag sets | cache-line-layout |
| `bitvec` | 1.1.1 | none | Flags, 512 per line | cache-hierarchy |
| `bumpalo`, `bump-scope` | 3.20.3, 2.4.1 | `bumpalo`: off, `collections` | Per-tick arena. Arena on own allocator | zero-alloc |
| `bytemuck` | 1.25.2 | off | `Pod` casts | huge-pages, cache-line-layout |
| `core_affinity` | 0.8.3 | off | One-call pin, returns `bool` | cpu-isolation, logging |
| `criterion`, `divan` | 0.8.2, 0.1.21 | off | Benchmarks, probe cost | counters |
| `crossbeam-epoch`, `crossbeam-skiplist`, `haphazard` | 0.9.21, 0.1.3, 0.1.8 | off, `std` | Epochs, lock-free map, hazard pointers | reclamation |
| `crossbeam-utils` | 0.8.23 | off, never `std` | `CachePadded`, `Backoff`, `AtomicCell` | cache-line-layout, atomics |
| `dhat` | 0.3.3 | off, dev | Allocation call stacks | zero-alloc |
| `enum_dispatch` | 0.3.13 | none | `match` dispatch | branching |
| `foldhash` | 0.2.0 | none | ID map hasher, trusted keys | cache-hierarchy |
| `hashbrown` | 0.17.1 | off, `allocator-api2`, `default-hasher` | Map on pool | zero-alloc |
| `hdrhistogram` | 7.6.0 | off | Percentiles | jitter, logging |
| `heapless`, `arrayvec` | 0.9.3, 0.7.8 | off | Fixed-capacity `Vec` | zero-alloc, kernel-bypass |
| `inqjet` | 1.2.0 | none | Text logger, warm path | logging |
| `loom` | 0.7.3 | off, under `cfg(loom)` | Model checker | atomics |
| `memmap2` | 0.9.12 | off | Anonymous map, `huge`, `populate` | huge-pages, zero-alloc |
| `mio`, `tokio` | 1.2.4, 1.54.1 | `mio`: off, `os-poll`, `net` | Control plane only | sockets |
| `nix` | 0.31.3 | off, `sched`, `mman`, `process` | Pin, `mlockall`, `prctl` | cpu-isolation, huge-pages |
| `perf-event2` | 0.7.4 | off, Linux | Counters, import as `perf_event` | counters |
| `portable-atomic` | 1.15.0 | `float` for `AtomicF64` | `AtomicU128`, `AtomicF64` | atomics |
| `procfs`, `mimalloc` | 0.18.0, 0.1.52 | default | `HugePages_Free`. Heap on huge pages | huge-pages |
| `quanta`, `raw-cpuid` | 0.13.0, 11.6.0 | `quanta`: off | TSC clock. Invariant TSC check | counters, jitter |
| `ringbuf`, `crossbeam-queue` | 0.6.0, 0.3.14 | off, `alloc` | SPSC in own memory. `ArrayQueue` | ring-buffers |
| `rtrb` | 0.4.1 | off | SPSC ring buffer | ring-buffers, logging |
| `seize`, `papaya` | 0.5.1, 0.2.5 | none | Bounded memory, stalled reader | reclamation |
| `slab`, `slotmap` | 0.4.12, 1.1.1 | none | Pools. `slab` grows when full | zero-alloc |
| `smoltcp`, `volatile` | 0.14.0, 0.6.1 | none | User-space TCP/IP. MMIO registers | kernel-bypass |
| `soa_derive` | 0.14.0 | off | SoA derive | cache-hierarchy |
| `socket2`, `io-uring` | 0.6.5, 0.7.16 | off. `socket2`: `all`. `io-uring`: Linux | Busy poll. SQPOLL | sockets |
| `static_assertions` | 1.1.0 | none | `assert_impl_all!` | cache-line-layout |
| `subtle` | 2.6.1 | none | Constant-time select | branching |
| `thiserror` | 2.0.21 | off. `std` only for `PathBuf` | Typed errors | each code file |
| `thread-priority` | 3.1.1 | default | `SCHED_FIFO` | cpu-isolation |
| `xsk-rs` | 0.11.0 | off, Linux | AF_XDP, C build | kernel-bypass |
| `xdpilone`, `xdp`, `aya` | 1.3.0, 0.8.0, 0.16.0 | none | AF_XDP, no C build | kernel-bypass |
| `zerocopy` | 0.8.63 | off, `derive` | Byte views, padding check | cache-line-layout, logging |

Named only as traps: `minstant` 0.1.7, `seqlock` 0.2.0, `parking_lot`, `tikv-jemallocator`, `smallvec`, `anyhow`, `log`, `tracing`, `tracing-subscriber`, `tracing-appender`, `env_logger`, `crossbeam-channel`, `async-std`, `smol`, `tokio-uring`, `rustix`.

- [ ] CAUTION: Keep `crossbeam-utils` defaults off in every crate of the build. Cargo unifies features: one `std` and `Backoff::snooze()` yields to the OS. Check: `cargo tree -e features -i crossbeam-utils`.
- [ ] `hdrhistogram` `new_with_bounds(1, 10_000_000, 3)` = 1 ns to 10 ms. IF stalls above 10 ms can happen (major fault, THP), THEN raise high bound: `10_000_000_000` = 10 s.

## Tools, not crates

- `cargo-show-asm` 0.2.64: `cargo asm`, read emitted assembly.
- `cargo-pgo` 0.3.0 + `llvm-tools-preview`: profile-guided build.
- Miri (`rustup +nightly component add miri`): UB in `unsafe` code and atomics.
- `perf stat`: counts. `perf c2c`: false sharing proof.
- `perf record`, `samply`, `cargo flamegraph`: offline only, never for tail.
- `strace -f -e trace=futex`: lock that parks a thread.
- `ethtool`, `irqbalance`: NIC coalescing, IRQ routing.
- `phc2sys`: host clock follows PTP.
- `heaptrack`, `valgrind --tool=dhat`, `bpftrace`: allocations of whole binary.
- `onload`: OpenOnload, no code change.
