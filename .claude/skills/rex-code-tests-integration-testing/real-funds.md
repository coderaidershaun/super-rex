# Real funds protocol

Hard rule + sandbox gates: `.claude/skills/rex-code-tests-integration-testing/SKILL.md`, section REAL FUNDS PROTOCOL.

## Code requirements

1. Mark the test `#[ignore]` so that the default `cargo test` skips it:
   ```rust
   #[test]
   #[ignore = "real funds — requires REX_REAL_FUNDS env var + human runbook"]
   fn places_real_buy_then_cancels() { ... }
   ```

2. Gate the test with an env var that needs an explicit acknowledgment:
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

3. To run the test, give the explicit `--ignored` flag and a filter:
   ```sh
   REX_REAL_FUNDS=yes-i-acknowledge-the-risk \
     REX_PROD_API=true \
     cargo test --test integration -- --ignored places_real_buy
   ```

## Runbook requirement (`tests/integration/README.md`)

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

## Agent + human collaboration protocol

When an agent encounters a real-funds test:

1. **The agent does NOT run the test autonomously.** It stops, reads the runbook, and tells the human about the test.
2. **The agent reads the runbook to the human:** "I'm about to run X. Preconditions are [list]. Have you confirmed each?"
3. **The human acknowledges each precondition** before the agent proceeds.
4. **The agent runs the dry-run first.** It reports the output to the human.
5. **The human gives an explicit go-ahead** before the real run. "ok" is not sufficient. The go-ahead is an explicit "yes, run the real-funds test now".
6. **The agent monitors for kill-switch conditions** during the run. IF anything is not as expected, THEN the agent stops and asks.
7. **The agent reports the outcome and the verification steps** to the human after the run.

IF the human is unclear or not available at any step, THEN the agent does NOT proceed. It defers.

## Real-funds checklist

Real-funds checklist, in addition to the checklist in `SKILL.md`:

- [ ] The `#[ignore = "real funds"]` attribute is present.
- [ ] The test body starts with a `require_real_funds_ack()` call.
- [ ] The test has a runbook section in `tests/integration/README.md`.
- [ ] The runbook lists the preconditions with checkboxes.
- [ ] A dry-run variant exists and runs first.
- [ ] The runbook documents the kill switch.
- [ ] The runbook documents the rollback.
- [ ] The agent obeys the collaboration protocol: it stops, asks, gets an ack, runs the dry-run first, monitors, and reports.

IF any real-funds item fails, THEN fix it before you push, and DO NOT RUN the test.
