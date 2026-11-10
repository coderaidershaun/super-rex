---
name: rex-code-tests-fitness-functions
description: Hard rules for Rust fitness function tests. Few, immortal architectural invariants. proptest for large input space. Lives in tests/fitness/. Runs every CI. Anchor example = orderbook reconstruction (snapshot + updates always equals streamed state). Use when testing critical invariants like orderbook reconstruction, money conservation, schema backward-compat, or user asks "make sure X never breaks", "invariant must always hold", "property test", "promote to fitness".
disable-model-invocation: false
user-invocable: true
---

# Rust Fitness Function Tests — Hard Rules

These rules HARD. No compromise.

Sister skills:
- `rex-code-tests-unit-testing` — TDD scaffold (promotion source)
- `rex-code-tests-contract-seams` — module boundary
- `rex-code-tests-integration-testing` — outside world
- `rex-code-philosophy` — bug found → regression test → fix
- `rex-code-ergonomics`, `rex-code-error-writing`, `rex-code-commenting`

## Iron rule

**Few. Immortal. Run every CI. Architectural invariants only.**

Fitness function = test of a property the system MUST satisfy regardless of impl. Survives refactor. Outlives whoever wrote it. Failure = silent corruption avoided.

Different from regression test (one-shot for a fixed bug). Fitness = continuous architectural validation.

## Where fitness tests live

```
tests/
├── fitness.rs             # entry. `mod fitness;`
└── fitness/
    ├── mod.rs             # `pub mod orderbook_reconstruction; pub mod money_conservation;`
    ├── orderbook_reconstruction.rs
    ├── money_conservation.rs
    └── api_backward_compat.rs
```

Cargo gotcha: top-level `tests/*.rs` is a crate. Subdir needs entry.

## What qualifies as a fitness function

ALL must be true:

1. **Architectural invariant** — property over input space, not single-input edge case
2. **Deterministic** — given inputs, output same (proptest needs determinism)
3. **Failure = silent corruption** — not loud crash (loud crash → unit catches it)
4. **Critical to product trust** — "we'd lose customers if this broke" tier
5. **Refactor-proof** — invariant survives any reasonable impl change

Fail any → not fitness. Maybe contract or integration.

## Examples for this domain

- **Orderbook reconstruction:** snapshot + N updates = state after streaming N updates from snapshot. Any input order. Any update set.
- **Money conservation:** sum(account_balances_after) == sum(account_balances_before) for any internal transfer.
- **API backward compat:** v1 API request shape never breaks (regression-protect public surface).
- **Order monotonic seq:** seq numbers never go backwards in any feed under any reorder.
- **Idempotency:** same external order_id submitted twice → exactly one fill.

## Tool: `proptest`

Input space large → use property-based testing. `proptest` generates inputs. Shrinks failures to minimum reproducer.

```toml
# Cargo.toml
[dev-dependencies]
proptest = "1"
```

## Anchor example — orderbook reconstruction

User-named anchor invariant. Canonical fitness test in this codebase.

```rust
// tests/fitness/orderbook_reconstruction.rs

use proptest::prelude::*;
use rex::orderbook::{OrderBook, Side, Snapshot, Update};

fn arb_update() -> impl Strategy<Value = Update> {
    (
        any::<u64>(),                                   // sequence
        prop_oneof![Just(Side::Bid), Just(Side::Ask)],
        1u64..1_000_000,                                // price
        0u64..10_000,                                   // qty (0 = remove level)
    ).prop_map(|(seq, side, price, qty)| Update { seq, side, price, qty })
}

fn arb_snapshot() -> impl Strategy<Value = Snapshot> {
    prop::collection::vec(
        (
            prop_oneof![Just(Side::Bid), Just(Side::Ask)],
            1u64..1_000_000,
            1u64..10_000,
        ),
        0..50,
    ).prop_map(|levels| Snapshot {
        last_seq: 0,
        levels: levels.into_iter().collect(),
    })
}

proptest! {
    /// FITNESS: orderbook reconstructed from snapshot + sequential updates
    /// must equal state produced by applying same updates as a batch.
    ///
    /// If this fails, market data state can desync from exchange = silent corruption.
    #[test]
    fn reconstructs_from_snapshot_plus_updates(
        snap in arb_snapshot(),
        mut updates in prop::collection::vec(arb_update(), 0..200),
    ) {
        // canonicalize: sort updates by seq so they're sequential
        updates.sort_by_key(|u| u.seq);

        // path A: apply updates one-by-one
        let mut book_a = OrderBook::from_snapshot(&snap);
        for u in &updates {
            let _ = book_a.apply_update(u);  // skip out-of-order, that's OK
        }

        // path B: apply same updates as single batch
        let mut book_b = OrderBook::from_snapshot(&snap);
        book_b.apply_batch(&updates);

        prop_assert_eq!(
            book_a.canonical_state(),
            book_b.canonical_state(),
            "orderbook divergence: streaming vs batch must produce identical state"
        );
    }
}
```

