---
name: orchestration-selection
description: >
  Stage 3 of the orchestration builder. It reuses or builds an orchestrator
  skill for the work breakdown, gives each task a role, defines the roles, and
  rates the seven inputs that select a model for each role.
when_to_use: >
  Use when the decision agent of `orchestration-builder` reaches stage 3.
disable-model-invocation: true
user-invocable: false
---

# Orchestration Selection

Select or build orchestrator skill that suits work breakdown. Give each role its agent.

```
1. Reuse. IF orchestrator skill in recall output suits work breakdown, THEN reuse it, do steps 3 + 4, give output.
2. ELSE select method: read `.claude/skills/orchestration-selection/build-new.md`.
3. Define roles, tasks + skills of each role.
4. For each role: rate inputs, run `rex oms`, read `demand` + `conflict`. OR give role session agent.
5. New only: write orchestrator skill + lead skills (`build-new.md`). Run `rex skills check` on its folder.
6. Give output. Builder records it with `rex job plan`.
```

| File | Read when |
| --- | --- |
| `.claude/skills/orchestration-selection/build-new.md` | Step 1 finds none to reuse. Steps 2 + 5. |
| `.claude/skills/orchestration-selection/plan-jobs.md` | Job = plan job. |
| `.claude/skills/orchestration-selection/model-function.md` | You must know why `rex oms` gave its output. |
| `.claude/skills/orchestration-selection/limits.md` | Orchestration has a lead, OR orchestrator skill runs below level 1. |
## Words

- Session: Claude Code conversation that loaded builder. It has one session ID.
- Builder: Agent that obeys `orchestration-builder`.
- Decision agent: Sub-agent of builder that does stages 1 to 3. Its brief gives input.
- Job: One request that SuperRex handles. It has one job ID.
- Classification: Four fields of job: work type, size, consequence, languages.
- Preference: Duration, budget, or autonomy that user stated in request.
- Skill list: Skills this job uses. Output of stage 1.
- Nuanced skill: Skill for something particular to repo or to kind of work, that no other skill covers.
- Gap skill: Nuanced skill for one language. It covers what Rust gives by default and that language does not.
- Task: One outcome of job, with evidence that shows it is complete.
- Work breakdown: Tasks of job, their dependencies, and groups that run in parallel. Output of stage 2.
- Role: Named part in orchestrator skill. Example: `engineer`.
- Grid agent: One of 20 agent definitions in `.claude/agents/`. Name: `model-<tier>-effort-<level>`.
- Session agent: `general-purpose` sub-agent that you start with no model. It runs at model + effort of session.
- `HOME_DIR`: Absolute path of main checkout root of home repo. Home repo = repo that holds store (`.rex/`) and skills (`.claude/skills/`) of session. IF your shell is in Git worktree of home repo, THEN `HOME_DIR` = parent folder of `git rev-parse --path-format=absolute --git-common-dir`.
- Role table: For one job: tasks, skills, inputs, and agent of each role. Output of stage 3.
- Orchestrator skill: Reusable file. It gives roles, order, skills of each role, and checks. Not about one task.
- Brief: Prompt that sub-agent gets.
- Orchestrator, lead, worker: Level 1 = session. Level 2 = sub-agent of session. Level 3 = sub-agent of lead.
- Paths: each path in this skill starts at `HOME_DIR`.

## Input

Work breakdown, skill list, classification, preferences, recall output. Level at which orchestrator skill runs.

## Step 1 — Reuse

Reuse before you build.

1. Candidates = `orchestrator` lines and `unused` lines of recall output.
2. `orchestrator` line: orchestrator skill that similar jobs used, with results of those jobs. `unused` line: orchestrator skill whose newest version never ran.
3. Read `SKILL.md` of each candidate IN FULL. Its path is in `path` of its line.
4. IF one suits work breakdown, THEN reuse it. You decide: weigh results of its jobs against preferences.
5. IF none suits, THEN build new one: steps 2, 3, 5. In both cases, do steps 3 + 4 for each role.

- [ ] Reused file gives roles + skills of each role. Add skills from skill list that role uses.
- [ ] Add no rule to reused file. Rule enters orchestrator skill only through `rex-feedback`.

## Step 3 — Roles

Roles, not models. Orchestrator skill says "start agent for role `engineer`". Role table says which agent that is.

For each role, define:

