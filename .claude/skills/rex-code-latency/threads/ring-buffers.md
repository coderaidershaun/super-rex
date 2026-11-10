# Ring buffers

Lock-free messages and shared state between cores: SPSC ring, multi-producer queue, sequence lock.

## Symptom

- Threads pass messages through `Mutex<VecDeque>`, `std::sync::mpsc` or blocking `crossbeam-channel` `recv()`.
- SPSC queue gets slower as message rate rises.
- Many cores read state that one thread updates. `RwLock` is slow.
- Reader sees some fields old and some new (torn read).
- Which queue crate? Write the ring myself?

## Confirm

Method: `.claude/skills/rex-code-latency/measure.md`.

1. IF hotpath thread calls `futex` (`strace -f -e trace=futex`), THEN a lock or blocking channel parks it.
2. IF latency rises with message rate while the queue is not full, THEN cause = cross-core index reads. Linux: `perf c2c record` + `perf c2c report` shows shared lines.
3. Torn read: writer stores one value in every payload word. Readers on aarch64 assert the words agree.

## Rules

| Pattern | Use |
| --- | --- |
| 1 producer, 1 consumer | `rtrb::RingBuffer::<T>::new(capacity)` → `(Producer, Consumer)` |
| Same, slots in your own memory (arena, huge page) | `ringbuf` `SharedRb` over `storage::Ref` |
| More than 1 producer or consumer | `crossbeam_queue::ArrayQueue`, CAS per operation |
| 1 writer, many readers, ≤ 8 bytes | one `AtomicU64`, or `AtomicCell<T>` |
| 1 writer, many readers, larger plain data | sequence lock, atomic payload words |
| 1 writer, many readers, data with pointers | not a sequence lock |

SPSC:

- [ ] Pre-fault slot pages at startup: push and pop `capacity` items once.
- [ ] IF `push` returns `Err(PushError::Full(value))`, THEN spin and retry, or drop `value` and count. Never block.
- [ ] Batch with `write_chunk_uninit`, `push_entire_slice`, `read_chunk`: one `Release` store per batch.
- [ ] Hand-write a ring only when no crate fits: across processes in shared memory.

Sequence lock:

- [ ] CAUTION: Read and write every payload word with an atomic access. A plain or `read_volatile` copy that overlaps a write is undefined behavior, even when the retry discards it.
- [ ] CAUTION: Put `fence(Release)` after the odd store. Else payload stores can move above it, and a reader accepts a torn read.
- [ ] CAUTION: Allow one writer only, with a writer handle that is not `Clone`. Two writers race on the counter.
- [ ] CAUTION: Use an `AtomicU64` counter. A 32-bit counter can wrap in seconds and pass a torn read.
- [ ] CAUTION: Put no pointer, `Box`, `Vec`, `String` or `dyn` value in the payload. A reader can follow a pointer that the writer freed before the check fails.

Each side runs its rows in order:

| Who | Access | Ordering |
| --- | --- | --- |
| writer | counter `store(s + 1)`, odd | `Relaxed` |
| writer | `fence` | `Release` |
| writer | each payload word `store` | `Relaxed` |
| writer | counter `store(s + 2)`, even | `Release` |
| reader | `s1 = counter.load()`. IF odd, THEN spin, retry | `Acquire` |
| reader | each payload word `load` into locals | `Relaxed` |
| reader | `fence` | `Acquire` |
| reader | `s2 = counter.load()`. IF `s1 != s2`, THEN retry | `Relaxed` |

- [ ] Payload decodes from any bits: `f64::from_bits`, integers, `bool` from one bit test.
- [ ] Counter and payload in one `CachePadded`, `const` assert `size_of <= 64`.
- [ ] Rare writes only: each write makes every reader miss.
- [ ] `AtomicCell<T>`: `#[repr(C, align(8))]` on `T`, and `const _: () = assert!(AtomicCell::<T>::is_lock_free());`.

## Numbers

Typical figures. Measure on your own hardware.

| Operation | Cost |
| --- | --- |
| `%` by run-time value, `&` mask | dozens of cycles, 1 cycle |
| SPSC push or pop, remote index load each call | about 5–6 ns |
| Sequence lock read, no write in progress | about 1 ns |

## Rust tools

| Crate | Use | Not obvious |
| --- | --- | --- |
| `rtrb` 0.4.1, `default-features = false` | production SPSC | Any capacity. Caches both indices. |
| `ringbuf` 0.6.0, `default-features = false`, `features = ["alloc"]` | `HeapRb`, `StaticRb<T, N>`, `SharedRb` | `no_std`. `SharedRb::from_raw_parts(storage, 0, 0)` is `unsafe`. |
| `crossbeam-queue` 0.3.14, `default-features = false`, `features = ["alloc"]` | `ArrayQueue`, bounded MPMC | Slower than SPSC. |
| `atomic-memcpy` 0.2.3 | atomic copy of a generic payload | API `unsafe`, copy race-free. Bound `T: bytemuck::Pod`. |

## Traps

- `std::sync::RwLock`, `parking_lot::RwLock`: each read increments a shared atomic, so readers bounce its line.
- Indices on separate lines still bounce: cross-core index reads are true sharing, same cost as false sharing.
- `seqlock` 0.2.0: racy `read_volatile` copy, `thread::yield_now()` (a syscall) in reader spin, `parking_lot` mutex in writer. Not for hotpath.
- `AtomicCell<T>` with no native atomic for `T` falls back to a global table of 67 locks, and readers write shared lines. A `u32` + `bool` struct needs `align(8)`.

## Code

- IF you write a sequence lock, THEN read "Sequence lock" in `.claude/skills/rex-code-latency/threads/ring-buffers-code.md`.
- IF you hand-write an SPSC ring, THEN read "Hand-written SPSC ring" in the same file.

## See also

- IF you choose orderings or tests, THEN read `.claude/skills/rex-code-latency/threads/atomics.md`.
- IF you place indices on cache lines, THEN read `.claude/skills/rex-code-latency/cpu/cache-line-layout.md`.
- IF payload holds pointers, THEN read `.claude/skills/rex-code-latency/memory/reclamation.md`.
- IF the ring carries telemetry, THEN read `.claude/skills/rex-code-latency/measurement/logging.md`.
