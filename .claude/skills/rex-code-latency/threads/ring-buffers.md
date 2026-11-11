# Ring buffers

Messages and shared state between cores: SPSC ring, multi-producer queue, sequence lock.

## Symptom

- Threads pass messages through `Mutex<VecDeque>`, `std::sync::mpsc` or blocking `crossbeam-channel` `recv()`.
- SPSC queue gets slower as message rate rises.
- Many cores read numbers and flags that one thread updates. `RwLock` is slow. IF that state holds a `Vec`, `String`, `Box` or map, THEN read `.claude/skills/rex-code-latency/memory/reclamation.md`.
- Reader sees some fields old and some new (torn read).
- Which queue crate? Write the ring myself?

## Confirm

Measure as `.claude/skills/rex-code-latency/measure.md` says.

1. IF hotpath thread calls `futex` (`strace -f -e trace=futex`), THEN lock or blocking channel parks it.
2. IF latency rises with message rate while queue is not full, THEN cause = cross-core index reads. Linux, root, Intel PEBS or AMD IBS: `perf c2c record` + `perf c2c report` shows shared lines.
3. Torn read: writer stores one value in every payload word. Readers on aarch64 assert words agree.

## Rules

| Pattern | Use |
| --- | --- |
| 1 producer, 1 consumer | `rtrb::RingBuffer::<T>::new(capacity)` → `(Producer, Consumer)` |
| Same, slots in own memory (arena, huge page) | `ringbuf` `SharedRb` over `storage::Ref` |
| More than 1 producer or consumer | `crossbeam_queue::ArrayQueue`, CAS per operation |
| 1 writer, many readers, ≤ 8 bytes | one `AtomicU64`, fields packed by hand |
| 1 writer, many readers, larger plain data | sequence lock, atomic payload words |
| 1 writer, many readers, data with pointers | `.claude/skills/rex-code-latency/memory/reclamation.md` |

- [ ] CAUTION: Use `AtomicCell<T>` only for a `T` with no padding bytes. The cell loads `T` as an integer, and a padding byte read as an integer is undefined behavior.

SPSC:

- [ ] Pre-fault slot pages at start: push and pop `capacity` items once.
- [ ] IF `push` returns `Err(PushError::Full(value))`, THEN retry, or drop `value` and count. Never block.
- [ ] Batch with `write_chunk_uninit`, `push_entire_slice`, `read_chunk`: one `Release` store per batch.
- [ ] Hand-write a ring only across processes in shared memory: no crate fits.

Sequence lock:

- [ ] CAUTION: Read and write every payload word with an atomic access. A plain or `read_volatile` copy that overlaps a write is undefined behavior, even when the retry discards it.
- [ ] CAUTION: Put `fence(Release)` after the odd store. Else payload stores can move above it, and a reader accepts a torn read.
- [ ] CAUTION: Allow one writer only, with a writer handle that is not `Clone`. Two writers race on the counter.
- [ ] CAUTION: Use an `AtomicU64` counter. A 32-bit counter can wrap in seconds and pass a torn read.
- [ ] CAUTION: Put no pointer, `Box`, `Vec`, `String` or `dyn` value in the payload. A reader can follow a pointer that the writer freed before the check fails.

Rows run in order:

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
- [ ] Counter and payload in one `CachePadded`. IF payload exceeds 64 B, THEN each extra line adds one miss per read.
- [ ] Rare writes only: each write makes every reader miss.

## Numbers

Typical figures. Measure on your own hardware.

| Operation | Cost |
| --- | --- |
| SPSC push or pop, remote index load each call | about 5–6 ns |
| Sequence lock read, no write in progress | about 1 ns |

## Rust tools

| Crate | Use | Not obvious |
| --- | --- | --- |
| `rtrb` 0.4.1, `default-features = false` | production SPSC | Any capacity. Caches both indices. |
| `ringbuf` 0.6.0, `default-features = false`, `features = ["alloc"]` | `HeapRb`, `StaticRb<T, N>`, `SharedRb` | `SharedRb::from_raw_parts(storage, 0, 0)` is `unsafe`. |
| `crossbeam-queue` 0.3.14, `default-features = false`, `features = ["alloc"]` | `ArrayQueue` | Slower than SPSC. |
| `atomic-memcpy` 0.2.3 | atomic copy of generic `T: bytemuck::Pod` payload | API `unsafe`, copy race-free. |

## Traps

- `RwLock` (std, `parking_lot`): each read writes a shared atomic, so readers bounce its line.
- Indices on separate lines still bounce: each cross-core index read is true sharing.
- `seqlock` 0.2.0: racy `read_volatile` copy, `yield_now()` syscall in reader spin, mutex in writer. Not for hotpath.
- `AtomicCell<T>` with no native atomic for `T` takes a global lock: readers write shared lines. Assert `is_lock_free()` in a `const`. A `u32` + `bool` struct needs `#[repr(C, align(8))]` and a `_pad: [u8; 3]` field.

## Code

- IF you write a sequence lock, THEN read "Sequence lock" in `.claude/skills/rex-code-latency/threads/ring-buffers-code.md`.
- IF you hand-write an SPSC ring, THEN read "Hand-written SPSC ring" in the same file.

## See also

- IF you choose orderings or tests, THEN read `.claude/skills/rex-code-latency/threads/atomics.md`.
- IF you place indices on cache lines, THEN read `.claude/skills/rex-code-latency/cpu/cache-line-layout.md`.