- [ ] Name: lower-case letters, digits, and `-` only. Examples: `engineer`, `reviewer`, `verifier`.
- [ ] Job: one primary job.
- [ ] Tasks: tasks of work breakdown that role does. Each task gets one role that does it. Lead: tasks that it leads. Role that only checks work of other roles (example: `reviewer`, `verifier`): `—`.
- [ ] Skills of role that does tasks: union of Skills of its tasks.
- [ ] Skills of lead, and of role with `—`: skills it reads for its own job, from skill list: core rules + skills for its job. Gap skill of each language that role writes.
- [ ] Skills that orchestrator skill gives a role: add them to Skills of each task of that role, and to skill list with their `Path` (`path=` line of `rex skills list`).
- [ ] Reuse: roles come from reused file. Give tasks + skills the same way.

Skills are files. Agent first reads, IN FULL, `SKILL.md` of each skill of its task and each sibling file that its task row names (lead, or role with `—`: skills of its role). It also reads a sibling file when a `SKILL.md` sends it there. Its brief gives absolute paths.

## Step 4 — Model for each role

Seven inputs. Each value = `low`, `medium`, `high`, or `none`.

| Input (flag) | Belongs to | `low` | `medium` | `high` |
| --- | --- | --- | --- | --- |
| `--duration` | Job | Finish quickly. Also `fast`. | Moderate wait is acceptable. | Longer run is acceptable. Also `long`. |
| `--budget` | Job | Keep spend low. | Moderate spend. | Higher spend when useful. |
| `--autonomy` | Job | Frequent guidance is acceptable. | Occasional help is acceptable. | Work alone. Ask only when necessary. |
| `--intelligence` | Role | Basic capability. Routine task. | Reasoning + judgment. | Advanced reasoning. Difficult task. |
| `--reliability` | Role | Occasional failures are acceptable. | Failures are uncommon. | Failures are very rare. |
| `--consequence` | Role | Mistake is minor, easy to reverse. | Mistake causes meaningful rework or disruption. | Mistake causes serious damage or loss, hard to reverse. |
| `--verification` | Role | Correctness is hard to examine. | Some review or test is necessary. | Quick, reliable checks exist. |

- [ ] `none` = input is not relevant. Function ignores it. Leave flag out: same as `none`.
- [ ] Job inputs: give `rex oms` same job inputs that `rex job create` got. These are only preferences that user stated.
- [ ] Role inputs: rate each role. Role consequence starts at consequence in classification. It can be lower for role that cannot cause that harm.
- [ ] Higher budget or longer duration permits more spend or time. It does not require it.

Who selects agent:

- [ ] Job with plan: `rex job plan` selects agent of each role from same inputs. Builder starts each role with agent that `rex job plan` printed.
- [ ] Run `rex oms` for each role in this step. Read `demand` + `conflict` before you fix roles: job gets one plan only.
- [ ] Builder inside open job has no plan. It starts each role with agent of `agent` line of `rex oms`.

Run:

```
rex oms [--duration <v>] [--budget <v>] [--intelligence <v>] [--autonomy <v>] \
        [--reliability <v>] [--consequence <v>] [--verification <v>]
```

Output = six `key=value` lines, in fixed order. Write `agent` in role table.

| Key | Value |
| --- | --- |
| `agent` | Grid agent to use. |
| `demand` | Pair that role needed before caps. Same as `agent` IF no cap applied. |
| `rules` | Rules that fired, comma-separated. |
| `conflict` | `none`, `budget`, `duration`, or `budget,duration`. |
| `inputs` | Seven inputs as seven characters: duration, budget, intelligence, autonomy, reliability, consequence, verification. `L`, `M`, `H`, or `-` for `none`. |
| `version` | Function version, whole number. |

Grid: tiers `low`, `medium`, `high`, `xhigh`. Levels `low`, `medium`, `high`, `xhigh`, `max`. Grid agent runs at model + effort of its definition.

- [ ] Orchestrator = session. It runs at model + effort of session. Run no `rex oms` for it.
- [ ] Role that must run as session does: run no `rex oms`. Write `session agent` in `Agent` column, and `—` in each input column. In plan file, it is `"session": true`.

## Output

Paired challenger checks both first (`orchestration-decision-challenge`). Then builder records both with `rex job plan`.

```
ORCHESTRATOR: <name> | <path> | new OR reused
ROLE TABLE:
| Role | Tasks | Skills | Intelligence | Reliability | Consequence | Verification | Agent |
| <role> | <task IDs> OR — | <skill names> | <v> | <v> | <v> | <v> | <grid agent> OR session agent |
```

Role with `session agent` has `—` in each input.

- [ ] Give work breakdown again, with Skills as step 3 changed them.
