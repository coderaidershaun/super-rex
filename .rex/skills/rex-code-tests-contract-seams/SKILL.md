---
name: rex-code-tests-contract-seams
description: >
  Hard rules for Rust contract seam tests: real impls on both sides of a module
  interface, in tests/contract/. Catches refactor breakage at module boundaries.
when_to_use: >
  Use when you test module-to-module interactions, refactor a seam, or define a
  new module interface, or user asks "test the boundary", "this seam keeps
  breaking", "interaction between modules".
---

# Rust Contract Seam Tests — Hard Rules

Other language: apply intent of each rule in idiom of that language. IF language lacks Rust feature that a rule relies on, THEN obey gap skill of that language: skill for what Rust gives by default and that language does not.

These rules HARD. No compromise.

Sister skills:
- `rex-code-tests-unit-testing` — pure fn scaffold (deletable)
- `rex-code-tests-integration-testing` — outside world
- `rex-code-tests-fitness-functions` — immortal invariants
- `rex-code-philosophy` — locality
- `rex-code-improve-codebase-architecture` — what is a seam, deepening
- `rex-code-ergonomics`, `rex-code-error-writing`, `rex-code-commenting`

## Iron rule

**Test seam. Not impls on either side.**

Seam = stable module-to-module interface inside codebase. Contract test = "if A calls B this way, B promises this shape back." Catch = refactor of A or B breaks promise → loud failure.

## What is a seam

- Stable interface between two modules
- Examples in this domain: `OrderBook → Strategy`, `Strategy → Risk`, `Risk → Executor`, `MarketData → OrderBook`
- BOTH sides = real impls in binary. Not external. Not mocked.
- Contract = method signature + error variants + ordering guarantees + invariants that seam promised

IF interface unstable, THEN too early for contract test. Wait for shape (per philosophy).

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

## Use real impls. Stub only outside.

- Real `OrderBook`. Real `Strategy`. They are the seam.
- Stub network, clock, FS — anything OUTSIDE codebase.
- IF you use `mockall` on module that you own, THEN wrong tool. Use real one.

IF you write a contract test and want its shape, THEN read `.rex/skills/rex-code-tests-contract-seams/examples.md`: real impls vs mocked impl, and one full test file.

## What contract tests catch

- Method signature change (caller breaks)
- Error variant added/removed (caller match no longer exhaustive)
- Ordering guarantee broken (events delivered out of order)
- Invariant that seam promised (e.g., "subscriber gets snapshot before any update")
- New required parameter

IF contract test breaks under refactor, THEN seam shifts. Decide: update contract or revert refactor.

## Pattern: one test per outcome shape

For each cross-module call, write tests that cover:

- Happy path (Ok variant)
- Each `Err` variant that seam can return
- Boundary: empty input, max input, concurrent calls
- Invariants under load (ordering, idempotency)

Not exhaustive coverage. Coverage of *contract*, not *impl*.

## Distinct from siblings

Contract = module ↔ module, real impls. Mocks: no. Outside world: no (stub outside).
Table of all four lanes: `.rex/skills/rex-code-tests-unit-testing/SKILL.md`, section Distinct from siblings.

IF you mock module that codebase owns, THEN switch to real impl. You contract-test.
IF you mock network, THEN stub it (unit/contract) or hit it real (integration).

## Checklist

- [ ] Test lives in `tests/contract/<seam_name>.rs`
- [ ] Both sides of seam = real impls (not mocks)
- [ ] Outside world (network, clock, FS): stub, no real hit
- [ ] One test per outcome shape (Ok + each Err variant + boundary)
- [ ] Asserts *contract* (signatures, ordering, errors), not impl details
- [ ] Test name describes contract clause that test proves
- [ ] Test matches expected `Err` explicitly with `matches!` or destructure (per `rex-code-error-writing`)

IF any fail, THEN fix before push.
