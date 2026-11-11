# Decision Challenge

Decision agent decides. Paired challenger attacks decision. Decision agent examines each finding, corrects decision. Builder records plan. Work starts only after this.

```
Builder:
1. Select challenger agent. Write brief.
2. Start challenger. Wait for findings.
3. Send findings to decision agent.

Challenger:
1. Read input + repo.
2. Do each check of `## Checks`.
3. Give findings. Change nothing.

Decision agent (`correct.md`):
1. Examine each finding: CONFIRMED, REJECTED, or PARTIAL.
2. Correct decision for each CONFIRMED + PARTIAL finding.
3. Add DECISION CHECK to work breakdown. Give corrected decision block to builder. Builder records plan.
```

| File | Read when |
| --- | --- |
| `.claude/skills/orchestration-builder/stages/challenge/start.md` | Builder: before it starts challenger. Steps 1 + 2 of builder. |
| `.claude/skills/orchestration-builder/stages/challenge/correct.md` | Decision agent: when builder sends findings. Steps of decision agent. |

## Words

- Builder: Agent that obeys `orchestration-builder`.
- Decision agent: Sub-agent of builder that does stages 1 to 3. It made decision. It examines findings + corrects decision.
- Decision: Output of three stages: skill list, work breakdown, orchestrator skill, role table.
- Assignment table: `.claude/skills/orchestration-builder/stages/1-skills/assign.md`. One home of which skills each kind of work gets.
- Decision block: Text form of decision. Decision agent gives it. Builder copies it into brief of challenger.
- Challenger: Fresh sub-agent that checks decision. It made no part of decision. Role `decision-challenger`.
- Skill list: Skills this job uses. Output of stage 1.
- Task: One outcome of job. Its row in work breakdown gives its `Skills`.
- Role table: For one job: tasks, skills, inputs, and agent of each role. Output of stage 3.
- Grid agent: One of 20 agent definitions in `.claude/agents/`. Name: `model-<tier>-effort-<level>`. It has no Skill tool.
- Finding: One claim of challenger: severity, part of decision, claim, evidence, correction.
- Tag block: First lines of brief: `Rex-Job: <job ID>`, `Rex-Role: <role>`, then one blank line.
- Paths: each path in this skill starts at `HOME_DIR`, main checkout root of home repo.

## Rules

- [ ] Challenge runs for each job, before `rex job plan`. Job gets one plan only, so correct decision first.
- [ ] Challenger made no part of decision. Never builder, never decision agent.
- [ ] Challenger only reads. It changes no file and runs no command that writes.
- [ ] One round. Builder does not send decision back for second check.
- [ ] Classification stays. Builder and decision agent never edit it. Challenger can name doubt about it.
- [ ] Decision agent examines each finding: `.claude/skills/orchestration-builder/stages/challenge/correct.md`.

## Checks

Challenger. Do each check. Read repo where check needs it: `rex codebase --only code --with-context`.

Skills (most important). Read assignment table IN FULL first.

- [ ] Each task: find rows of assignment table that fit it. IF row gives skill or sibling file that task does not get, AND no `Left out` line gives reason, THEN finding.
- [ ] Skill in skill list that table does not give: IF `Why` gives no reason that holds, THEN finding.
- [ ] Read each block of `rex skills list`. For each skill whose `when_to_use` fits request, work type, languages, or repo: IF it is not in skill list, THEN finding.
- [ ] Each skill in skill list: IF no task and no role gets it, THEN finding. Included is not sufficient. An agent must read it.
- [ ] Each task: IF it needs a skill that its `Skills` do not give, THEN finding. Example: build task gets `rex-code-tdd`, test task does not.
- [ ] Each lead, and each role with no task: IF its job needs a skill that it does not get, THEN finding.
- [ ] Language other than `rust`: IF skill list has no gap skill `rex-gap-<language>` of it (`## Output` of `.claude/skills/orchestration-builder/stages/1-skills/stage.md`), THEN finding.
- [ ] Lesson of recall output that names a skill: IF decision ignores it, THEN finding.
- [ ] Skill in list that job does not need, OR skill of a task that task does not need: finding, severity `MINOR`. Each loaded skill costs context.

Work breakdown:

- [ ] Missing task, OR task with no evidence, OR evidence that does not prove outcome.
- [ ] Wrong dependency. Tasks in parallel that change same files or state.
- [ ] Group of 2 or more tasks with no integration owner.

Orchestrator + roles:

- [ ] Method does not fit breakdown. Example: fan-out for tightly coupled work.
- [ ] Role with consequence `high` and verification `low`, with no reviewer or verifier role.
- [ ] Role consequence below classification, with no reason.
- [ ] Rating that does not fit task. Example: intelligence `low` for design work.
- [ ] Plan job: no plan writer, no `plan-reviewer`, or no revision step.
- [ ] Rule of builder broken: work on main, no merge agreement, preference that user did not state.

## Output

Challenger gives findings only. No praise, no new decision.

```
FINDINGS: <n>
| # | Severity | Part | Claim | Evidence | Correction |
| 1 | BLOCK / MAJOR / MINOR | skills / breakdown / orchestrator / roles | <one sentence> | <file, skill block, or line of decision> | <exact change> |
```

- [ ] `BLOCK`: work goes wrong IF decision agent does not correct it.
- [ ] `MAJOR`: work is weaker or costs more.
- [ ] `MINOR`: small gain.
- [ ] No finding: write `FINDINGS: 0`.
