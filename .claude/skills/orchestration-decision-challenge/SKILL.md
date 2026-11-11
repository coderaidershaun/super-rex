---
name: orchestration-decision-challenge
description: >
  A paired challenger checks the decision of the orchestration builder before
  the plan is recorded: skills, tasks, roles, orchestrator skill, ratings. It
  looks most for important skills that a task or role does not get. It
  changes nothing.
when_to_use: >
  Use when the challenger of the `orchestration-builder` decision starts its
  check, or when the decision agent gets the findings.
disable-model-invocation: true
user-invocable: false
---

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
| `.claude/skills/orchestration-decision-challenge/start.md` | Builder: before it starts challenger. Steps 1 + 2 of builder. |
| `.claude/skills/orchestration-decision-challenge/correct.md` | Decision agent: when builder sends findings. Steps of decision agent. |

## Words

- Builder: Agent that obeys `orchestration-builder`.
- Decision agent: Sub-agent of builder that does stages 1 to 3. It made decision. It examines findings + corrects decision.
- Decision: Output of three stages: skill list, work breakdown, orchestrator skill, role table.
- Core rules: Skills that each task that writes code reads. Each language: `rex-code-philosophy`, `rex-code-commenting`. Rust adds `rex-code-ergonomics`, `rex-code-error-writing`. Kind of work adds its skill: tests → `rex-code-tdd` OR skill of that test kind (`rex-code-tests-*`). New, split, or moved file → `rex-code-folder-management`. Crate added → `rex-code-external-libraries`. Outside input, retry, or main loop → `rex-code-failure-policy`.
- Decision block: Text form of decision. Decision agent gives it. Builder copies it into brief of challenger.
- Challenger: Fresh sub-agent that checks decision. It made no part of decision. Role `decision-challenger`.
- Skill list: Skills this job uses. Output of stage 1.
- Task: One outcome of job. Its row in work breakdown gives its `Skills`.
- Role table: For one job: tasks, skills, inputs, and agent of each role. Output of stage 3.
- Grid agent: One of 20 agent definitions in `.claude/agents/`. Name: `model-<tier>-effort-<level>`.
- Finding: One claim of challenger: severity, part of decision, claim, evidence, correction.
- Tag block: First lines of brief: `Rex-Job: <job ID>`, `Rex-Role: <role>`, then one blank line.
- Paths: each path in this skill starts at `HOME_DIR`, main checkout root of home repo.

## Rules

- [ ] Challenge runs for each job, before `rex job plan`. Job gets one plan only, so correct decision first.
- [ ] Challenger made no part of decision. Never builder, never decision agent, never stage sub-agent.
- [ ] Challenger only reads. It changes no file and runs no command that writes.
- [ ] One round. Builder does not send decision back for second check.
- [ ] Classification stays. Builder and decision agent never edit it. Challenger can name doubt about it.
- [ ] Decision agent examines each finding: `.claude/skills/orchestration-decision-challenge/correct.md`.

## Checks

Challenger. Do each check. Read repo where check needs it: `rex codebase --only code --with-context`.

Skills (most important):

- [ ] Read each block of `rex skills list`. For each skill whose `when_to_use` fits request, work type, languages, or repo: IF it is not in skill list, THEN finding.
- [ ] Each skill in skill list: IF no task and no role gets it, THEN finding. Included is not sufficient. An agent must read it.
- [ ] Each task: IF it needs a skill that its `Skills` do not give, THEN finding. Example: build task gets `rex-code-tdd`, test task does not.
- [ ] Each lead, and each role with no task: IF its job needs a skill that it does not get, THEN finding.
- [ ] Task that writes code: IF its `Skills` do not give core rules for its language and its kind of work, THEN finding.
- [ ] Language other than `rust`: IF no gap skill covers it, THEN finding.
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
