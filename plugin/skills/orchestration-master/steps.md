# Steps and requests

Loop steps 6 to 8 of `orchestration-master/SKILL.md`. Uses its words and rules.

## Steps

- [ ] Small goal = one step.
- [ ] Each step: one outcome, one area of code, acceptance = command or test + expected result.
- [ ] Divide step with two outcomes in areas that do not touch.
- [ ] Join two steps that change same files.
- [ ] Contract that 2 steps share: stable first, in own step.
- [ ] Parallel only IF steps own different files and state, and use no output of each other. ELSE sequence.
- [ ] Steps + fix jobs ≤ `max_jobs`. Consequence of each ≤ `consequence_ceiling`.
- [ ] `merge=never`: later job does not see work of earlier job. Cut steps that do not build on each other, OR one step.
- [ ] Fix job: done check fails, OR builder reports defect it did not fix. Request names failed line or defect.
- [ ] Plan job: step too unclear to build. Request asks for plan file, no code.

## Request

Request = whole brief of builder, for agent with no context. Full sentences, this form:

```
Outcome: <one deliverable, and the part of the goal it serves>.
Acceptance: <each command or test the builder runs, and the result that passes>.
Scope: <files and areas the builder owns, and what it leaves unchanged>.
Context: <facts of the code the step needs. For each earlier step it builds on: job ID, merge commit, report file>.
Standing rules: <each rule of the charter that touches the step, in the words of the charter>.
```

- [ ] Never write how to use Git or merges: builder skill + mod do that.
- [ ] No models for agents of builder, unless charter names them. No rule that charter does not give.

| Argument | Rule |
| --- | --- |
| `type` | Classify step as `orchestration-builder/classify.md` classifies request. |
| `size` | Same file. |
| `langs` | Languages step writes, ELSE `[]`. |
| `consequence` | True damage of mistake. Never lower it to get merge. |
| `depends_on` | Jobs whose delivered work step uses. Mod refuses before they deliver. |
| `merge_after` | Jobs that must merge before this one. |
| `budget_usd` | Only IF charter gives spend. Max `per_job_usd`. |
| `ratings` | Leave out: ## Sub-agent models. |
| `repo` | Only IF charter names repo of policy. |

## Sub-agent models

- [ ] Default ratings for most steps.
- [ ] Top tier only for named risk: security boundary, data that cannot come back, money, or work two builders failed. Record risk with `rex_note`.

## Refusals

| `rex_build` refusal | Then |
| --- | --- |
| `max_parallel_builders` | End turn. Next builder end wakes you. |
| `is not delivered` | Start step after hand-off of that job. |
| `plan usage at` | End turn. Mod wakes you later. |
| `master tree <id> has spent` | Tell person commands of refusal. End turn. |
| `consequence_ceiling` | Cut step smaller, OR ## Questions with category `scope`. |
| `max_jobs` | Join remaining steps, OR mark them `not built`. |
| `policy changed` | Tell person: confirm with `/rex policy confirm`. End turn. |
| `the first commit of`, `is not a git repo` | Tell person commands of refusal. End turn. |
| other | Call `rex_build` once more. IF refused again, THEN `rex_note` text, mark step `not built`. |

- [ ] Never call `rex_build` again for refusal whose cause stays.
