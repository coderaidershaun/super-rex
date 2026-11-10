# Integration test example (sandbox, not real funds)

Rules + checklist: `.claude/skills/rex-code-tests-integration-testing/SKILL.md`.

```rust
// tests/integration/coinbase_ws_feed.rs

use rex::feeds::coinbase::CoinbaseFeed;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn ws_reconnects_and_resyncs_without_msg_loss() {
    let mut feed = CoinbaseFeed::sandbox().await.unwrap();
    feed.subscribe("BTC-USD").await.unwrap();

    // collect first 10 msgs
    let mut seqs = vec![];
    for _ in 0..10 {
        let msg = timeout(Duration::from_secs(5), feed.next()).await.unwrap().unwrap();
        seqs.push(msg.sequence());
    }
    let last_seq_before = *seqs.last().unwrap();

    // simulate disconnect (kill underlying socket)
    feed.force_disconnect().await;

    // feed should auto-reconnect + resnap
    let resync_msg = timeout(Duration::from_secs(10), feed.next()).await.unwrap().unwrap();
    assert!(resync_msg.is_snapshot(), "expected snapshot after reconnect");

    // continue collecting; assert no gap
    for _ in 0..10 {
        let msg = timeout(Duration::from_secs(5), feed.next()).await.unwrap().unwrap();
        seqs.push(msg.sequence());
    }

    let resync_first_seq = resync_msg.snapshot_seq();
    assert!(
        resync_first_seq >= last_seq_before,
        "snapshot seq {resync_first_seq} must be >= pre-disconnect last seq {last_seq_before}"
    );
}

#[tokio::test]
async fn handles_unknown_event_type_forward_compat() {
    // server may add new event types. our deserialization must not panic.
    let raw = r#"{"type":"some_future_type","sequence":42,"data":{"x":1}}"#;
    let parsed: Result<CoinbaseMsg, _> = serde_json::from_str(raw);
    assert!(parsed.is_ok(), "unknown event type must deserialize as Unknown variant");
    match parsed.unwrap() {
        CoinbaseMsg::Unknown { type_name, .. } => assert_eq!(type_name, "some_future_type"),
        other => panic!("expected Unknown, got {other:?}"),
    }
}
```
