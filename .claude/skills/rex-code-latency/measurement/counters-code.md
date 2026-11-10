# Counters code

Listings for `.claude/skills/rex-code-latency/measurement/counters.md`. Each compiles as shown. IF you change one, THEN run `cargo check` on each target again.

`Cargo.toml`:

```toml
[dependencies]
quanta = { version = "0.13.0", default-features = false }
thiserror = { version = "2.0.21", default-features = false }

[target.'cfg(target_os = "linux")'.dependencies]
perf-event2 = { version = "0.7.4", default-features = false }   # imported as `perf_event`

[profile.release]
lto = "fat"
codegen-units = 1
panic = "abort"
```

## Bounded and unfenced TSC probe, x86_64

Compile-checked: cargo check, x86_64-unknown-linux-gnu.

- Put it in module `probe`, gated `#[cfg(target_arch = "x86_64")]`. The counter snapshot below imports `crate::probe::lfence`.
- `lfence()` holds the only `unsafe` block. The `const` assert proves sse2 at build time.
- `terminate_profiling_boundary` returns `Err` when the counter goes back: thread moved core or socket.

```rust
use quanta::Clock;

const _: () = assert!(cfg!(target_feature = "sse2"), "lfence needs sse2");

/// Waits until all earlier instructions complete; later ones do not start before it.
#[inline(always)]
pub fn lfence() {
    // SAFETY: sse2 is part of the x86_64 baseline, and the assert above proves it is on.
    unsafe { core::arch::x86_64::_mm_lfence() }
}

#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
    #[error("counter went backward: start {start}, end {end}; thread moved core?")]
    CounterWentBack { start: u64, end: u64 },
}

#[inline(always)]
pub fn capture_bounded_timestamp(clock: &Clock) -> u64 {
    lfence();
    let raw_cycle_count = clock.raw();
    lfence();
    raw_cycle_count
}

/// Only for tight loops that must stay vectorized. Start and end can drift.
#[inline(always)]
pub fn capture_unfenced_timestamp(clock: &Clock) -> u64 {
    clock.raw()
}

pub struct ExecutionLatencyProfiler<'c> {
    clock: &'c Clock,
    initial_cycle_marker: u64,
}

impl<'c> ExecutionLatencyProfiler<'c> {
    #[inline(always)]
    pub fn engage_profiling_boundary(clock: &'c Clock) -> Self {
        Self { clock, initial_cycle_marker: capture_bounded_timestamp(clock) }
    }

    #[inline(always)]
    pub fn terminate_profiling_boundary(&self) -> Result<u64, ProbeError> {
        let final_cycle_marker = capture_bounded_timestamp(self.clock);
        final_cycle_marker
            .checked_sub(self.initial_cycle_marker)
            .ok_or(ProbeError::CounterWentBack {
                start: self.initial_cycle_marker,
                end: final_cycle_marker,
            })
    }
}
```

Use: `let clock = Clock::new();` at start. On hotpath: `let probe = ExecutionLatencyProfiler::engage_profiling_boundary(&clock);`, then the work, then `let ticks = probe.terminate_profiling_boundary()?;`. Push `ticks` raw. Convert on slow path.

## Ordered `CNTVCT_EL0` read, aarch64

Compile-checked: cargo check, aarch64-apple-darwin and aarch64-unknown-linux-gnu. Run on Apple M1: `CNTFRQ_EL0` = 24 MHz.

- Gate with `#[cfg(target_arch = "aarch64")]`.
- Count read has no `nomem`, so it is also a compiler barrier for memory operations. Frequency read has `nomem`: value is fixed at boot.
- Convert on slow path: `ns = Δticks × 1e9 / counter_frequency_hz()`, in `u128`.

```rust
use core::arch::asm;

/// isb before: earlier work completes first. isb after: later work waits for the read.
#[inline(always)]
pub fn capture_bounded_counter() -> u64 {
    let ticks: u64;
    // SAFETY: CNTVCT_EL0 is readable at EL0 because Linux and macOS set
    // CNTKCTL_EL1.EL0VCTEN. isb and mrs touch no memory and no stack.
    unsafe {
        asm!(
            "isb",
            "mrs {t}, cntvct_el0",
            "isb",
            t = out(reg) ticks,
            options(nostack, preserves_flags),
        );
    }
    ticks
}

/// Startup only. Fixed at boot. Armv8.6-A and later fix it at 1 GHz.
pub fn counter_frequency_hz() -> u64 {
    let hz: u64;
    // SAFETY: CNTFRQ_EL0 is readable at EL0 on the same condition as above.
    unsafe {
        asm!("mrs {f}, cntfrq_el0", f = out(reg) hz, options(nomem, nostack, preserves_flags));
    }
    hz
}
```

