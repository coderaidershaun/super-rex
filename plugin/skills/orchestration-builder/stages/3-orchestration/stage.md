# Stage 3 — Orchestration

Select or build orchestrator skill that suits work breakdown. Give each role its agent.

```
1. Reuse. IF orchestrator skill in recall output suits work breakdown, THEN reuse it, do steps 3 + 4, give output.
2. ELSE select method: read `orchestration-builder/stages/3-orchestration/build-new.md`.
3. Define roles, tasks + skills of each role.
4. For each role: rate inputs, run `rex oms`, read `demand` + `conflict`.
5. New only: write orchestrator skill + lead skills (`build-new.md`). Run `rex skills check` on its folder.
6. Give output. Builder records it with `rex_job_plan`.
```

| File | Read when |
| --- | --- |
| `orchestration-builder/stages/3-orchestration/build-new.md` | Step 1 finds none to reuse. Steps 2 + 5. |
| `orchestration-builder/stages/3-orchestration/plan-jobs.md` | Job = plan job. |
| `orchestration-builder/stages/3-orchestration/model-function.md` | You must know why `rex oms` gave its output. |
| `orchestration-builder/stages/3-orchestration/limits.md` | Orchestration has a lead. |
| `orchestration-builder/stages/3-orchestration/role-skills.md` | Step 3. Skills + Open when of each role. |

## Words

- Session: Claude Code conversation that runs builder.
- Builder: Agent that obeys `orchestration-builder`.
- Decision agent: Sub-agent of builder that does stages 1 to 3. Its brief gives input.
- Classification: Four fields of job: work type, size, consequence, languages.
- Preference: Duration, budget, or autonomy that user stated in request.
- Skill list: Skills this job uses. Output of stage 1.
- Gap skill: Skill for one language. It covers what Rust gives by default and that language does not.
- Assignment table: `orchestration-builder/stages/1-skills/assign.md`. Which skills each kind of work gets.
- Task: One outcome of job, with evidence that shows it is complete.
- Work breakdown: Tasks of job, their dependencies, and groups that run in parallel. Output of stage 2.
- Role: Named part in orchestrator skill. Example: `engineer`.
- Grid agent: One of 20 agent types of plugin `super-rex`. Name: `super-rex:model-<tier>-effort-<level>`. It has no Skill tool.
- `HOME_DIR`: Absolute path of main checkout root of home repo. Home repo = repo that holds store (`.rex/`) and repo skills (`.rex/skills/`) of job. IF your shell is in Git worktree of home repo, THEN `HOME_DIR` = parent folder of `git rev-parse --path-format=absolute --git-common-dir`.
- Skill file: file of a skill. Text names it `<skill>/<file>.md`, or `<skill>` for its `SKILL.md`. Read it with `rex_skill`: `name` = `<skill>`, `file` = `<file>.md`.
- Role table: For one job: tasks, skills, inputs, and agent of each role. Output of stage 3.
- Orchestrator skill: Reusable file. It gives roles, order, skills of each role, and checks. Not about one task.
- Brief: Prompt that sub-agent gets.
- Orchestrator, lead, worker: Level 1 = builder. Level 2 = sub-agent of builder. Level 3 = sub-agent of lead.
- Paths: each path that is not a skill file starts at `HOME_DIR`.

## Input

Work breakdown, skill list, classification, preferences, recall output.

## Step 1 — Reuse

Reuse before you build.

1. Candidates = `orchestrator` lines and `unused` lines of recall output.
2. `orchestrator` line: orchestrator skill that similar jobs used, with results of those jobs. `unused` line: orchestrator skill whose newest version never ran.
3. Read `SKILL.md` of each candidate IN FULL with Read tool. Its path is in `path` of its line.
4. IF one suits work breakdown, THEN reuse it. You decide: weigh results of its jobs against preferences.
   IF two candidates suit work AND have equal results, THEN prefer lower `cost_per_job_usd` of its `orchestrator` line.
5. IF none suits, THEN build new one: steps 2, 3, 5. In both cases, do steps 3 + 4 for each role.