Key features:
- `proptest!` generates many input combos, shrinks on failure
- Tests an invariant (two paths to same state)
- Doc comment explains WHY this is fitness (silent-corruption risk)
- Uses real `OrderBook`, no mocks
- Lives forever — never delete unless `OrderBook` itself removed

## What NOT to put in fitness

- Single-input regression test → keep inline `#[cfg(test)] mod tests` near code, OR delete after fix locked in by other coverage
- Flaky integration test → fix the flake or delete; fitness MUST pass every CI
- Parsing edge case → unit test or delete
- "I think this might break someday" → speculation. Wait until invariant proven critical.

```rust
// bad — single-input regression. Wrong place.
#[test]
fn parses_specific_btc_usd_msg() {
    let raw = r#"{"type":"match","sequence":12345,...}"#;
    assert!(serde_json::from_str::<CoinbaseMsg>(raw).is_ok());
}
// → belongs inline as #[cfg(test)] mod tests in coinbase.rs, or delete

// good — architectural invariant. Right place.
proptest! {
    #[test]
    fn money_conserved_in_internal_transfer(
        accts in arb_accounts(),
        transfers in prop::collection::vec(arb_transfer(), 0..50),
    ) {
        let total_before: u64 = accts.iter().map(|a| a.balance).sum();
        let mut state = AccountState::from(accts);
        for t in &transfers {
            let _ = state.apply_transfer(t);  // some may fail; that's fine
        }
        let total_after: u64 = state.accounts().iter().map(|a| a.balance).sum();
        prop_assert_eq!(total_before, total_after, "internal transfers must conserve money");
    }
}
```

## Promotion path — from unit to fitness

Per `rex-code-tests-unit-testing`:

1. TDD a function with inline `#[cfg(test)]` tests
2. Tests survive multiple refactors
3. Identify the invariant tests are circling
4. Express it as a `proptest!` block
5. Move to `tests/fitness/<name>.rs`
6. Delete inline scaffold tests

Promote ≠ "move all unit tests up". Promote ≠ "have many fitness tests". Bar is high.

## CI requirement

- Every fitness test runs on every PR. No `#[ignore]`.
- Fitness fail → block merge. No "we'll fix it later".
- Slow fitness (proptest with many cases) → cap at 256 for CI, full 4096 nightly:

```rust
proptest! {
    #![proptest_config(ProptestConfig {
        cases: if std::env::var("CI_NIGHTLY").is_ok() { 4096 } else { 256 },
        ..ProptestConfig::default()
    })]

    #[test]
    fn ... {}
}
```

## Distinct from siblings

| Test type | When deleted? | Mocks? | Outside world? |
|-----------|---------------|--------|----------------|
| unit | after TDD green + other coverage | no | no |
| contract | when seam removed | no | stub outside |
| integration | when feature removed | no | yes |
| **fitness** | **when invariant itself dies** | no | depends |

## Checklist

- [ ] Test lives in `tests/fitness/<invariant>.rs`
- [ ] Tests an architectural invariant, not a single input
- [ ] Deterministic given inputs
- [ ] Uses `proptest` if input space large
- [ ] Doc comment explains WHY this is fitness (silent-corruption risk)
- [ ] Real impls, no mocks
- [ ] Runs on every CI (no `#[ignore]`)
- [ ] Failure mode = silent corruption, not loud crash
- [ ] Survives reasonable refactor of impl

Fail any → not fitness. Move to correct sibling skill's lane.
