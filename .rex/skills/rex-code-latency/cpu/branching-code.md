# Branching and dispatch: code

```toml
[package]
edition = "2024"
rust-version = "1.95" # std::hint::cold_path is stable since 1.95.0

[dependencies]
thiserror = { version = "2.0.21", default-features = false }

[profile.release]
codegen-units = 1 # one unit: LLVM sees every fn for inlining and layout
lto = "fat"
panic = "abort"   # no unwind tables on the hotpath
```

All code below is safe Rust. Assembly facts below come from `cargo asm --lib --target x86_64-unknown-linux-gnu --intel --simplify <fn>`, rustc 1.97.1, release profile above. Read them as what to look for, then read your own output.

## Route by a data-dependent condition with no branch

Compile-checked: cargo check and clippy, aarch64-apple-darwin, x86_64-unknown-linux-gnu and aarch64-unknown-linux-gnu. Not run.

```rust
// Route each price to the bid or ask side with no data-dependent branch.
use std::hint::select_unpredictable;

pub struct SideRouter {
    pub bids: Vec<u64>,
    pub asks: Vec<u64>,
}

impl SideRouter {
    pub fn with_capacity(n: usize) -> Self {
        Self { bids: Vec::with_capacity(n), asks: Vec::with_capacity(n) }
    }

    // `if p > mid { asks.push(p) } else { bids.push(p) }` keeps a branch on p.
    // The select picks the Vec with cmov, then one push path runs.
    pub fn route(&mut self, prices: &[u64], mid: u64) {
        for &p in prices {
            select_unpredictable(p > mid, &mut self.asks, &mut self.bids).push(p);
        }
    }
}
```

- The `if`/`else` form compiles to `cmp; ja` on the price. The select form compiles to `cmp; cmova`. The only branches left are the loop and the `Vec` capacity check.
- IF more prices arrive than the capacity, THEN `push` allocates on the hotpath. Size the capacity for the largest burst.
- A plain filter with a small body (`if v >= thr { sum += v }`) already compiles to `cmp; cmovb; add` with no hint. Do not rewrite that one.

## Packet validator with a cold error path

Compile-checked: cargo check and clippy, aarch64-apple-darwin, x86_64-unknown-linux-gnu and aarch64-unknown-linux-gnu. Not run.

```rust
// The checksum test falls through into the hotpath block. The error path lives out of line.
use std::hint::cold_path;

pub const EXPECTED_PROTOCOL_SIGNATURE: u32 = 0xABCD_1234;

#[repr(C)]
pub struct ExchangeMarketPacket {
    pub protocol_checksum: u32,
    pub payload_length: u32,
    pub resting_price: u64,
    pub resting_volume: u64,
}

#[derive(Debug, thiserror::Error)]
#[error("bad protocol signature {found:#010x}")]
pub struct PacketRejected {
    pub found: u32,
}

pub struct DeterministicPacketValidator {
    pub notional: u64,
    pub rejected: u64,
}

impl DeterministicPacketValidator {
    pub fn process_incoming_market_data(
        &mut self,
        packet: &ExchangeMarketPacket,
    ) -> Result<(), PacketRejected> {
        if packet.protocol_checksum == EXPECTED_PROTOCOL_SIGNATURE {
            self.execute_critical_fast_path(packet);
            Ok(())
        } else {
            cold_path();
            Err(self.execute_cold_error_path(packet))
        }
    }

    #[inline]
    fn execute_critical_fast_path(&mut self, packet: &ExchangeMarketPacket) {
        self.notional = self
            .notional
            .wrapping_add(packet.resting_price.wrapping_mul(packet.resting_volume));
    }

    // Rare path: never inlined, so its bytes stay out of the caller.
    #[cold]
    #[inline(never)]
    fn execute_cold_error_path(&mut self, packet: &ExchangeMarketPacket) -> PacketRejected {
        self.rejected += 1;
        PacketRejected { found: packet.protocol_checksum }
    }
}
```

- With the hints: hotpath is `cmp; jne cold; imul; add; ret`, 8 instructions from entry to `ret`. The register save (`push rbx`) moves to the cold block. The cold fn sits in `.text.unlikely.*`.
- With no hints: 11 instructions, `push rbx` is the first hotpath instruction, and the error fn sits in plain `.text`.
- No hint on the `if` side: marking the other side cold is enough.

## Generic gateway with a build-time version check

Compile-checked: cargo check and clippy, aarch64-apple-darwin, x86_64-unknown-linux-gnu and aarch64-unknown-linux-gnu. The build-fail case was checked with `cargo build`. Not run.

```rust
// The forbidden pattern on the hotpath: one indirect call through a vtable.
pub trait StandardExchangeGateway {
    fn transmit_market_order(&self, order_identifier: u64);
}

pub struct LegacyMarketGateway;

impl StandardExchangeGateway for LegacyMarketGateway {
    fn transmit_market_order(&self, order_identifier: u64) {
        std::hint::black_box(order_identifier);
    }
}

pub fn route_dynamic(gateway: &dyn StandardExchangeGateway, id: u64) {
    gateway.transmit_market_order(id);
}

// The replacement: a trait bound that the compiler resolves per concrete type.
pub trait CompileTimeExchangeGateway {
    const PROTOCOL_VERSION: u32;

    fn execute_transmission(&self, order_identifier: u64);

    #[inline]
    fn transmit_market_order(&self, order_identifier: u64) {
        self.execute_transmission(order_identifier);
    }
}

pub struct UltraFastMarketGateway;

impl CompileTimeExchangeGateway for UltraFastMarketGateway {
    const PROTOCOL_VERSION: u32 = 2026;

    #[inline]
    fn execute_transmission(&self, order_identifier: u64) {
        std::hint::black_box(order_identifier);
    }
}

pub const MIN_PROTOCOL_VERSION: u32 = 2026;

pub fn process_and_route_order<G: CompileTimeExchangeGateway>(active_gateway: &G, id: u64) -> u32 {
    // Evaluated once per concrete G at build time. A gateway below the minimum fails the build.
    let current_version = const {
        assert!(G::PROTOCOL_VERSION >= MIN_PROTOCOL_VERSION, "gateway protocol too old");
        G::PROTOCOL_VERSION
    };
    active_gateway.transmit_market_order(id);
    current_version
}

pub fn route_ultra_fast(id: u64) -> u32 {
    process_and_route_order(&UltraFastMarketGateway, id)
}
```

- `route_dynamic` compiles to `mov rax, qword ptr [rsi + 24]; jmp rax`: one load from the vtable, one indirect jump.
- `route_ultra_fast` inlines the transmit body, has no call, and returns `mov eax, 2026`.
- A gateway with version 2025 passes `cargo check` and fails `cargo build` with `error[E0080]: evaluation panicked: gateway protocol too old`.
- `black_box` only keeps the empty bodies alive for the inspection. Do not put it on a real hotpath.
- IF the set of gateways is closed and chosen at run time, THEN put them in an `enum` and derive dispatch with `enum_dispatch` 0.3.13. The call becomes a `match` on the tag, and each arm can inline.
