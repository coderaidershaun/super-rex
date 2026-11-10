---
name: rex-code-spike
description: >
  Throwaway code in a scratch folder that answers one yes/no question before
  real code depends on it. Never enters the source tree.
when_to_use: >
  Use when plan or sketch lists an unknown, an engineer doubts an approach,
  crate, or outside service, or user says "spike this", "prove it works first",
  "quick experiment", "scratch test", "will this even work".
disable-model-invocation: false
user-invocable: true
---

# Spike — Hard Rules

Not sure idea works? Prove it before you build on it. Then throw proof away.

Sister skills:

- `rex-code-design-sketch` — source of unknowns.
- `rex-code-external-libraries` — crate rules still apply in spike.
- `rex-code-tests-integration-testing` — real-funds rules (`.claude/skills/rex-code-tests-integration-testing/real-funds.md`) still apply in spike.

## Words

| Term | Meaning |
| --- | --- |
| Unknown | Fact that plan needs and nobody has proven. |
| Spike | Smallest code that proves or disproves one unknown. |
| Scratch dir | `<tmp dir>/spike/<slug>/`. Not in repo. |
| Answer | YES, NO, or YES-IF with condition. |

## When

```
IF plan lists unknown, THEN spike before engineer builds on it.
IF you can find answer in code or official docs, THEN read. No spike.
IF question is "which design is better", THEN no spike. That is review, not unknown.
IF nothing is unknown, THEN no spike.
```

## Rules

- [ ] One spike = one yes/no question. Write question before code.
- [ ] Write what result means YES and what result means NO, before run.
- [ ] Smallest code that answers. One file when possible.
- [ ] Limit: 30 minutes or 3 tries. IF no answer, THEN report "no answer" + what blocked.
- [ ] Skills for quality do not apply: no tests, no error types, no comments, `unwrap` OK.
- [ ] Spike reads repo. Spike never edits repo.
- [ ] Use sandbox or test endpoint for outside service.
- [ ] Measure, do not estimate. IF question is about speed or size, THEN print number.

CAUTION: Do not run a spike against real funds or production data. A spike has no safety checks.

## Good question / bad question

| Bad | Good |
| --- | --- |
| "Try the queue crate" | "Can one consumer drain 100k messages per second from this queue on one thread?" |
| "See how the API behaves" | "Does the API send a full snapshot after resubscribe?" |
| "Prototype the parser" | "Can the parser read the 3 sample files with no custom grammar?" |

## Procedure

1. Write question + YES result + NO result in `<scratch dir>/question.md`.
2. Write spike in scratch dir.
3. Run. Save output.
4. Write answer + evidence.
5. IF answer = NO or YES-IF, THEN tell lead. Lead changes plan before engineer starts.
6. Delete scratch dir.

CAUTION: Delete only the scratch dir of this spike. Make sure the path is inside the tmp dir before you delete it.

## Never promote spike

- [ ] Do not copy spike code into `src/`. Engineer writes real code new, with TDD.
- [ ] Do not keep spike as test, example, or bench.
- [ ] Only answer + numbers go on. They go in plan.

Why: code that answers question has wrong shape for production. Kept spike = first bandage.

## Report

```
SPIKE: <question>
ANSWER: YES | NO | YES-IF <condition> | NO ANSWER
EVIDENCE: <output, number, or doc reference>
EFFECT ON PLAN: none | <what must change>
SCRATCH DIR: deleted
```

## Checklist

- [ ] Question is yes/no, written before code
- [ ] YES + NO results defined before run
- [ ] Scratch dir outside repo
- [ ] No real funds, no production data
- [ ] Answer has evidence
- [ ] Plan updated IF answer changed it
- [ ] Scratch dir deleted, no spike code in repo
