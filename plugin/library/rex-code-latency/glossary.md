# Glossary

One name for each term. Every file of this skill uses these names.

## General

- **hotpath**: code from stimulus to reaction: packet in, order out.
- **slow path**: all other work: logging, telemetry, disk, config.
- **isolated core**: core out of scheduler, for pinned hotpath threads only.
- **housekeeping core**: core for OS work, IRQs, slow path threads.
- **pin**: fix a thread to one core. Not an epoch pin.

## OS and pages

- **scheduling tick**: timer interrupt on each core, 100 to 1000 per second.
- **NMI**: interrupt that software cannot mask: watchdog, profiler.
- **SMI**: firmware interrupt that the OS cannot see or mask.
- **IPI**: interrupt that one core sends to another: reschedule, TLB shootdown.
- **C-state**: CPU idle power state. Deep state wakes slowly.
- **SMT sibling**: other logical core on same physical core. Shares L1, L2.
- **first touch**: Linux puts a page on NUMA node of its first writer.
- **vDSO**: kernel data in user space: `clock_gettime` with no kernel entry.
- **pre-fault**: touch every page at start, so no page fault reaches hotpath.
- **THP**: Transparent Huge Pages. Background merges that stall threads.
- **gap scan**: back-to-back clock reads for 10 s or more. Fails on any gap above threshold. Start gate.
- **jitter detector**: short loop of paired clock reads. Probe floor and quick check, not a start gate.

## CPU

- **LLC**: last-level cache, shared by cores, usually L3.
- **false sharing**: independent variables on one line. One core writes, another core reads or writes.
- **true sharing**: cores read and write one variable on purpose. Same line traffic as false sharing.
- **line bouncing**: line ownership moves between cores on each write.
- **line pair**: two adjacent 64-byte lines, 128-byte aligned. x86_64 prefetcher can fetch both.
- **IPC**: instructions retired per unhalted cycle. Below 1.0 = stalled pipeline.
- **PMU**: per-core hardware counters: cycles, instructions, misses.
- **PEBS / IBS**: Intel / AMD precise load sampling.

## Memory

- **ABA problem**: CAS sees same index or address after its node was recycled.
- **tag**: counter beside an index, bumped on each change, so stale CAS fails.
- **hazard pointer**: per-core slot where reader publishes what it reads.
- **retired list**: writer's private list of removed nodes that wait to be freed.
- **read section**: span in which a reader holds a protected reference.
- **grace period**: time until every reader active at a swap leaves.
- **RCU**: writer copies, swaps, retires old version. Readers see immutable versions.
- **epoch pin**: `crossbeam_epoch::pin()`: enter read section. Garbage frees two epochs later.
- **debt slot**: `arc-swap` per-thread slot: a pointer, not a count.

## Threads

- **happens-before**: order the memory model guarantees: program order, plus a `Release` store read by an `Acquire` load.
- **store buffer**: CPU queue of writes that other cores cannot see yet.
- **load buffering**: two threads each load, then store. Each load sees the store of the other thread. loom misses it.
- **SPSC ring buffer**: fixed array queue: one producer thread, one consumer thread.
- **sequence lock**: counter plus payload. Odd = write in progress. Readers retry on change.
- **LSE**: Armv8.1 single-instruction atomics (`cas`, `ldadd`).
- **LSE2**: Armv8.4: aligned 128-bit `ldp` / `stp` are atomic.

## Network

- **kernel stamp gap**: time from `SO_TIMESTAMPNS` kernel stamp to handler entry.
- **socket ladder**: kernel socket tuning in order: spin, `SO_BUSY_POLL`, `io_uring` SQPOLL.
- **reactor**: async runtime part that waits in `epoll_wait` and wakes tasks.
- **AF_XDP**: Linux socket for raw frames. UMEM = frame memory it shares with kernel. Fill ring gives kernel free frames.
- **DPDK**: user-space packet framework. EAL initializes it and pins each lcore (polling thread).
- **mbuf / mempool**: DPDK packet buffer / its fixed pool.
- **bifurcated driver**: DPDK driver that keeps the kernel interface (`mlx5`).
- **sys crate**: crate of raw FFI bindings to one C library.
- **C shim**: small C file that wraps macros and inline functions as real functions, for `bindgen`.
- **OpenOnload / ef_vi**: Solarflare socket-compatible bypass / its raw frame API.
- **Pipeline model**: poll core feeds worker cores over rings. Adds jitter.
- **Flow Steering**: NIC rule that sends one flow to ring of one core.
- **RSS**: Receive Side Scaling: NIC hashes each flow to one of its RX queues.
- **IGMP**: protocol that joins a host to a multicast group.
- **DDIO**: NIC DMA writes go to L3, max 2 ways.
- **MMIO**: load or store to a device register mapped into memory.
- **doorbell**: MMIO register write that tells NIC new work is ready.
- **NIC ring**: descriptor ring that NIC and software share. RX inbound, TX outbound.
- **TX completion**: NIC signal that it finished reading a TX buffer.

## Measurement

- **TSC**: x86 per-core counter, read with `rdtsc`. Ticks at a fixed rate, not core clock cycles.
- **invariant TSC**: TSC that ticks at fixed rate in every power state.
- **bounded probe**: TSC read with `lfence` before and after it.
- **probe cost**: median duration of empty probed region. Subtract from each sample.
- **`rdpmc`**: instruction that reads a PMU counter from user code.
- **`CNTVCT_EL0` / `CNTFRQ_EL0`**: aarch64 fixed-rate counter / its tick rate.
- **anchor**: wall-clock time read next to bounded probe, at start and shutdown.
- **PTP**: IEEE 1588 network clock sync.
