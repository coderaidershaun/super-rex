# CPU isolation code

Startup checks for the pinned core and for the boot settings. Run each on the pinned thread, before the network connects. Each returns `Err`: carry it to `main` with `?`, so the process exits.

```toml
[dependencies]
thiserror = { version = "2.0.21", default-features = false }
quanta = { version = "0.13.0", default-features = false }
```

## Jitter detector: probe floor and quick check

Compile-checked: cargo check and clippy, x86_64-unknown-linux-gnu. Not run.

```rust
#![cfg(target_arch = "x86_64")]
use std::sync::atomic::{Ordering::SeqCst, compiler_fence};

pub const CALIBRATION_ITERATIONS: usize = 1_000_000;
pub const MAXIMUM_ACCEPTABLE_CYCLES: u64 = 150;

#[derive(Debug, thiserror::Error)]
#[error("jitter detected: max {maximum_cycles} cycles > {MAXIMUM_ACCEPTABLE_CYCLES} (median {median_cycles})")]
pub struct JitterError {
    pub median_cycles: u64,
    pub maximum_cycles: u64,
}

#[derive(Debug)]
pub struct JitterReport {
    pub median_cycles: u64,
    pub maximum_cycles: u64,
}

pub fn calibrate(clock: &quanta::Clock) -> Result<JitterReport, JitterError> {
    let mut samples: Vec<u64> = Vec::with_capacity(CALIBRATION_ITERATIONS);
    let _ = clock.raw(); // One read first, so the loop starts with warm instruction cache.

    for _ in 0..CALIBRATION_ITERATIONS {
        let start = clock.raw();
        compiler_fence(SeqCst); // Stops the compiler from moving the reads. It emits no CPU instruction.
        let end = clock.raw();
        samples.push(end.wrapping_sub(start));
    }

    samples.sort_unstable();
    let median_cycles = samples[CALIBRATION_ITERATIONS / 2];
    let maximum_cycles = samples[CALIBRATION_ITERATIONS - 1];

    if maximum_cycles > MAXIMUM_ACCEPTABLE_CYCLES {
        return Err(JitterError { median_cycles, maximum_cycles });
    }
    Ok(JitterReport { median_cycles, maximum_cycles })
}

// The caller passes the error up to main with `?`, so the process exits non-zero.
pub fn initialize_bare_metal_environment() -> Result<JitterReport, JitterError> {
    let clock = quanta::Clock::new(); // Calibration is slow, so make the clock once, before the hotpath.
    calibrate(&clock)
}
```

- It reads the clock 1,000,000 times in pairs: a few tens of ms. It times only the window inside each pair. It catches a disturbance that hits one of those windows. It does not prove the core is quiet: gate the start with the gap scan below.
- The median is the cost of one read pair: the floor of any probe built on `raw()`.
- 150 is an example in x86_64 TSC ticks. Set it per host: run on a known-quiet core, then put the threshold above its max.
- IF the CPU has no invariant TSC, THEN `raw()` returns OS clock nanoseconds and 150 has another meaning. `quanta` has no API that says which source it chose.
- `samples` is 8 MB, allocated at startup. One spike above threshold fails the run.

## Gap scan: start gate

Compile-checked: cargo check and clippy, x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu, aarch64-apple-darwin. Run for 2 s on aarch64-apple-darwin, on a core that is not isolated: a 1 ms threshold passed, a 5 µs threshold failed with 63 gaps, largest 39 µs. Not run on Linux.

