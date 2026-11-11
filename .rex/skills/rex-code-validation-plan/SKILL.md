---
name: rex-code-validation-plan
description: >
  Hard rules for expected traces of flows and failures, written in plan.md
  before code.
when_to_use: >
  Use when plan has sketch or failure table and no code yet, or flow or RESET,
  RETRY, HALT row lacks scenario, or user says "validation plan", "expected
  trace", "what happens step by step", "prove the order", "failure scenarios",
  "how do we know it works".
disable-model-invocation: false
user-invocable: true
---

# Validation Plan — Hard Rules

Write what a test must see, step by step, before code. No expected trace = no meaning for "works".
Sketch says which boxes exist. Failure table says what system does after error. This skill says which events occur, in which order.

Sister skills:

- `rex-code-design-sketch` — source of each flow where order or time matters.
- `rex-code-failure-policy` — source of each failure scenario: each RESET, RETRY, or HALT row of failure table (`.rex/skills/rex-code-failure-policy/failure-table.md`).
- `rex-code-spike` — proves each unknown that a trace finds.
- `rex-code-tdd` — engineer turns each scenario into one test, one at a time.
- `rex-code-tests-contract-seams`, `rex-code-tests-integration-testing`, `rex-code-tests-fitness-functions` — how test of each lane is written.

## Words

| Term | Meaning |
| --- | --- |
| Validation plan | Scenario list + one expected trace for each scenario. Part of `plan.md`. |
| Box, arrow, sketch list | As in `rex-code-design-sketch`. Sketch list = flows that its gate puts on list for this skill. |
| Failure class, supervisor, trusted source | As in `rex-code-failure-policy`. |
| Failure table, guard | Table in `plan.md` + its Guard column. Format: `.rex/skills/rex-code-failure-policy/failure-table.md`. |
| Flow | Data that moves through boxes, from one trigger to one end state. |
| Scenario | One flow or one failure, from one start state and one trigger. |
| Trigger | Input or event that starts scenario: message, call, timeout. |
| Step | One event that a test can see on an arrow or at public interface of a box. |
| Expected trace | Start state, trigger, numbered steps in order, end state. End state = state + output after last step, plus what must not occur. |
| Lane | Test type that proves scenario: contract, integration, or fitness. |

## When

```
IF sketch lists flow where order or time matters, THEN write scenario for it.
IF failure table has RESET, RETRY, or HALT row, THEN write scenario for it.
IF row is REJECT, THEN no scenario. Guard in failure table is sufficient.
```

## Where plan lives

- [ ] Lead writes validation plan in `plan.md`, after sketch and failure table.
- [ ] Never make new file in repo for it. No docs folder, no test-plan doc (`rex-code-priorities`).
- [ ] After build, tests carry traces. Test name = scenario name.

## Scenario list

| ID | Scenario | Source | Lane | Test |
| --- | --- | --- | --- | --- |
| V1 | Snapshot comes before first update | sketch: `feed → book` | contract | `snapshot_arrives_before_any_update` |
| V2 | Sequence gap resets book | failure: `SyncError::SequenceGap` | contract | `sequence_gap_resets_book_from_snapshot` |
| V3 | Timeout: success on third try | failure: `SyncError::Timeout` | integration | `timeout_retries_then_succeeds` |
| V4 | Timeout: limit reached | failure: `SyncError::Timeout` | integration | `timeout_limit_reached_resets_feed` |

- [ ] One row for each flow on sketch list. One row for each RESET, RETRY, or HALT row.
- [ ] Source names arrow of sketch or error variant. Row with no source = scope that task did not ask for. Remove it.
- [ ] Each row names lane + test name before code.

## Trace rules

- [ ] Start state + trigger come first, with real values.
- [ ] Each step = `<box>: <event> (<values>)`. Box name from module table. Real values, not "some data".
- [ ] Step is event a test can see through public interface. Not inner field, not private function.
- [ ] Order of steps is the claim. IF two steps can occur in either order, THEN write "3a + 3b, any order".
- [ ] IF time matters, THEN step gives limit: `within 2 s`, `after 200 ms or more`.
- [ ] End state names what must not occur: no output, no second send, no restart.
- [ ] Max 10 steps. IF more, THEN divide scenario.
- [ ] IF you cannot write a step because a fact is unknown, THEN add yes/no question to Unknowns of sketch. Spike before code.
- [ ] Test asserts whole event list. Exact `assert_eq!` proves order and proves no extra event.

IF you write your first trace, or are not sure of trace shape, THEN read `.rex/skills/rex-code-validation-plan/examples.md`: bad trace, good trace, and its test.

## Flows where order or time matters

- [ ] Each mark in diagram (code waits, or work crosses thread, process, or network) = flow on list. IF list does not have it, THEN add it.
- [ ] IF two flows can interleave, THEN write scenario for order that does damage. Not only order that works.

## Failure scenarios

Trace shows supervisor action of failure table as steps. Each class rule of `rex-code-failure-policy` = one step or one end-state line.

| Class | Trace must show |
| --- | --- |
| HALT | New work stops first. Then safe stop of open work. State kept. One loud line with values seen. End state: no restart. |
| RESET | Only bad part resets. Rebuild from trusted source. End state: no output from part between reset and ready. Second scenario: rebuild fails 2 times → HALT. |
| RETRY | Each try + its delay. Delay grows. Receiver sees one action. Second scenario: limit reached → class of next row up. |

CAUTION: Do not change an expected trace to make a test pass. The trace defines correct behavior. A changed trace hides a defect. Send the trace to the lead, who changes the plan first.

## Pick lane

| Scenario touches | Lane | Skill |
| --- | --- | --- |
| Two or more boxes, all in process | contract | `rex-code-tests-contract-seams` |
| Outside world, real time, or wire format | integration | `rex-code-tests-integration-testing` |
| Invariant over many inputs | fitness | `rex-code-tests-fitness-functions` |

- [ ] Never unit lane only. Unit test = scaffold. Engineer deletes it after green (`rex-code-tests-unit-testing`).

## Procedure

```
FOR EACH flow on sketch list + each RESET, RETRY, or HALT row of failure table:
    Write scenario row, expected trace (Trace rules), lane, test name.
Advisor reads validation plan with sketch, before code. Missing scenario = finding.
Engineer writes one test for each scenario, one at a time (rex-code-tdd).
```

## Report

```
VALIDATION PLAN: <n> scenarios in plan.md
FLOWS: <n> of <n> on sketch list have trace
FAILURE ROWS: <n> of <n> RESET, RETRY, HALT rows have trace
LANES: contract <n>, integration <n>, fitness <n>
UNKNOWNS FOUND: none | <yes/no questions sent to spike>
```

## Checklist

- [ ] Validation plan in `plan.md`. No new file in repo
- [ ] Each flow on sketch list + each RESET, RETRY, HALT row has scenario. RETRY + RESET rows also have limit scenario
- [ ] Each step: box, event, real values, test can see it. End state names what must not occur
- [ ] Each scenario has lane + test name. No unit-only scenario
- [ ] Unknowns sent to spike. Advisor read plan. Both before code
