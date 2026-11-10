---
name: rex-code-tests-integration-testing
description: Hard rules for Rust integration tests. Real outside world — APIs, exchanges, DBs, WebSockets. Real time, real serialization, real failure modes. Lives in tests/integration/. REAL FUNDS = human-in-the-loop runbook required, no autonomous execution. Use when testing against real APIs or exchanges, end-to-end flows, websocket reconnect, deserialization edge cases, or user asks "test the Coinbase API", "real exchange test", "end-to-end", "run with real funds".
disable-model-invocation: false
user-invocable: true
---

# Rust Integration Tests — Hard Rules

These rules HARD. No compromise.

Sister skills:
- `rex-code-tests-unit-testing` — pure fn scaffold
- `rex-code-tests-contract-seams` — module-to-module
- `rex-code-tests-fitness-functions` — invariants
- `rex-code-philosophy` — integration = sweet spot
- `rex-code-ergonomics`, `rex-code-error-writing`, `rex-code-commenting`

## Iron rule

**Real outside world. Full path. Anticipate reality.**

Job: prove the binary actually works against the real systems it depends on. APIs, exchanges, DBs, message brokers, file system. Real wire format. Real time. Real failure.

If everything is in-process → contract test, not integration.

## Where integration tests live

```
tests/
├── integration.rs         # entry crate. `mod integration;`
└── integration/
    ├── mod.rs             # `pub mod coinbase_ws_feed; pub mod kraken_rest; ...`
    ├── README.md          # ← real-funds runbooks live here
    ├── coinbase_ws_feed.rs
    ├── kraken_rest.rs
    └── postgres_roundtrip.rs
```

Cargo gotcha: top-level `tests/*.rs` is a crate. Subdir needs entry file.

## In scope — what you MUST test

- **Real API call** — sandbox endpoint preferred for default runs
- **Real WebSocket** — subscribe, receive, disconnect, reconnect, resync from snapshot
- **Real DB** — schema migrations, roundtrip, transactional rollback, concurrent writers
- **Real time:**
  - Timeouts (slow server)
  - Retries with backoff
  - Exponential backoff jitter
  - Market hours / session boundaries
  - Daylight saving transitions
  - Clock skew between local + remote
- **Real serialization:**
  - Schema drift (server adds field)
  - Null vs missing vs empty string vs absent key
  - Unknown enum variants (forward compat)
  - Partial response (server cut connection mid-msg)
  - Truncated buffer / framing edge
  - Number precision (f64 vs Decimal across wire)
  - Unicode in user-supplied fields
- **Real failure modes:**
  - 5xx + retry semantics
  - 429 rate limit + Retry-After honoring
  - Partial fill / partial response
  - Duplicate message (idempotency check)
  - Out-of-order delivery (sequence gap detection)
  - Stale data (last-update-time check)

If you didn't anticipate one of these → integration test incomplete.

## Sandbox by default. Prod = explicit gate.

- Default `cargo test --test integration` hits sandbox / mock-server / paper-trade endpoints.
- Prod endpoint requires env var: `REX_PROD_API=true`.
- Real funds requires extra gate (next section).

## REAL FUNDS PROTOCOL — hard rule

**Real-funds integration tests MUST involve a human overseer with explicit runbook. NO autonomous execution.**

### Code requirements

1. Test marked `#[ignore]` so default `cargo test` skips:
   ```rust
   #[test]
   #[ignore = "real funds — requires REX_REAL_FUNDS env var + human runbook"]
   fn places_real_buy_then_cancels() { ... }
   ```

2. Test gated by env var with explicit acknowledgment:
   ```rust
   fn require_real_funds_ack() {
       let ack = std::env::var("REX_REAL_FUNDS").unwrap_or_default();
       assert_eq!(
           ack, "yes-i-acknowledge-the-risk",
           "real funds tests require REX_REAL_FUNDS=yes-i-acknowledge-the-risk"
       );
   }

   #[test]
   #[ignore = "real funds"]
   fn places_real_buy_then_cancels() {
       require_real_funds_ack();
       // ...
   }
   ```

3. Invocation requires explicit `--ignored` flag + filter:
   ```sh
   REX_REAL_FUNDS=yes-i-acknowledge-the-risk \
     REX_PROD_API=true \
     cargo test --test integration -- --ignored places_real_buy
   ```

### Runbook requirement (`tests/integration/README.md`)

Every real-funds test MUST have a runbook section. Template:

````markdown
## `places_real_buy_then_cancels` — real-funds runbook

### Preconditions
- [ ] Account has ≥ $20 USDC available
- [ ] Market is open (not maintenance window)
- [ ] No other active orders on this account
- [ ] Recent `cargo test --test integration -- coinbase_ws_feed` was green
- [ ] Kill switch script ready: `scripts/cancel_all.sh`

### Dry-run first
```sh
REX_PROD_API=true \
  cargo test --test integration -- places_real_buy_then_cancels_DRYRUN
```
Expect: order constructed but not submitted. Logs show "would submit: ...".

### Real run
```sh
REX_REAL_FUNDS=yes-i-acknowledge-the-risk \
  REX_PROD_API=true \
  cargo test --test integration -- --ignored places_real_buy_then_cancels
```

### Expected outcome
- Order placed for $5 worth at 5% below mid (won't fill)
- Wait 2s
- Cancel order
- Test asserts: order_id in cancel response matches placed order_id
- Account balance unchanged (within fee tolerance)

### Verify manually
- [ ] Coinbase web UI → Orders → confirm placed + canceled
- [ ] Account balance returned to within ±$0.10 of pre-test

### Kill switch
If test hangs or behaves oddly:
```sh
scripts/cancel_all.sh coinbase
```
Cancels all open orders on the configured account.

### Rollback
If order partially filled despite price guard:
- Place opposite-side market order for filled qty
- Document incident in `docs/incidents/`
````

### Agent + human collaboration protocol

When agent encounters a real-funds test:

1. **Agent does NOT run autonomously.** Stop. Read runbook. Surface to human.
2. **Agent reads runbook to human:** "I'm about to run X. Preconditions are [list]. Have you confirmed each?"
3. **Human acknowledges each precondition** before agent proceeds.
4. **Agent runs dry-run first.** Reports output to human.
5. **Human gives explicit go-ahead** before real run. Not "ok" — explicit "yes, run the real-funds test now".
6. **Agent monitors for kill-switch conditions** during run. Stops + asks if anything off.
7. **Agent reports outcome + verification steps** to human after run.

If at any step human is unclear or unavailable → agent does NOT proceed. Defer.

## End-to-end example (sandbox, not real funds)

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

## Checklist

- [ ] Test lives in `tests/integration/<source>.rs`
- [ ] Hits real outside world (or sandbox of it)
- [ ] Anticipates time, serialization, failure modes
- [ ] Sandbox by default; prod gated by `REX_PROD_API=true`
- [ ] Errors carry context (per `rex-code-error-writing`)
- [ ] No mocks of services we don't own (use sandbox or stub at network layer)

Real-funds checklist additionally:

- [ ] `#[ignore = "real funds"]` attribute present
- [ ] `require_real_funds_ack()` call at top of test body
- [ ] Runbook section in `tests/integration/README.md`
- [ ] Preconditions listed with checkboxes
- [ ] Dry-run variant exists + runs first
- [ ] Kill switch documented
- [ ] Rollback documented
- [ ] Agent collaboration protocol followed (agent stops, asks, gets ack, runs dry-run first, monitors, reports)

Fail any → fix before push. Real-funds fail any → DO NOT RUN.
