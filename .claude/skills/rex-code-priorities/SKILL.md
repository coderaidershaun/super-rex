---
name: rex-code-priorities
description: >
  Hard rules for the priority block of a repo: ranked qualities plus things
  the system must never do, written as the crate-level `//!` comment in
  `lib.rs` or `main.rs`. The orchestrator asks the user for it one time, up
  front. Each agent that designs or reviews reads it and uses it to break
  ties. No CONTEXT.md, no docs folder, no ADR. Use this skill when an
  orchestrator starts work on a repo, when a plan or architecture decision has
  a tie, when the priority block is missing, or when the user says "repo
  priorities", "what matters most here", "rank the priorities", or
  "constitution".
disable-model-invocation: false
user-invocable: true
---

# Repo Priorities — Hard Rules

No best design. Only best design for this repo's ranking.
Ranking lives in code. One place. Each agent opens that file.

Sister skills:

- `rex-code-philosophy` — default ranking when repo gives none for a small choice: simplicity first, correctness second.
- `rex-code-design-sketch` — first reader of priority block.
- `rex-code-failure-policy` — each Never line gets HALT class.
- `rex-code-tests-fitness-functions` — each Never line is fitness candidate.
- `rex-code-commenting` — law for `//!` lines.

## Words

| Term | Meaning |
| --- | --- |
| Priority block | The `//!` lines in crate root that hold Priority line + Never lines. |
| Priority line | Ranked list of qualities. First = most important. |
| Never line | One thing system must never do. |
| Quality | Property of system: correctness, time to ship, extensibility, latency, throughput, cost, ease of operation. |
| Crate root | `lib.rs`. IF crate has no `lib.rs`, THEN `main.rs`. |
| Caller | Agent or person that started you. |

## Shape

```rust
//! Sends invoices and collects payments for small shops.
//!
//! Priorities, in order: correctness, time to ship, extensibility (new payment providers).
//! Never: charge a customer twice.
//! Never: lose a saved invoice.
```

- [ ] First `//!` line: why crate exists. Plain English.
- [ ] Priority line starts with `Priorities, in order:`. Exact words. Agents grep for them.
- [ ] Each Never line starts with `Never:`.
- [ ] 2 to 4 qualities. Ranked. No ties. No "all equal".
- [ ] IF quality has a subject, THEN name it in brackets: `extensibility (new exchanges)`.
- [ ] IF lower quality has a floor, THEN give number: `latency (under 5 ms per message)`.
- [ ] 0 to 3 Never lines. Each one = fact that a test can prove.
- [ ] Priority block: max 8 lines. More = style guide. Skills already hold style.
- [ ] Workspace: one priority block in each crate root that has different ranking. ELSE one block in main binary crate.

## WRITE: orchestrator only

Ask user these questions one time, with other up-front questions. Give recommended answer for each, from task + code.

1. Rank these for this repo: correctness, time to ship, extensibility, performance, cost. Which 2 to 4 are important, in which order?
2. What will there be more of later? (Example: exchanges, file formats, tenants, payment providers.)
3. What must this system never do, even one time?
4. For each quality below first: what is good enough?

Then:

- [ ] Write priority block in crate root.
- [ ] Read it back to user in report.
- [ ] IF repo already has priority block, THEN ask nothing. Use it.

CAUTION: Do not change a priority block without an instruction from the user. A changed ranking changes each later design decision, and no test reports it.

## READ: all other agents

```
grep -rn 'Priorities, in order:' --include=lib.rs --include=main.rs .
```

- [ ] IF no priority block AND decision depends on ranking, THEN stop. Send `STATUS: QUESTIONS` to caller. Never guess ranking.
- [ ] IF no priority block AND decision is small + local, THEN obey `rex-code-philosophy`. Continue.
- [ ] Never ask the person direct. Only orchestrator talks to person about ranking.
- [ ] Put Priority line + Never lines in each brief for a subagent that designs or reviews.

## USE: break ties

```
IF two options are close, THEN pick option that serves highest quality where they differ.
IF option breaks a Never line, THEN reject option. No trade.
IF option gives lower quality at cost of higher quality, THEN reject option.
IF ranking does not divide options, THEN pick simpler option.
```

- [ ] State in plan or report: which quality decided, in one line.
- [ ] Advisor + auditor: finding that names no quality and no Never line = opinion. Mark it NIT or drop it.
- [ ] Quality that priority line does not name gets no special work. No cache, no pool, no generic "for later".

## Never lines become guards

- [ ] Each Never line: one fitness function, or one reason in plan why not yet.
- [ ] Each Never line: HALT class in `rex-code-failure-policy`.
- [ ] Settled decision that is not ranking does not go here. It goes at place it matters: fitness function or permanent warning comment.

## Checklist

- [ ] Priority block in crate root, max 8 lines
- [ ] `Priorities, in order:` with 2 to 4 ranked qualities, no ties
- [ ] Each Never line is testable
- [ ] No CONTEXT.md, no ADR made for this
- [ ] Missing block + ranking decision → asked caller, did not guess
- [ ] Plan names quality that decided each trade-off