- [ ] Reused file gives roles + skills of each role. Add skills from skill list that role uses.
- [ ] Add no rule to reused file. Rule enters orchestrator skill only through `rex-feedback`.

## Step 3 — Roles

Roles, not models. Orchestrator skill says "start agent for role `engineer`". Role table says which agent that is.

For each role, define:

- [ ] Name: lower-case letters, digits, and `-` only. Examples: `engineer`, `reviewer`, `verifier`.
- [ ] Job: one primary job.
- [ ] Tasks: tasks of work breakdown that role does. Each task gets one role that does it. Lead: tasks that it leads. Role that only checks work of other roles (example: `reviewer`, `verifier`): `—`.
- [ ] Skills + Open when of each role: obey `orchestration-builder/stages/3-orchestration/role-skills.md`.

Agent count = each sub-agent of job: decision agent, challenger, leads, workers, reviewers. Role that runs N times counts N.

| Size | Max agents |
| --- | --- |
| small | 5 |
| medium | 6 |
| large | 10 |

- [ ] IF count > max, THEN merge tasks into fewer roles. Each sub-agent builds its context from nothing: cache write = most of its cost.
- [ ] Small plan job fits: decision agent, challenger, plan writer, `plan-reviewer`, fallback reviewer = 5.
- [ ] Spend limit of job = (max agents + 2) x dollars for each agent of budget: `low` $1.50, `medium` $4.00, `high` $8.00. The 2 = main session. No stated budget = `medium`.
- [ ] SuperRex table `fixtures/shared/tables/limits.json` holds same caps + dollars. A test compares them. Change both together.

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
- [ ] Job inputs: give `rex oms` same job inputs that `rex_job_open` got in `prefs`. These are only preferences that user stated.
- [ ] Role inputs: rate each role. Role consequence starts at consequence in classification. It can be lower for role that cannot cause that harm.
- [ ] Higher budget or longer duration permits more spend or time. It does not require it.

Who selects agent:

- [ ] Job with plan: `rex_job_plan` selects agent of each role from same inputs. Builder starts each role with agent of `role.<name>=` line of `rex_job_plan`.
- [ ] Run `rex oms` for each role here. Read `demand` + `conflict` before you fix roles: job gets one plan only.

Run:

```
rex oms [--duration <v>] [--budget <v>] [--intelligence <v>] [--autonomy <v>] \
        [--reliability <v>] [--consequence <v>] [--verification <v>]
```

Output = six `key=value` lines, in fixed order. Write `agent` in role table.

| Key | Value |
| --- | --- |
| `agent` | Grid agent to use. `subagent_type` = `super-rex:` + this name. |
| `demand` | Pair that role needed before caps. Same as `agent` IF no cap applied. |
| `rules` | Rules that fired, comma-separated. |
| `conflict` | `none`, `budget`, `duration`, or `budget,duration`. |
| `inputs` | Seven inputs as seven characters: duration, budget, intelligence, autonomy, reliability, consequence, verification. `L`, `M`, `H`, or `-` for `none`. |
| `version` | Function version, whole number. |

- [ ] Orchestrator = builder. It runs at model + effort of its session, or of its grid agent. Run no `rex oms` for it.
- [ ] Each other role gets grid agent from `rex oms`. No role gets fixed agent.
- [ ] Tier `xhigh` costs most. `rex oms` gives it only IF consequence = `high`. Rate consequence `high` only IF mistake of role can do damage that revert does not undo.

## Output

Paired challenger checks both first (`orchestration-builder/stages/challenge/stage.md`). Then builder records both: `rex_job_plan`.

```
ORCHESTRATOR: <name> | <path> | new OR reused
AGENTS: <count> of max <max>
ROLE TABLE:
| Role | Tasks | Skills | Open when | Intelligence | Reliability | Consequence | Verification | Agent |
| <role> | <task IDs> OR — | <skill names> | <skill (case)> OR — | <v> | <v> | <v> | <v> | super-rex:<grid agent> |
```

- [ ] Give work breakdown again, with Skills + Open when as step 3 changed them.
