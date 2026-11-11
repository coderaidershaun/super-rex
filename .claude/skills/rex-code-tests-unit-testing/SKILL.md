---
name: rex-code-tests-unit-testing
description: >
  Hard rules for Rust unit tests: inline TDD scaffolds. Delete when contract or
  integration coverage is green. Promote invariants to fitness.
when_to_use: >
  Use when you write unit tests, do TDD, or decide to keep or delete a unit
  test, or user asks "should I unit test this", "TDD this fn", "should I delete
  this test", "promote to fitness".
disable-model-invocation: false
user-invocable: true
---

# Rust Unit Tests — Hard Rules

Other language: apply intent of each rule in idiom of that language. IF language lacks Rust feature that a rule relies on, THEN obey gap skill of that language: skill for what Rust gives by default and that language does not.

These rules HARD. No compromise.

Sister skills:
- `rex-code-tests-contract-seams` — module-to-module
- `rex-code-tests-integration-testing` — outside world
- `rex-code-tests-fitness-functions` — immortal invariants (promotion target)
- `rex-code-philosophy` — integration > unit, no mock of own code
- `rex-code-ergonomics`, `rex-code-error-writing`, `rex-code-commenting`

## Iron rule

**Unit test = TDD scaffold. Delete when done. Or promote to fitness.**

Job: drive small fn into existence + catch immediate regression during TDD. IF module stable + contract or integration test covers it, THEN DELETE. Codebase hygiene > false safety.

Heterodox vs classical pyramid. This repo: integration + contract carry load. Unit = scaffold.

## Where unit tests live

**Inline. Next to code.** `#[cfg(test)] mod tests { ... }` at bottom of `.rs` file.

```rust
// src/orderbook.rs
pub fn parse_price(s: &str) -> Result<Price, ParseError> { ... }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_integer() {
        assert_eq!(parse_price("42").unwrap(), Price(4200));
    }
}
```

NEVER `tests/unit/`. That folder convention = only for contract/integration/fitness. Unit lives with code → delete with code.

## What to unit test

Yes:
- Pure fns (no I/O, no clock, no network, no DB)
- Deterministic transforms (parse, format, compute)
- Single-struct invariants expressible in 3 lines
- Edge cases of pure logic (empty, max, min, off-by-one)

No:
- Anything that touches I/O → contract or integration
- Anything that mocks collaborator → contract (real impls) or integration (real outside)
- Anything with `tokio::time::sleep` or wall clock → integration
- Cross-module call chain → contract seam

## No mocks

Mock = wrong layer. IF you use `mockall`, `mockito`, or hand-rolled fake, THEN you unit-test what belongs in contract or integration test.

```rust
// bad — mocked DB in unit test. Wrong layer.
#[test]
fn updates_user_balance() {
    let mock_db = MockDb::new();
    mock_db.expect_update().returning(|_| Ok(()));
    let svc = UserService::new(mock_db);
    svc.credit(UserId(1), 100).unwrap();
}

// good — pure fn, no collaborators
#[test]
fn balance_after_credit() {
    let acct = Account::new(0);
    let after = acct.credit(100);
    assert_eq!(after.balance(), 100);
}
// (DB roundtrip → tests/integration/db_credit.rs)
```

## TDD workflow

- Red → green → refactor loop: obey only steps 2 Tracer Bullet, 3 Incremental Loop, and 4 Refactor of `.claude/skills/rex-code-tdd/SKILL.md`.
- Then **Decide** — keep inline, promote to fitness, or delete. This step = discipline. You delete most tests.

## Removal criteria — delete when ALL true

- [ ] Module covered by contract test (seam tested) OR integration test (outside-world path tested)
- [ ] Logic stable (>2 weeks unchanged)
- [ ] Test was scaffolding for TDD. It does not assert load-bearing invariant
- [ ] Removal does not reduce signal — other tests still detect refactor

IF all 4 true, THEN delete. Do not grieve. Git remembers.

## Promotion criteria — to fitness function

IF ALL true, THEN move to `tests/fitness/<name>.rs`:

- Tests invariant with large input space (good `proptest` candidate)
- Failure = silent corruption, not loud crash
- Critical to product trust
- Refactor-proof (outlives any specific impl)

Canonical example: TDD for `orderbook::apply_update`. Single-input unit served TDD. Invariant — *orderbook from snapshot + N updates = state after streaming N updates from snapshot* — is fitness material. Promote.

```rust
// during TDD: src/orderbook.rs
#[cfg(test)]
mod tests {
    #[test]
    fn applies_single_update() {
        let mut book = OrderBook::from_snapshot(&snap);
        book.apply(&update);
        assert_eq!(book.bid(0).price, expected);
    }
}

// after TDD green + invariant identified: tests/fitness/orderbook_reconstruction.rs
// (proptest version. inline tests deleted.)
```

See `rex-code-tests-fitness-functions` for promotion target shape.

## End-to-end example

```rust
// === during TDD ===
// src/parser.rs
pub fn parse_iso8601(s: &str) -> Result<DateTime<Utc>, ParseError> { ... }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn rfc3339_basic() { ... }
    #[test] fn rejects_garbage() { ... }
    #[test] fn handles_offset() { ... }
}

// === after TDD green + integration coverage exists ===
// tests/integration/coinbase_ws_feed.rs already exercises parse_iso8601
//   on real WS messages with weird timestamps.
// → DELETE inline tests. Module load-bearing-tested via integration.

// src/parser.rs (no #[cfg(test)] block any more)
pub fn parse_iso8601(s: &str) -> Result<DateTime<Utc>, ParseError> { ... }
```

## Distinct from siblings

| Test type | Scope | Mocks? | Outside world? | Lifespan |
|-----------|-------|--------|----------------|----------|
| **unit** | single fn, pure | no | no | scaffold — delete after green |
| contract | module ↔ module | no | no (stub outside) | as long as seam exists |
| integration | outside world | no | yes | as long as feature exists |
| fitness | architectural invariant | no | depends | until invariant dies |

## Checklist

- [ ] Test lives `#[cfg(test)] mod tests` inline, NOT `tests/unit/`
- [ ] No mocks, no fakes, no spies
- [ ] No I/O, clock, network, DB
- [ ] Tests pure fn or single-struct invariant
- [ ] After TDD green: revisit. Delete or promote.
- [ ] IF module gained contract or integration coverage, THEN consider delete
- [ ] IF invariant load-bearing forever, THEN promote to fitness
- [ ] `unwrap()` only inside test bodies (fine here, `#[cfg(test)]`-scoped)

IF any fail, THEN fix before push.
