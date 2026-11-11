---
name: rex-code-tests-fitness-functions
description: >
  Hard rules for Rust fitness functions: few, immortal architectural invariants
  in tests/fitness/, with proptest for large input spaces, run on every CI.
when_to_use: >
  Use when you test critical invariants (orderbook reconstruction, money
  conservation, schema backward-compat), or user asks "make sure X never
  breaks", "invariant must always hold", "property test", "promote to fitness".
disable-model-invocation: false
user-invocable: true
---

# Rust Fitness Function Tests — Hard Rules

Other language: apply intent of each rule in idiom of that language. IF language lacks Rust feature that a rule relies on, THEN obey gap skill of that language: skill for what Rust gives by default and that language does not.

These rules HARD. No compromise.

Sister skills:
- `rex-code-tests-unit-testing` — TDD scaffold (promotion source)
- `rex-code-tests-contract-seams` — module boundary
- `rex-code-tests-integration-testing` — outside world
- `rex-code-philosophy` — bug found → regression test → fix
- `rex-code-ergonomics`, `rex-code-error-writing`, `rex-code-commenting`

## Iron rule

**Few. Immortal. Run every CI. Architectural invariants only.**

Fitness function = test of property that system MUST satisfy regardless of impl. Survives refactor. Outlives whoever wrote it. Failure = silent corruption avoided.

Different from regression test (one-shot for fixed bug). Fitness = continuous architectural validation.

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

Cargo gotcha: top-level `tests/*.rs` is one crate. Subdir needs entry.

## What qualifies as a fitness function

ALL must be true:

1. **Architectural invariant** — property over input space, not single-input edge case
2. **Deterministic** — given inputs, output same (proptest needs determinism)
3. **Failure = silent corruption** — not loud crash (loud crash → unit catches it)
4. **Critical to product trust** — "we'd lose customers if this broke" tier
5. **Refactor-proof** — invariant survives any reasonable impl change

IF any fail, THEN not fitness. Can be contract or integration.

## Examples for this domain

- **Orderbook reconstruction:** snapshot + N updates = state after streaming N updates from snapshot. Any input order. Any update set.
- **Money conservation:** sum(account_balances_after) == sum(account_balances_before) for any internal transfer.
- **API backward compat:** v1 API request shape never breaks (regression-protect public surface).
- **Order monotonic seq:** seq numbers never go backwards in any feed under any reorder.
- **Idempotency:** same external order_id submitted twice → exactly one fill.

## Tool: `proptest`

IF input space large, THEN use property-based testing. `proptest` generates inputs. Shrinks failures to minimum reproducer.

```toml
# Cargo.toml
[dev-dependencies]
proptest = "1"
```

## Example tests

IF you write a `proptest!` fitness test, or are not sure a test belongs in fitness, THEN read `.rex/skills/rex-code-tests-fitness-functions/examples.md`. It holds anchor test (orderbook reconstruction) and wrong place / right place.

## What NOT to put in fitness

- Single-input regression test → keep inline `#[cfg(test)] mod tests` near code, OR delete after other coverage protects fix
- Flaky integration test → fix flake or delete. Fitness MUST pass every CI
- Parsing edge case → unit test or delete
- "I think this might break someday" → speculation. Wait until invariant proven critical.

## Promotion path — from unit to fitness

Per `rex-code-tests-unit-testing`:

1. TDD function with inline `#[cfg(test)]` tests
2. Tests survive multiple refactors
3. Identify invariant that tests circle
4. Express it as `proptest!` block
5. Move to `tests/fitness/<name>.rs`
6. Delete inline scaffold tests

Promote ≠ "move all unit tests up". Promote ≠ "have many fitness tests". Bar is high.

## CI requirement

- Every fitness test runs on every PR. No `#[ignore]`.
- IF fitness fails, THEN block merge. No "we'll fix it later".
- IF slow fitness (proptest with many cases), THEN cap at 256 for CI, full 4096 nightly:

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

Fitness = delete **when invariant itself dies**. Mocks: no. Outside world: depends.
Table of all four lanes: `.rex/skills/rex-code-tests-unit-testing/SKILL.md`, section Distinct from siblings.

## Checklist

- [ ] Test lives in `tests/fitness/<invariant>.rs`
- [ ] Tests architectural invariant, not single input
- [ ] Deterministic given inputs
- [ ] IF input space large, THEN uses `proptest`
- [ ] Doc comment explains WHY this is fitness (silent-corruption risk)
- [ ] Real impls, no mocks
- [ ] Runs on every CI (no `#[ignore]`)
- [ ] Failure mode = silent corruption, not loud crash
- [ ] Survives reasonable refactor of impl

IF any fail, THEN not fitness. Move to correct sibling skill's lane.
