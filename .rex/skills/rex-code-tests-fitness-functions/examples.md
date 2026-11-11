# Fitness function examples

Rules + checklist: `.rex/skills/rex-code-tests-fitness-functions/SKILL.md`.

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
- Tests invariant (two paths to same state)
- Doc comment explains WHY this is fitness (silent-corruption risk)
- Uses real `OrderBook`, no mocks
- Lives forever — never delete unless `OrderBook` itself removed

## Wrong place / right place

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