```rust
use std::time::Duration;

pub const SPIKE_LOG_CAPACITY: usize = 4096;

#[derive(Debug, thiserror::Error)]
#[error("{spike_count} gaps above {threshold_nanos} ns in {scanned:?}, largest {maximum_gap_nanos} ns")]
pub struct NoisyCoreError {
    pub threshold_nanos: u64,
    pub maximum_gap_nanos: u64,
    pub spike_count: u64,
    pub scanned: Duration,
    /// Raw clock value at the start of each logged gap, oldest first.
    pub spike_starts: Vec<u64>,
}

/// Reads the clock back to back for the whole `duration`. A gap between two reads is time
/// that this thread did not run, so the scan sees every disturbance of the run.
pub fn scan_for_noise(
    clock: &quanta::Clock,
    duration: Duration,
    threshold_nanos: u64,
) -> Result<u64, NoisyCoreError> {
    let duration_nanos = u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX);
    let mut spike_starts = Vec::with_capacity(SPIKE_LOG_CAPACITY);
    let mut spike_count = 0;
    let mut maximum_gap_nanos = 0;

    let start = clock.raw();
    let mut previous = start;
    while clock.delta_as_nanos(start, previous) < duration_nanos {
        let now = clock.raw();
        let gap_nanos = clock.delta_as_nanos(previous, now);
        if gap_nanos > threshold_nanos {
            spike_count += 1;
            // The log stops at its reserved capacity, so the scan never allocates.
            if spike_starts.len() < SPIKE_LOG_CAPACITY {
                spike_starts.push(previous);
            }
        }
        maximum_gap_nanos = maximum_gap_nanos.max(gap_nanos);
        previous = now;
    }

    if spike_count > 0 {
        return Err(NoisyCoreError {
            threshold_nanos,
            maximum_gap_nanos,
            spike_count,
            scanned: duration,
            spike_starts,
        });
    }
    Ok(maximum_gap_nanos)
}
```

- Call it with 10 s or more: `scan_for_noise(&clock, Duration::from_secs(10), threshold_nanos)?`. It returns the largest gap when no gap passes the threshold.
- Set `threshold_nanos` per host: scan a core that you know is quiet, then put the threshold above its largest gap.
- The threshold is in nanoseconds, so the same value works on x86_64 TSC, aarch64 `CNTVCT_EL0` and the OS clock fallback.
- Period of the noise: take the difference of neighbors in `spike_starts` with `clock.delta_as_nanos(a, b)`. A steady difference names the source: 1 ms or 4 ms = scheduling tick (`CONFIG_HZ` 1000 or 250), about 1 s = residual tick or a 1 s timer.
- The gaps do not say which interrupt it was. Read `/proc/interrupts` before and after the scan to name it. A gap with no interrupt count can be an SMI: `turbostat` shows an SMI column on Intel.

## Kernel audit

Compile-checked: cargo check and clippy, x86_64-unknown-linux-gnu and aarch64-unknown-linux-gnu. Not run.

```rust
#![cfg(target_os = "linux")]
use std::fs;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    #[error("cannot read {}", path.display())]
    Read { path: PathBuf, #[source] source: std::io::Error },
    #[error("{what} is `{found}`, expected `{expected}`")]
    Mismatch { what: &'static str, found: String, expected: String },
}

fn read_trimmed(path: &str) -> Result<String, AuditError> {
    fs::read_to_string(path)
        .map(|text| text.trim().to_owned())
        .map_err(|source| AuditError::Read { path: path.into(), source })
}

fn expect(what: &'static str, path: &str, expected: &str) -> Result<(), AuditError> {
    let found = read_trimmed(path)?;
    if found != expected {
        return Err(AuditError::Mismatch { what, found, expected: expected.to_owned() });
    }
    Ok(())
}

/// Checks the running kernel against the boot string, before the hotpath starts.
pub fn audit_kernel(isolated: &str, hotpath_cpu: usize) -> Result<(), AuditError> {
    expect("isolcpus", "/sys/devices/system/cpu/isolated", isolated)?;
    expect("nohz_full", "/sys/devices/system/cpu/nohz_full", isolated)?;
    expect("nmi_watchdog", "/proc/sys/kernel/nmi_watchdog", "0")?;
    let siblings = format!("/sys/devices/system/cpu/cpu{hotpath_cpu}/topology/thread_siblings_list");
    expect("SMT siblings", &siblings, &hotpath_cpu.to_string())?;
    Ok(())
}
```

- The kernel prints CPU lists in its short form (`4-7`). Pass `isolated` in that form, or parse the list.
- IRQ routing is not in this audit. Add a read of `/proc/irq/*/smp_affinity` IF the host must prove it.
