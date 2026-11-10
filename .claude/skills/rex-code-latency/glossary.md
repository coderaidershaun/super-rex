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
- **residual tick**: small tick that stays on a `nohz_full` core.
- **RCU callback**: deferred kernel free work. `rcu_nocbs` moves it off isolated cores.
- **NMI**: interrupt that software cannot mask: watchdog, profiler.
- **C-state**: CPU idle power state. Deep state wakes slowly.
- **SMT sibling**: other logical core on same physical core. Shares L1, L2.
- **first touch**: Linux puts a page on NUMA node of its first writer.
- **vDSO**: kernel data in user space: `clock_gettime` with no kernel entry.
- **pre-fault**: touch every page at start, so no page fault reaches hotpath.
- **TLB reach**: memory that TLB maps: entries × page size.
- **THP**: Transparent Huge Pages. Background merges that stall threads.
- **jitter detector**: startup loop of back-to-back TSC reads. Fails on a spike.

## CPU

- **LLC**: last-level cache, shared by cores, usually L3.
- **dependent load**: load whose address comes from previous load. Not prefetched.
- **AoS / SoA**: array of structs / struct of arrays, one array per field.
- **false sharing**: independent variables on one line, written by different cores.
- **true sharing**: several cores write one variable on purpose.
- **cache line bouncing**: line ownership moves between cores on each access.
- **HITM**: load that hits a line that another core holds modified.
- **hidden padding**: compiler-inserted bytes. Uninitialized: reading them is UB.
- **predication**: comparison result used as 0 or 1 multiplier. No branch.
- **select**: `cmov` (x86_64) or `csel` (aarch64): picks one of two values, no jump.
- **IPC**: instructions retired per unhalted cycle. Below 1.0 = stalled pipeline.

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

- **store buffer**: CPU queue of writes that other cores cannot see yet.
- **torn read**: copy with some bytes old and some new.
- **load buffering**: load sees a store later in program order. loom misses it.
- **SPSC ring buffer**: fixed array queue: one producer thread, one consumer thread.
- **cached index**: private copy of other side's index, refreshed only on full or empty.
- **sequence lock**: counter plus payload. Odd = write in progress. Readers retry on change.
- **LSE**: ARMv8.1 single-instruction atomics (`cas`, `ldadd`).

## Network

- **kernel stamp gap**: time from `SO_TIMESTAMPNS` kernel stamp to handler entry.
- **socket ladder**: kernel socket tuning in order: spin, `SO_BUSY_POLL`, `io_uring` SQPOLL.
- **busy poll**: each `recv` polls NIC queue in calling thread. No interrupt wait.
- **SQPOLL**: `io_uring` kernel thread polls submission ring: no syscall per receive.
- **AF_XDP**: Linux socket for raw frames. UMEM = frame memory it shares with kernel.
- **DPDK**: user-space packet framework. EAL initializes it and pins each lcore thread.
- **mbuf / mempool**: DPDK packet buffer / its fixed pool.
- **OpenOnload / ef_vi**: Solarflare socket-compatible bypass / its raw frame API.
- **Run to Completion**: one isolated core polls, parses, decides, sends. No handoff.
- **Pipeline model**: poll core feeds worker cores over rings. Adds jitter.
- **Flow Steering**: NIC rule that sends one flow to ring of one core.
- **DDIO**: NIC DMA writes go to L3, max 2 ways.
- **doorbell**: MMIO register write that tells NIC new work is ready.
- **NIC ring**: descriptor ring that NIC and software share. RX inbound, TX outbound.
- **TX completion**: NIC signal that it finished reading a TX buffer.

## Measurement

- **TSC**: x86 per-core cycle counter, read with `rdtsc`.
- **invariant TSC**: TSC that ticks at fixed rate in every power state.
- **bounded probe**: TSC read with `lfence` before and after it.
- **unfenced probe**: bare TSC read. Only in tight vectorizable loops.
- **serialization**: earlier instructions retire before timing read. `rdtsc` is not serializing.
- **probe cost**: median duration of empty probed region. Subtract from each sample.
- **noise**: largest change of one percentile between two runs of one build.
- **`rdpmc`**: instruction that reads a PMU counter from user code. Needs CR4.PCE.
- **`CNTVCT_EL0` / `CNTFRQ_EL0`**: aarch64 fixed-rate counter / its tick rate.
- **event block**: 64-byte binary record: timestamp, id, length, payload. No text.
- **drop on overflow**: IF queue is full, THEN discard record and count it.
- **anchor**: wall-clock time read next to bounded probe, at start and shutdown.
- **PTP**: IEEE 1588 network clock sync.
