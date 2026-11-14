# Decision Challenge

Decision agent decides. Paired challenger attacks decision. Decision agent examines each finding, corrects decision. Builder records plan. Work starts only after this.

```
Builder:
1. Select challenger agent. Write brief.
2. Start challenger. Wait for findings.
3. Send path of findings file to decision agent.

Challenger:
1. Read input + repo.
2. Do each check of `## Checks`.
3. Write findings to output file. Change nothing else.

Decision agent (`correct.md`):
1. Examine each finding: CONFIRMED, REJECTED, or PARTIAL.
2. Correct decision for each CONFIRMED + PARTIAL finding.
3. Add DECISION CHECK to work breakdown. Give corrected decision block to builder. Builder records plan.
```

| File | Read when |
| --- | --- |
| `orchestration-builder/stages/challenge/start.md` | Builder: before it starts challenger. Steps 1 + 2 of builder. |
| `orchestration-builder/stages/challenge/correct.md` | Decision agent: when builder sends findings. Steps of decision agent. |

## Words

- Builder: Agent that obeys `orchestration-builder`.
- Decision agent: Sub-agent of builder that does stages 1 to 3. It made decision. It examines findings + corrects decision.
- Decision: Output of three stages: skill list, work breakdown, orchestrator skill, role table.
- Assignment table: `orchestration-builder/stages/1-skills/assign.md`. One home of which skills each kind of work gets. Data of plugin. `rex_assign` applies it.
- Recorded answer: answer of decision agent to one question of `rex_assign`, with its reason. Mod keeps it in record of job.
- Decision line: one line of `rex_status` for job that starts `decision.`. Form: `decision.<task>.<item>=<source>: <placement or answer>; <reason>`. Item = skill, `skill/file.md`, `<row>.<criterion>` of answer, or `grid_agent` of role.
- Decision block: Text form of decision. Decision agent gives it. Builder copies it into brief of challenger.
- Challenger: Fresh sub-agent that checks decision. It made no part of decision. Role `decision-challenger`.
- Skill list: Skills this job uses. Output of stage 1.
- Task: One outcome of job. Its row in work breakdown gives its `Skills`.
- Role table: For one job: tasks, skills, inputs, and agent of each role. Output of stage 3.
- Grid agent: One of 20 agent types of plugin `super-rex`. Name: `super-rex:model-<tier>-effort-<level>`. It has no Skill tool. `rex oms` prints it with no `super-rex:` prefix.
- Finding: One claim of challenger: severity, part of decision, claim, evidence, correction.
- Brief: first line `Task: <role or task ID>`, then five sections (`orchestration-builder/stages/3-orchestration/briefs.md`). Mod adds tags, skills, tier text, and output file at spawn.
- Paths: each path that is not a skill file starts at `HOME_DIR`, main checkout root of home repo. Skill file `<skill>/<file>.md`: read it with `rex_skill`.

## Rules

- [ ] Challenge runs for each job, before `rex_job_plan`. Job gets one plan only, so correct decision first.
- [ ] Challenger made no part of decision. Never builder, never decision agent.
- [ ] Challenger only reads. It changes no file except its output file, and runs no command that writes.
- [ ] One round. Builder does not send decision back for second check.
- [ ] Classification stays. Builder and decision agent never edit it. Challenger can name doubt about it.
- [ ] Decision agent examines each finding: `orchestration-builder/stages/challenge/correct.md`.

## Checks

Challenger. Do each check. Read repo where check needs it: `rex codebase --only code --with-context`.

Skills (most important). Read assignment table IN FULL first. Then call `rex_status` with `job`. Read each decision line.

- [ ] Each task of work breakdown: IF it has no decision line, THEN finding, severity `BLOCK`: decision did not call `rex_assign` for it.
- [ ] Each recorded answer `yes` OR `no`: read task in work breakdown + repo. IF reason does not hold for that task, THEN finding. Correction: one row of `OVERRIDES` with reason.
- [ ] Each answer `no` to `each-step` (`Open when`): IF each step of task needs that skill, THEN finding, severity `MINOR`.
- [ ] `Skills` + `Open when` of each task in decision block: IF they differ from its decision lines + `OVERRIDES`, THEN finding.
- [ ] Code gives rule-required skills, cap, and placement. Do not check them.
- [ ] Skill in skill list that table does not give: IF `Why` gives no reason that holds, OR no row of `OVERRIDES` gives it to a task, THEN finding.
- [ ] Read each block of `rex skills list`, and each skill of skill library that assignment table names. For each skill whose `when_to_use` fits request, work type, languages, or repo: IF it is not in skill list, THEN finding.
- [ ] Each skill in skill list: IF no task and no role gets it in `Skills` or `Open when`, THEN finding. Included is not sufficient. An agent must be able to read it.
- [ ] Each task: IF it needs a skill that its `Skills` and `Open when` do not give, THEN finding. Example: build task gets `rex-code-tdd`, test task does not.
- [ ] Each lead, and each role with no task: IF its job needs a skill that it does not get, THEN finding.
- [ ] Language other than `rust`: IF skill list has no gap skill `rex-gap-<language>` of it (`## Output` of `orchestration-builder/stages/1-skills/stage.md`), THEN finding. IF repo already has a skill that covers gaps of that language under another name, THEN finding, severity `MINOR`: rename it to `rex-gap-<language>`.
- [ ] Lesson of recall output that names a skill: IF decision ignores it, THEN finding.
- [ ] Skill in list that job does not need, OR skill of a task that task does not need: finding, severity `MINOR`. Each loaded skill costs context.
- [ ] Item in `Skills` that agent uses only in one case of its task: finding, severity `MINOR`. Correction: answer `each-step` was wrong, override it.

Work breakdown:

- [ ] Missing task, OR task with no evidence, OR evidence that does not prove outcome.
- [ ] Wrong dependency. Tasks in parallel that change same files or state.
- [ ] Group of 2 or more tasks with no integration owner.

Orchestrator + roles:

- [ ] Method does not fit breakdown. Example: fan-out for tightly coupled work.
- [ ] Agent count > max of job size (`## Step 3 — Roles` of `orchestration-builder/stages/3-orchestration/stage.md`). Count = decision agent + challenger + each agent of role table. Severity `MAJOR`. IF `AGENTS` line gives other count, THEN also finding.
- [ ] Role with consequence `high` and verification `low`, with no reviewer or verifier role.
- [ ] Role consequence below classification, with no reason.
- [ ] Rating that does not fit task. Example: intelligence `low` for design work.
- [ ] Plan job: no plan writer, no `plan-reviewer`, or no revision step.
- [ ] Rule of builder broken: work on main, no merge agreement, preference that user did not state.

## Output

Challenger gives findings only. No praise, no new decision. Write them to output file that brief names. Final message: path + count of findings for each severity.

```
FINDINGS: <n>
| # | Severity | Part | Claim | Evidence | Correction |
| 1 | BLOCK / MAJOR / MINOR | skills / breakdown / orchestrator / roles | <one sentence> | <file, skill block, or line of decision> | <exact change> |
```

- [ ] `BLOCK`: work goes wrong IF decision agent does not correct it.
- [ ] `MAJOR`: work is weaker or costs more.
- [ ] `MINOR`: small gain.
- [ ] No finding: write `FINDINGS: 0`.
