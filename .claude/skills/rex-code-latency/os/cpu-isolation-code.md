# CPU isolation code

Startup checks that prove the hotpath core is quiet and that the host booted with the isolation settings. Run both on the pinned thread, before the network connects. Each returns `Err`: carry it to `main` with `?`, so the process exits.

```toml
[dependencies]
thiserror = { version = "2.0.21", default-features = false }
quanta = { version = "0.13.0", default-features = false }
```

## Jitter detector

Compile-checked: cargo check and clippy, x86_64-unknown-linux-gnu. Not run on an isolated core.

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

- 150 is an example in x86_64 TSC ticks. Set it per host: run on a known-quiet core, then put the threshold above its max.
- IF the CPU has no invariant TSC, THEN `raw()` returns OS clock nanoseconds and 150 has another meaning. `quanta` has no API that says which source it chose.
- `samples` is 8 MB, allocated at startup. One spike above threshold fails the run.

## Kernel audit

Compile-checked: cargo check and clippy, x86_64-unknown-linux-gnu. Not run.

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
