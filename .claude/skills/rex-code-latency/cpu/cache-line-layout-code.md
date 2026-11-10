# Cache line layout: code

```toml
[package]
edition = "2024"

[dependencies]
crossbeam-utils = { version = "0.8.23", default-features = false }
zerocopy = { version = "0.8.63", default-features = false, features = ["derive"] }
bitfield-struct = "0.13.0" # proc macro, no features
```

All code below is safe Rust.

## Padded counters for two writer cores

Compile-checked: cargo check, aarch64-apple-darwin and x86_64-unknown-linux-gnu. Clippy clean.

```rust
use core::mem::{align_of, offset_of, size_of};
use core::sync::atomic::{AtomicU64, Ordering::Relaxed};
use crossbeam_utils::CachePadded;

// The vulnerable pattern: both counters share one cache line.
#[derive(Debug, Default)]
pub struct VulnerableEngineTelemetry {
    pub buy_orders_processed: AtomicU64,
    pub sell_orders_processed: AtomicU64,
}

// Each counter owns a full boundary, so the two cores never write one line.
#[derive(Debug, Default)]
pub struct ImmuneEngineTelemetry {
    buy_orders_processed: CachePadded<AtomicU64>,
    sell_orders_processed: CachePadded<AtomicU64>,
}

// Fails the build if a later edit puts the two counters inside one boundary.
const _: () = {
    let boundary = align_of::<CachePadded<AtomicU64>>();
    assert!(offset_of!(ImmuneEngineTelemetry, sell_orders_processed)
        .abs_diff(offset_of!(ImmuneEngineTelemetry, buy_orders_processed)) >= boundary);
    assert!(size_of::<ImmuneEngineTelemetry>() == 2 * boundary);
};

#[derive(Debug, Default)]
pub struct MatchingEngineMetrics {
    telemetry_data: ImmuneEngineTelemetry,
}

impl MatchingEngineMetrics {
    // Runs on core zero.
    #[inline]
    pub fn record_buy_order(&self) {
        self.telemetry_data.buy_orders_processed.fetch_add(1, Relaxed);
    }

    // Runs on core one.
    #[inline]
    pub fn record_sell_order(&self) {
        self.telemetry_data.sell_orders_processed.fetch_add(1, Relaxed);
    }
}

// Two cores share one instance through `&`, so the type must be Sync.
const _: () = {
    const fn assert_sync<T: Sync>() {}
    assert_sync::<MatchingEngineMetrics>();
};
```

- Sizes on x86_64 and aarch64: `ImmuneEngineTelemetry` = 256 bytes, `VulnerableEngineTelemetry` = 16 bytes.
- `assert_sync` is the std-only form of `static_assertions::assert_impl_all!(MatchingEngineMetrics: Sync)` (`static_assertions` 1.1.0).
- `fetch_add` fits a counter with many writers. IF one core is the only writer, THEN `load` + `store` is cheaper: on x86_64 `fetch_add` is a `lock` instruction with any ordering.

## Hot state packed into one aligned line

Compile-checked: cargo check, aarch64-apple-darwin and x86_64-unknown-linux-gnu. Clippy clean.

Use this form when the bytes leave the process. IF no peer reads the bytes, THEN use the default repr with `#[repr(align(64))]` and the size and alignment asserts only.

```rust
use bitfield_struct::bitfield;
use core::mem::{align_of, offset_of, size_of};
use zerocopy::{Immutable, IntoBytes, KnownLayout};

// Three flags in one byte. The macro writes the masks and shifts as const fns.
#[bitfield(u8)]
#[derive(IntoBytes, Immutable, KnownLayout)]
pub struct StateFlags {
    pub is_algorithm_active: bool,
    pub is_short_selling_allowed: bool,
    pub has_market_data_arrived: bool,
    #[bits(5)]
    __: u8,
}

// repr(C) keeps the written order, so the order below is the layout.
// align(64) puts the struct at the start of a line, which size alone cannot prove.
#[repr(C, align(64))]
#[derive(Debug, Default, Clone, Copy, IntoBytes, Immutable, KnownLayout)]
pub struct OptimizedTradingState {
    // 1. Eight-byte block: 32 bytes.
    pub upper_price_bound: f64,
    pub lower_price_bound: f64,
    pub optimal_execution_price: f64,
    pub parent_order_id: u64,
    // 2. Four-byte block.
    pub total_filled_volume: u32,
    // 3. Two-byte block.
    pub execution_venue_id: u16,
    // 4. One-byte block.
    pub risk_category: u8,
    // 5. Flag block: one byte.
    pub flags: StateFlags,
    // 6. Explicit padding to the end of the line: 40 bytes used, 24 filled.
    // Explicit, not tail padding: IntoBytes refuses any struct with hidden padding.
    _security_padding: [u8; 24],
}

const _: () = {
    assert!(size_of::<OptimizedTradingState>() == 64, "state left its cache line");
    assert!(align_of::<OptimizedTradingState>() == 64, "state may straddle two lines");
    assert!(offset_of!(OptimizedTradingState, flags) == 39, "hot fields were reordered");
};
```

- Flag accessors: `is_algorithm_active()`, `set_is_algorithm_active(true)`. `StateFlags` is a `repr(transparent)` `u8`.
- Remove `_security_padding` and the build fails: "`OptimizedTradingState` has 24 total byte(s) of padding" (E0277). The size assert alone still passes then, because `align(64)` pads the tail.
- IF the struct is never viewed as bytes, THEN drop the pad array and the `zerocopy` derives. `align(64)` alone gives size 64.
