# Logging code

Listings for `.claude/skills/rex-code-latency/measurement/logging.md`. Each compiles as shown. IF you change one, THEN run `cargo check` on each target again.

`Cargo.toml`:

```toml
[dependencies]
rtrb = { version = "0.4.1", default-features = false }
zerocopy = { version = "0.8.63", default-features = false, features = ["derive"] }
core_affinity = { version = "0.8.3", default-features = false }
thiserror = { version = "2.0.21", default-features = false, features = ["std"] }
hdrhistogram = { version = "7.6.0", default-features = false }
quanta = { version = "0.13.0", default-features = false }   # hotpath timestamps
```

## Event block and hand-off to a pinned slow path

Compile-checked: cargo check, x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu and aarch64-apple-darwin. Run as a test on aarch64-apple-darwin with the pin check bypassed: capacity 8, 100 pushes, 8 written, 92 dropped.

- `spawn` pre-faults every slot, pins the slow path thread, and fails IF the pin fails. On macOS the pin always fails, so `spawn` returns `Err(PipelineError::Pin)`. Bypass the check only in development tests.
- `dispatch_telemetry_event` never blocks. A full ring counts a drop in a cold fn.
- `shutdown` drops the `Producer`, waits for the drain, and returns written and dropped counts and any write error.
- No `Drop` impl: it cannot drop the `Producer` before the join unless the field is an `Option`, which adds a branch per push. IF the pipeline drops without `shutdown`, THEN the slow path still drains and exits, but nothing waits for it.
- Latency sample: start ticks in `hardware_timestamp`, `end.to_le_bytes()` in payload bytes 0..8. Anchor: ticks in `hardware_timestamp`, wall-clock ns in payload bytes 0..8, probe cost in 8..16. Give each kind its own `event_identifier` and set `payload_length`.
- IF the slow path shares a core, THEN replace `spin_loop()` with a sleep of some µs.

```rust
use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread::{self, JoinHandle};

use core_affinity::CoreId;
use rtrb::{Consumer, Producer, PushError, RingBuffer};
use zerocopy::{FromBytes, FromZeros, Immutable, IntoBytes, KnownLayout};

#[cfg(not(target_endian = "little"))]
compile_error!("the archive format is native little-endian");

#[repr(C, align(64))]
#[derive(Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct TelemetryEventBlock {
    pub hardware_timestamp: u64,
    pub event_identifier: u16,
    pub payload_length: u16,
    pub raw_payload_bytes: [u8; 52],
}

const _: () = assert!(size_of::<TelemetryEventBlock>() == 64);
const _: () = assert!(align_of::<TelemetryEventBlock>() == 64);
const _: () = assert!(core::mem::offset_of!(TelemetryEventBlock, event_identifier) == 8);
const _: () = assert!(core::mem::offset_of!(TelemetryEventBlock, raw_payload_bytes) == 12);

#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("cannot open telemetry archive {path}")]
    Open { path: PathBuf, #[source] source: io::Error },
    #[error("cannot spawn the slow path thread")]
    Spawn(#[source] io::Error),
    #[error("cannot pin the slow path thread to core {0}")]
    Pin(usize),
    #[error("slow path thread panicked")]
    Panicked,
    #[error("slow path write failed")]
    Write(#[source] io::Error),
}

pub struct ShutdownReport {
    pub written_events: u64,
    pub dropped_events: u64,
}

pub struct AsynchronousTelemetryPipeline {
    producer: Producer<TelemetryEventBlock>,
    dropped_events: u64,
    slow_path: JoinHandle<io::Result<u64>>,
}

impl AsynchronousTelemetryPipeline {
    pub fn spawn(path: &Path, capacity: usize, core: CoreId) -> Result<Self, PipelineError> {
        let file = OpenOptions::new().create(true).append(true).open(path)
            .map_err(|source| PipelineError::Open { path: path.to_owned(), source })?;
        let (mut producer, mut consumer) = RingBuffer::new(capacity);
        // Touch every slot now, so the first pass of the hotpath takes no page fault.
        while producer.push(TelemetryEventBlock::new_zeroed()).is_ok() {}
        while consumer.pop().is_ok() {}
        let (pinned_tx, pinned_rx) = mpsc::sync_channel(1);
        let slow_path = thread::Builder::new()
            .name("telemetry-slow-path".into())
            .spawn(move || {
                let pinned = core_affinity::set_for_current(core);
                let _ = pinned_tx.send(pinned);
                if !pinned {
                    return Ok(0);
                }
                drain_to_disk(consumer, file)
            })
            .map_err(PipelineError::Spawn)?;
        // Startup may block. Refuse to run if the slow path shares a core.
        if pinned_rx.recv() != Ok(true) {
            return Err(PipelineError::Pin(core.id));
        }
        Ok(Self { producer, dropped_events: 0, slow_path })
    }

    #[inline]
    pub fn dispatch_telemetry_event(&mut self, event: TelemetryEventBlock) {
        if let Err(PushError::Full(_)) = self.producer.push(event) {
            self.register_dropped_event();
        }
    }

    #[cold]
    #[inline(never)]
    fn register_dropped_event(&mut self) {
        self.dropped_events += 1;
    }

    /// Drops the producer, waits for the slow path to drain the queue, and returns the counts.
    pub fn shutdown(self) -> Result<ShutdownReport, PipelineError> {
        let Self { producer, dropped_events, slow_path } = self;
        drop(producer);
        let written_events = slow_path.join()
            .map_err(|_| PipelineError::Panicked)?
            .map_err(PipelineError::Write)?;
        Ok(ShutdownReport { written_events, dropped_events })
    }
}

fn drain_to_disk(mut consumer: Consumer<TelemetryEventBlock>, file: File) -> io::Result<u64> {
    let mut out = BufWriter::with_capacity(1 << 20, file);
    let mut written = 0u64;
    loop {
        // Read the flag before the slot count. The producer sets it with
        // Release after its last push, so an empty queue now means done.
        let abandoned = consumer.is_abandoned();
        let ready = consumer.slots();
        match consumer.read_chunk(ready) {
            Ok(chunk) if ready > 0 => {
                let (first, second) = chunk.as_slices();
                out.write_all(first.as_bytes())?;
                out.write_all(second.as_bytes())?;
                written += ready as u64;
                chunk.commit_all();
            }
            _ if abandoned => break,
            _ => std::hint::spin_loop(),
        }
    }
    out.into_inner().map_err(io::IntoInnerError::into_error)?.sync_data()?;
    Ok(written)
}
```

