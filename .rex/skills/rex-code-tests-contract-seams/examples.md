# Contract seam test examples

Rules + checklist: `.rex/skills/rex-code-tests-contract-seams/SKILL.md`.

## Real impls / mocked impl

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
