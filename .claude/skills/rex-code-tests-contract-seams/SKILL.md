---
name: rex-code-tests-contract-seams
description: Hard rules for Rust contract seam tests. Test interface between modules using real impls on both sides. Lives in tests/contract/. Catches refactor breakage at module boundaries. Use when testing module-to-module interactions, after refactoring a seam, defining a new module interface, or user asks "test the boundary", "this seam keeps breaking", "interaction between modules".
disable-model-invocation: false
user-invocable: true
---

# Rust Contract Seam Tests — Hard Rules

These rules HARD. No compromise.

Sister skills:
- `rex-code-tests-unit-testing` — pure fn scaffold (deletable)
- `rex-code-tests-integration-testing` — outside world
- `rex-code-tests-fitness-functions` — immortal invariants
- `rex-code-philosophy` — locality, deepening
- `rex-code-improve-codebase-architecture` — what is a seam
- `rex-code-ergonomics`, `rex-code-error-writing`, `rex-code-commenting`

## Iron rule

**Test the seam. Not the impls on either side.**

Seam = stable module-to-module interface inside the codebase. Contract test = "if A calls B this way, B promises this shape back." Catch = refactor of A or B breaks the promise → loud failure.

## What is a seam

- Stable interface between two modules
- Examples in this domain: `OrderBook → Strategy`, `Strategy → Risk`, `Risk → Executor`, `MarketData → OrderBook`
- BOTH sides = real impls in the binary. Not external. Not mocked.
- Contract = method signature + error variants + ordering guarantees + invariants the seam promised

If interface unstable → too early for contract test. Wait for shape (per philosophy).

## Where contract tests live

```
tests/
├── contract.rs            # entry crate. just: `mod contract;`
└── contract/
    ├── mod.rs             # `pub mod orderbook_to_strategy; pub mod strategy_to_risk;`
    ├── orderbook_to_strategy.rs
    └── strategy_to_risk.rs
```

Cargo gotcha: each `.rs` directly under `tests/` is its own crate. Subdir needs entry file.

`tests/contract.rs`:
```rust
mod contract;
```

`tests/contract/mod.rs`:
```rust
pub mod orderbook_to_strategy;
pub mod strategy_to_risk;
```

## Use real impls. Stub only the outside.

- Real `OrderBook`. Real `Strategy`. They are the seam.
- Stub network, clock, FS — anything OUTSIDE the codebase.
- If you reach for `mockall` on a module you own → wrong tool. Use the real one.

```rust
// good — real impls of both sides
#[test]
fn strategy_receives_snapshot_then_updates_in_order() {
    let mut book = OrderBook::new();
    let mut strategy = Strategy::new();
    book.subscribe(&mut strategy);

    book.apply_snapshot(snapshot());
    book.apply_update(update_1());
    book.apply_update(update_2());

    assert_eq!(strategy.events_received(), vec![
        Event::Snapshot { /* ... */ },
        Event::Update { seq: 1, /* ... */ },
        Event::Update { seq: 2, /* ... */ },
    ]);
}

// bad — mocked OrderBook. Tests the mock, not the seam.
#[test]
fn strategy_handles_mocked_book() {
    let mock = MockOrderBook::new();
    mock.expect_subscribe().times(1);
    Strategy::new().connect(&mock);  // proves nothing about real seam
}
```

## What contract tests catch

- Method signature change (caller breaks)
- Error variant added/removed (caller match no longer exhaustive)
- Ordering guarantee broken (events delivered out of order)
- Invariant the seam promised (e.g., "subscriber gets snapshot before any update")
- New required parameter

If contract test breaks under refactor → seam is shifting. Decide: update contract or revert refactor.

## Pattern: one test per outcome shape

For each cross-module call, write tests covering:

- Happy path (Ok variant)
- Each `Err` variant the seam can return
- Boundary: empty input, max input, concurrent calls
- Invariants under load (ordering, idempotency)

Not exhaustive coverage. Coverage of the *contract*, not the *impl*.

## Distinct from siblings

| Test type | Scope | Mocks? | Outside world? |
|-----------|-------|--------|----------------|
| unit | single fn, pure | no | no |
| **contract** | module ↔ module, real impls | no | no (stub outside) |
| integration | outside world | no | yes |
| fitness | architectural invariant | no | depends |

If you're mocking a module the codebase owns → switch to real impl, you're contract-testing.
If you're mocking the network → either stub it (unit/contract) or hit it real (integration).

## End-to-end example

```rust
// tests/contract/orderbook_to_strategy.rs

use rex::orderbook::{OrderBook, OrderBookError, Snapshot, Update};
use rex::strategy::{Event, Strategy};

#[test]
fn snapshot_arrives_before_any_update() {
    let mut book = OrderBook::new();
    let mut strategy = Strategy::new();
    book.subscribe(&mut strategy);

    // even if updates queued first, snapshot must land first
    book.queue_update(Update { seq: 1, /* ... */ });
    book.queue_update(Update { seq: 2, /* ... */ });
    book.apply_snapshot(Snapshot { /* ... */ });
    book.flush();

    let events = strategy.events_received();
    assert!(matches!(events[0], Event::Snapshot { .. }), "snapshot must be first");
    assert!(matches!(events[1], Event::Update { seq: 1, .. }));
    assert!(matches!(events[2], Event::Update { seq: 2, .. }));
}

#[test]
fn out_of_order_update_returns_specific_err() {
    let mut book = OrderBook::new();
    let mut strategy = Strategy::new();
    book.subscribe(&mut strategy);
    book.apply_snapshot(Snapshot { last_seq: 10, /* ... */ });

    let result = book.apply_update(Update { seq: 8, /* ... */ });

    assert!(matches!(
        result,
        Err(OrderBookError::OutOfOrder { expected: 11, got: 8 })
    ));
}

#[test]
fn subscribe_then_unsubscribe_stops_delivery() {
    let mut book = OrderBook::new();
    let mut strategy = Strategy::new();
    let sub_id = book.subscribe(&mut strategy);
    book.apply_snapshot(Snapshot { /* ... */ });
    book.unsubscribe(sub_id);
    book.apply_update(Update { /* ... */ });

    assert_eq!(strategy.events_received().len(), 1, "no events after unsubscribe");
}
```

## Checklist

- [ ] Test lives in `tests/contract/<seam_name>.rs`
- [ ] Both sides of seam = real impls (not mocks)
- [ ] Outside world (network, clock, FS) is stubbed, not hit
- [ ] One test per outcome shape (Ok + each Err variant + boundary)
- [ ] Asserts the *contract* (signatures, ordering, errors), not impl details
- [ ] Test name describes the contract clause being verified
- [ ] Expected `Err` matched explicitly with `matches!` or destructure (per `rex-code-error-writing`)

Fail any → fix before push.
