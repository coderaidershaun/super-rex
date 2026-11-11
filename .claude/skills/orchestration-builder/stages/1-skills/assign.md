# Assignment table

One home of which skills each task gets. Stage 1 starts skill list from it. Stage 2 gives each task skills + sibling files of each row that fits task. Stage 3 gives each lead, and each role with no task, rows that fit its job. Challenger checks decision against it.

- [ ] Row fits task IF task does kind of work of that row. One task can fit many rows.
- [ ] Skill that table does not give: one-line reason in `Why` of SKILL LIST, starts `Not in table:`.
- [ ] Skill that table gives and job does not get: one `Left out` line below SKILL LIST.
- [ ] Sibling file: row gives its path `.rex/skills/<skill>/<file>.md`. Write it `<skill>/<file>.md` in `Skills` or `Open when` of task, not in SKILL LIST. Its skill goes in SKILL LIST.
- [ ] Row that task does in each step: its skills go in `Skills`. Row that task reaches only in one case: `Open when`, with that case.
- [ ] Sibling file after a case word (`Move:`, `Audit:`, `Real funds:`, `Plan for such feature:`): IF task is that case, THEN `Skills`. ELSE `Open when`, with that case.

## Each task that writes code

Each language. Rulebook text is about Rust. Agent applies intent of each rule in idiom of its language.

| Skill | Why |
| --- | --- |
| `rex-code-philosophy` | Complexity, habitability. |
| `rex-code-commenting` | Comments, and module comment of each file. |
| `rex-code-ergonomics` | Names, types, signatures. |
| `rex-code-error-writing` | Typed errors that carry context. |

- [ ] IF language is not `rust`, THEN add gap skill `rex-gap-<language>` of that language (`## Gap skill` of `.claude/skills/orchestration-builder/stages/1-skills/stage.md`).

## By kind of work

| Kind of work | Skills + sibling files |
| --- | --- |
| Builds feature or fixes bug, with tests | `rex-code-tdd` + lane skill of each test kind that task writes (4 rows below) |
| Unit test: inline scaffold for pure function | `rex-code-tests-unit-testing` |
| Test of interface between two modules | `rex-code-tests-contract-seams` |
| Invariant that must always hold, property test | `rex-code-tests-fitness-functions` |
| Test against real outside world: API, exchange, database, WebSocket | `rex-code-tests-integration-testing`. Real funds: + `.rex/skills/rex-code-tests-integration-testing/real-funds.md` |
| Test crosses system boundary: outside API, database, time, randomness, file system | + `.rex/skills/rex-code-tdd/mocking.md` |
| Creates, splits, or moves file, module, or folder | `rex-code-folder-management`. Move: + `.rex/skills/rex-code-folder-management/move.md` |
| Adds or upgrades dependency | `rex-code-external-libraries` |
| Outside input, long-running state, retry, reconnect, main loop | `rex-code-failure-policy`. Plan for such feature: + `.rex/skills/rex-code-failure-policy/failure-table.md` |
| New system, feature, or module: design before code | `rex-code-design-sketch` + `rex-code-validation-plan` |
| Approach, crate, or outside service not proven | `rex-code-spike` |
| Refactor of unfamiliar or old code, OR code audit | `rex-code-smells`. Audit: + `.rex/skills/rex-code-folder-management/audit.md` + `.rex/skills/rex-code-failure-policy/audit.md` |
| Refactor candidates, module grouping, architecture friction | `rex-code-improve-codebase-architecture` |
| Design, plan, or review decision that depends on ranking of qualities | `rex-code-priorities` |
| Hotpath latency of ns to tens of µs, on Linux host you control | `rex-code-latency` + `.rex/skills/rex-code-latency/measure.md` + first file of row of `## Router` in its `SKILL.md` that fits problem |
| Review of branch or pull request by agent that wrote none of it | `rex-pr-review` |
| Diagram in plan or document | `rex-mermaid-diagrams` |
| Writes skill, prompt, or agent instructions | `rex-skill-language` |
| Plan job: review of plan (role `plan-reviewer`) | `rex-codex-adversarial-review`. Code plan, writer of plan: + `rex-code-design-sketch` + `rex-code-validation-plan` |

## Not in table

A person starts these, OR builder reads their files by path: `orchestration-builder`, `rex-deliver`, `rex-feedback`, `rex-grill-me`. No task gets them.

`rex-rules`: router for session with no builder. It sends that session to this table. No task gets it.