## Percentiles from tick pairs

Compile-checked: cargo check, aarch64-apple-darwin and x86_64-unknown-linux-gnu. Run with synthetic pairs on aarch64-apple-darwin.

- 1 ns to 10 ms, 3 significant digits. Memory is fixed after construction. `saturating_record` puts a value above 10 ms in the top bucket and never fails.
- Pairs with end not after start are skipped.
- In process: `to_nanos` = `|s, e| clock.delta_as_nanos(s, e)`.
- For p99.99, add `h.value_at_quantile(0.9999)`. Report `samples` with the dropped count.

```rust
use hdrhistogram::{CreationError, Histogram};

pub struct LatencyReport {
    pub p50_ns: u64,
    pub p99_ns: u64,
    pub p999_ns: u64,
    pub max_ns: u64,
    pub samples: u64,
}

/// `to_nanos` is the clock's own delta, for example `quanta::Clock::delta_as_nanos`.
pub fn latency_report(
    pairs: impl IntoIterator<Item = (u64, u64)>,
    to_nanos: impl Fn(u64, u64) -> u64,
) -> Result<LatencyReport, CreationError> {
    // 1 ns to 10 ms, 3 significant digits. Values above 10 ms clamp to the top bucket.
    let mut h = Histogram::<u64>::new_with_bounds(1, 10_000_000, 3)?;
    for (start, end) in pairs {
        if end > start {
            h.saturating_record(to_nanos(start, end).max(1));
        }
    }
    Ok(LatencyReport {
        p50_ns: h.value_at_quantile(0.50),
        p99_ns: h.value_at_quantile(0.99),
        p999_ns: h.value_at_quantile(0.999),
        max_ns: h.max(),
        samples: h.len(),
    })
}
```

Offline read of the archive into this report:

```
pseudocode
bytes = std::fs::read(path)?                     # safe. Mmap::map only IF too big
IF bytes.len() % 64 != 0, THEN error
FOR raw IN bytes.chunks_exact(64):
    block = TelemetryEventBlock::read_from_bytes(raw)?
    IF block is Anchor, THEN keep (ticks, wall_ns, probe_cost)
    ELSE pairs.push((block.hardware_timestamp, end ticks from payload))
hz = (a1.ticks - a0.ticks) * 1e9 / (a1.wall_ns - a0.wall_ns)            # u128
to_nanos = |s, e| (e - s).saturating_sub(probe_cost) * 1e9 / hz         # u128
report = latency_report(pairs, to_nanos)                                 # + dropped count
```