## Counter snapshot for IPC and L1D misses, Linux x86_64

Compile-checked: cargo check, x86_64-unknown-linux-gnu. Not run.

- Gate with `#[cfg(all(target_os = "linux", target_arch = "x86_64"))]`.
- `new()` fails with `PmuError::Open` IF kernel refuses a counter (`perf_event_paranoid`, `CAP_PERFMON`).
- `capture_hardware_state` fails with `PmuError::NoUserRead` IF a counter is off the PMU. It never returns a stale value.
- Each `read_user()` reads the perf control page, runs `rdpmc`, and retries IF the page sequence changed. No syscall. Its cost is not measured: price it with the TSC probe.
- Deltas use `wrapping_sub`. `read_user()` gives the 64-bit kernel count, not the 48-bit register, so a wrap is not expected.

```rust
use crate::probe::lfence;
use perf_event::events::{Cache, CacheId, CacheOp, CacheResult, Event, Hardware};
use perf_event::{Builder, Sampler};

#[derive(Debug, thiserror::Error)]
pub enum PmuError {
    #[error("perf_event_open failed for {counter}")]
    Open { counter: &'static str, #[source] source: std::io::Error },
    #[error("{0} is not on the PMU now, or user-space rdpmc is off")]
    NoUserRead(&'static str),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TelemetrySnapshot {
    pub unhalted_core_cycles: u64,
    pub instructions_retired: u64,
    pub level_one_cache_misses: u64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PerformanceMetrics {
    pub total_execution_cycles: u64,
    pub instructions_per_cycle: f64,
    pub cache_misses: u64,
}

pub struct HardwarePerformanceAnalyzer {
    cycles: Sampler,
    instructions: Sampler,
    l1d_misses: Sampler,
}

fn open(counter: &'static str, event: impl Event) -> Result<Sampler, PmuError> {
    let err = |source| PmuError::Open { counter, source };
    Builder::new(event)
        .pinned(true)
        .enabled(true)
        .build()
        .map_err(err)?
        .sampled(0)
        .map_err(err)
}

impl HardwarePerformanceAnalyzer {
    /// Startup only. The kernel programs each counter; no MSR writes by hand.
    pub fn new() -> Result<Self, PmuError> {
        let l1d_read_miss =
            Cache { which: CacheId::L1D, operation: CacheOp::READ, result: CacheResult::MISS };
        Ok(Self {
            cycles: open("cycles", Hardware::CPU_CYCLES)?,
            instructions: open("instructions", Hardware::INSTRUCTIONS)?,
            l1d_misses: open("l1d_read_miss", l1d_read_miss)?,
        })
    }

    #[inline(always)]
    pub fn capture_hardware_state(&self) -> Result<TelemetrySnapshot, PmuError> {
        lfence();
        let cycles = self.cycles.read_user().count();
        let instructions = self.instructions.read_user().count();
        let misses = self.l1d_misses.read_user().count();
        lfence();
        Ok(TelemetrySnapshot {
            unhalted_core_cycles: cycles.ok_or(PmuError::NoUserRead("cycles"))?,
            instructions_retired: instructions.ok_or(PmuError::NoUserRead("instructions"))?,
            level_one_cache_misses: misses.ok_or(PmuError::NoUserRead("l1d_read_miss"))?,
        })
    }
}

pub fn compute_execution_metrics(start: &TelemetrySnapshot, end: &TelemetrySnapshot) -> PerformanceMetrics {
    let cycles = end.unhalted_core_cycles.wrapping_sub(start.unhalted_core_cycles);
    let instructions = end.instructions_retired.wrapping_sub(start.instructions_retired);
    PerformanceMetrics {
        total_execution_cycles: cycles,
        instructions_per_cycle: if cycles > 0 { instructions as f64 / cycles as f64 } else { 0.0 },
        cache_misses: end.level_one_cache_misses.wrapping_sub(start.level_one_cache_misses),
    }
}
```

Use: `let pmu = HardwarePerformanceAnalyzer::new()?;` at start. Per region: `let a = pmu.capture_hardware_state()?;`, then the work, then `let b = pmu.capture_hardware_state()?;` and `compute_execution_metrics(&a, &b)`. Keep snapshots raw on hotpath. Compute metrics on slow path.
