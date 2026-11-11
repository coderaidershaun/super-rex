# Stage 4 — Cause

Words + rules: `.claude/skills/rex-feedback/SKILL.md`. Evidence comes from Stage 3 there.

1. Find earliest point where work went wrong: request, brief, plan, role, run, or commit.
2. Find cause at that point. Use table. More than one cause can apply.
3. For each cause, write its evidence: run, field, or line from Stage 3.

| Cause | When |
| --- | --- |
| `human-error` | Request asked for wrong thing. Agents did what it said. |
| `missing-context` | Request did not give a fact. No agent can find that fact in repo. No agent asked for it. |
| `agent-skill` | Wrong skill, no skill, skill not opened at right time, brief did not name sibling file that holds rule, or a skill rule gave wrong action. |
| `agent-model-effort` | Model or effort too weak for role. Do sub-section below. |
| `agent-orchestration` | Shape of orchestrator wrong: roles, order, parallel split, checks, brief contents. |

- IF an agent can find that fact in repo, THEN cause = agent cause, not `missing-context`.
- Sibling file not named: evidence = `brief` of run. `skills_opened` shows that a skill was opened, not which of its files.

## Model or effort

Do checks in this order. Stop at first check that is true. Only check 4 points at mapping.

1. Chosen agent = `agent` of role. Its file `.claude/agents/<agent>.md` has keys `model` (for example `sonnet`) and `effort`. IF `agent_type` of a run ≠ `agent`, OR `model_ran` does not contain `model`, OR `effort_ran` ≠ `effort`, THEN environment overrode choice. Mapping not at fault.
   - IF `model_ran` or `effort_ran` is `null`, THEN write it under EMPTY FIELDS. Go to check 2.
2. IF `rules` has `C1`, `C2` or `C3`, THEN user's budget or duration capped choice. Lesson is about that preference.
3. IF `inputs` were wrong (for example, hard role rated low intelligence), THEN lesson belongs to builder rating. Home = `.claude/skills/orchestration-builder/stages/3-orchestration/stage.md`.
4. ELSE mapping is suspect. List missed jobs (below). DO read `roles[].rules` of next one, newest first, UNTIL all read OR 20 read. One job is not enough. IF same `rules` set fails on several jobs, THEN propose a mapping change in report. Only a `rex` release makes one.

```sh
rex jobs --json | jq -r '.jobs[] | select(.stored_status == "rejected" or .stored_status == "analyzed" or .stored_status == "failed") | .id'
rex job show "$ID" --json | jq -c '.roles[] | {role, rules}'
```
