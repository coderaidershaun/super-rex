---
name: rex-code-tests-integration-testing
description: >
  Hard rules for Rust integration tests against the real outside world (APIs,
  exchanges, DBs, WebSockets), in tests/integration/. Real funds need a human
  runbook, never autonomous runs.
when_to_use: >
  Use when you test real APIs or exchanges, websocket reconnect, or
  deserialization edge cases, or user asks "test the Coinbase API", "real
  exchange test", "end-to-end", "run with real funds".
disable-model-invocation: false
user-invocable: true
---

# Rust Integration Tests — Hard Rules

Other language: apply intent of each rule in idiom of that language. IF language lacks Rust feature that a rule relies on, THEN obey gap skill of that language: skill for what Rust gives by default and that language does not.

These rules HARD. No compromise.

Sister skills:
- `rex-code-tests-unit-testing` — pure fn scaffold
- `rex-code-tests-contract-seams` — module-to-module
- `rex-code-tests-fitness-functions` — invariants
- `rex-code-philosophy` — integration = sweet spot
- `rex-code-ergonomics`, `rex-code-error-writing`, `rex-code-commenting`

## Iron rule

**Real outside world. Full path. Anticipate reality.**

Job: prove binary works against real systems that it depends on. APIs, exchanges, DBs, message brokers, file system. Real wire format. Real time. Real failure.

IF everything is in-process, THEN contract test, not integration.

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

Cargo gotcha: top-level `tests/*.rs` is one crate. Subdir needs entry file.

## In scope — what you MUST test

- **Real API call** — prefer sandbox endpoint for default runs
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
  - 429 rate limit + obey Retry-After
  - Partial fill / partial response
  - Duplicate message (idempotency check)
  - Out-of-order delivery (sequence gap detection)
  - Stale data (last-update-time check)

IF you did not anticipate one of these, THEN integration test is incomplete.

## Sandbox by default. Prod = explicit gate.

- Default `cargo test --test integration` hits sandbox / mock-server / paper-trade endpoints.
- Prod endpoint requires env var: `REX_PROD_API=true`.
- Real funds requires extra gate (next section).

## REAL FUNDS PROTOCOL — hard rule

**Real-funds integration tests MUST involve a human overseer with an explicit runbook. An agent must NOT run them autonomously.**

IF you write, change, run, or meet a real-funds test, THEN read `.rex/skills/rex-code-tests-integration-testing/real-funds.md` IN FULL before you go on. It holds code gates, runbook template, agent + human protocol, and real-funds checklist.

## Example

IF you write an integration test and want its shape, THEN read `.rex/skills/rex-code-tests-integration-testing/examples.md`: sandbox WebSocket reconnect test + forward-compat deserialization test.

## Checklist

- [ ] Test lives in `tests/integration/<source>.rs`
- [ ] Hits real outside world (or sandbox of it)
- [ ] Anticipates time, serialization, failure modes
- [ ] Sandbox by default. `REX_PROD_API=true` gates prod
- [ ] Errors carry context (per `rex-code-error-writing`)
- [ ] No mocks of services we do not own (use sandbox or stub at network layer)

IF any fail, THEN fix before push.
