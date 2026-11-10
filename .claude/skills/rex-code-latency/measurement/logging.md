# Logging

Record hotpath events with no added latency. Rebuild percentiles offline.

## Symptom

- p99 rises when logging is on.
- `log`, `tracing`, `format!` or `println!` in hotpath code.
- Every sample must be kept for percentiles.
- Records lost at shutdown or in bursts.
- Tick counts must match wall time of packet captures.

## Confirm

- [ ] Measure p99 and p99.9 with logging off and on: `.claude/skills/rex-code-latency/measure.md`. IF difference > noise, THEN logging is the cause.
- [ ] Search hotpath for `format!`, `write!`, `to_string`, `{:?}`, `info!`, `#[instrument]`, `println!`, `File`, `Mutex`.

## Rules

Record:

- [ ] Hotpath writes binary records only: raw integers. No text, no number-to-text.
- [ ] One record = one event block: `#[repr(C, align(64))]`, 8 B timestamp, 2 B event id, 2 B length, 52 B payload. Lock with `const` asserts on `size_of`, `align_of`, `offset_of!`.
- [ ] Derive `zerocopy` `FromBytes, IntoBytes, Immutable, KnownLayout`. Write with `as_bytes()`: no `unsafe` cast.
- [ ] Payload struct fields: `zerocopy::byteorder::little_endian::{U16, U32, U64}`. Alignment 1: no padding.
- [ ] Writer and reader share one module: record types + `#[repr(u16)] enum` of event ids with `TryFrom<u16>`. Layout change breaks both builds.

Hand-off:

- [ ] Hotpath thread owns `rtrb::Producer`, slow path thread owns `Consumer`. Build at start: `RingBuffer::new(capacity)`.
- [ ] Capacity ≥ event rate × longest slow path stall.
- [ ] Fill ring once, drain once, at start: pre-faults every slot.
- [ ] Drop on overflow: IF `push` gives `Err(PushError::Full(_))`, THEN count drop in a `#[cold]` `#[inline(never)]` fn and return. Never wait or retry.
- [ ] Slow path thread pins itself to a housekeeping core, first, inside its closure, and sends result on a channel. IF pin fails, THEN start fails.
- [ ] Drain in batches: `read_chunk(slots())`, `as_bytes()` per slice, `write_all` into `BufWriter::with_capacity(1 << 20, file)`.
- [ ] Idle: `std::hint::spin_loop()` on own core. IF core is shared, THEN `thread::sleep` some µs.
- [ ] Shutdown: drop `Producer`, join. Slow path reads `is_abandoned()` before `slots()`, drains, `sync_data`. Return written count, dropped count, I/O error.

Time anchor:

- [ ] At start and at shutdown, write an anchor event: `SystemTime::now()` next to a bounded probe.
- [ ] Offline: `hz = (c1 − c0) × 1e9 / (t1 − t0)`. Wall time of tick `c` = `t0 + (c − c0) × 1e9 / hz`. `u128` integer math. `checked_sub` for ticks before anchor.
- [ ] Linux PTP: with `phc2sys`, `CLOCK_REALTIME` follows PTP card, so `SystemTime::now()` = PTP time.

Offline:

- [ ] Analyze off production host. Read with `std::fs::read`. `memmap2::Mmap::map` only for archives too big for memory.
- [ ] IF length is not a multiple of 64, THEN error. Walk `chunks_exact(64)` + `read_from_bytes`.
- [ ] Report p50, p99, p99.9, p99.99, max, count, dropped. Load test at fixed send rate: `record_correct(value, expected_interval)` adds samples that a stall hid.
- [ ] Join each slow record to counters and packet captures by wall time.

Usual crates:

- [ ] Never call `log` or `tracing` on hotpath. `log` backends get `fmt::Arguments` that borrow caller stack: they format on caller. `tracing-subscriber` `fmt` also formats on caller. `env_logger` writes each line under a lock.
- [ ] `tracing_appender::non_blocking` moves only the write. Caller still formats + allocates each line.
- [ ] IF a macro must stay in hotpath code, THEN enable `release_max_level_off` on `log` or `tracing`.

## Numbers

Typical figures. Measure on your own hardware.

| Item | Value |
| --- | --- |
| Integer to text | about 20 divisions per large number, dozens of cycles each |
| Format code in L1i | thousands of bytes |
| Binary record write | 1 to 2 ns |
| SPSC push | about 5 ns |
| Disk write stall | hundreds of µs |
| p50, deferred-format logger / `tracing` | 0.08–0.46 µs / 0.99–1.62 µs |
| Example tail to investigate | p50 12 ns, p99.9 400 ns |

## Rust tools

All `default-features = false`.

- `rtrb` 0.4.1: wait-free SPSC ring buffer, chunk API, abandoned flag.
- `zerocopy` 0.8.63, `features = ["derive"]`: checked byte views, byte-order fields.
- `core_affinity` 0.8.3: pin slow path thread.
- `hdrhistogram` 7.6.0: fixed-memory percentiles.
- `memmap2` 0.9.12: huge archives only.
- `inqjet` 1.2.0: deferred-format text logger. Warm paths only.
- `thiserror` 2.0.21, `features = ["std"]`: `PathBuf` in messages.

## Traps

- Reference to a `#[repr(C, packed)]` field = error E0793.
- `vec![0; n]` and `with_capacity` map pages lazily: first write faults on hotpath.
- `Producer` is `Send`, not `Sync`. `push` takes `&mut self`: compiler keeps one producer.
- Push copies 64 B. `write_chunk_uninit(1)` builds block in slot.
- Stop flag + exit loses queued records. Drop `Producer`: its `Release` pairs with `Acquire` in `is_abandoned()`.
- `set_for_current` returns `false` on macOS. Test the `bool`.
- Rust `File` has no buffer: one `write` syscall per call.
- CAUTION: `Mmap::map` is `unsafe`. Another process can write or truncate a mapped file: undefined behavior or SIGBUS.

## Code

In `.claude/skills/rex-code-latency/measurement/logging-code.md`:

- Event block and hand-off to a pinned slow path: IF you record from hotpath.
- Percentiles from tick pairs, offline read: IF you report the tail.

## See also

- IF you need the bounded probe, THEN read `.claude/skills/rex-code-latency/measurement/counters.md`.
- IF you write the SPSC ring buffer yourself, THEN read `.claude/skills/rex-code-latency/threads/ring-buffers.md`.
- IF building a record allocates, THEN read `.claude/skills/rex-code-latency/memory/zero-alloc.md`.
