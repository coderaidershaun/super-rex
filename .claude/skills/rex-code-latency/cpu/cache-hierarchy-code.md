# Cache hierarchy: code

```toml
[package]
edition = "2024"

[dependencies]
thiserror = { version = "2.0.21", default-features = false }
```

`thiserror` with `default-features = false` derives `core::error::Error`. That needs no `std` feature on Rust 1.81 and later.

## Software prefetch before a random lookup

Compile-checked: cargo check, aarch64-apple-darwin and x86_64-unknown-linux-gnu.

Use it only for a lookup that is random by design, such as a directory indexed by an ID from the wire. The state is one aligned 64-byte line per instrument. The prefetch overlaps the memory fetch with work that does not need the state.

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FeedError {
    #[error("instrument id {0} is not in the directory")]
    UnknownInstrument(u32),
}

// align(64) makes every element start on a cache line, not only fill one.
#[repr(C, align(64))]
pub struct InstrumentState {
    pub current_bid: f64,
    pub current_ask: f64,
    pub last_update_timestamp: u64,
}

const _: () = assert!(size_of::<InstrumentState>() == 64);
const _: () = assert!(align_of::<InstrumentState>() == 64);

#[inline(always)]
fn issue_software_prefetch<T>(target: &T) {
    #[cfg(target_arch = "x86_64")]
    {
        use core::arch::x86_64::{_MM_HINT_T0, _mm_prefetch};
        // SAFETY: SSE is part of the x86_64 baseline, so the target feature is always present.
        // A prefetch never faults and never changes memory, and `target` is a valid reference.
        unsafe { _mm_prefetch::<_MM_HINT_T0>(core::ptr::from_ref(target).cast::<i8>()) };
    }
    #[cfg(not(target_arch = "x86_64"))]
    let _ = target;
}

pub struct FeedHandler {
    global_instrument_directory: Vec<InstrumentState>,
}

impl FeedHandler {
    pub fn process_incoming_packet(
        &self,
        parsed_instrument_id: u32,
        remaining_packet_data: &[u8],
    ) -> Result<(), FeedError> {
        // Only the length of the directory is read here, so the target line is not loaded yet.
        let target_state = usize::try_from(parsed_instrument_id)
            .ok()
            .and_then(|index| self.global_instrument_directory.get(index))
            .ok_or(FeedError::UnknownInstrument(parsed_instrument_id))?;

        issue_software_prefetch(target_state);

        // Work that does not need the state runs while the line travels.
        validate_checksum(remaining_packet_data);
        extract_pricing_fields(remaining_packet_data);

        // The line should now be in L1.
        evaluate_trading_strategy(target_state);
        Ok(())
    }
}

fn validate_checksum(_data: &[u8]) {}
fn extract_pricing_fields(_data: &[u8]) {}
fn evaluate_trading_strategy(_state: &InstrumentState) {}
```

- Replace the three stubs with real work. IF that work finishes in less time than one memory fetch, THEN the read still stalls for the rest.
- The ID comes from the wire, so `get` rejects an unknown ID with a typed error. A plain index would panic.
- On a non-x86_64 target the helper does nothing, and the code still compiles on stable.
