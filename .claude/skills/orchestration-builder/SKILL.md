---
name: orchestration-builder
description: >
  This is the entry skill of SuperRex. It classifies a request, settles with
  the requester whether to merge the work to main, and opens a job. It recalls
  experience, then runs three stages: skill selection, work breakdown, and
  orchestration selection. It records its plan, runs the orchestrator on a new
  branch, delivers the work, and closes the job. Use this skill when a user or
  an agent requests substantial work, or work that must have a record. The work
  can be of any kind: code, research, or other work. Do not use this skill for a
  small change, such as a one-line fix.
disable-model-invocation: false
user-invocable: true
# PENDING(C7): form of the "use when" line and the header keys.
---

# Orchestration Builder

You = builder. After you record plan, you = orchestrator.
You run at model + effort of your session.

```
IF your brief starts with tag lines with job ID, THEN do "Open job exists" first.
1. Classify request.
2. Settle merge agreement with requester.
3. Open job. IF session has open job, THEN do "Open job exists".
4. Recall.
5. Stages, in sequence: nuance-selection, then work-breakdown, then orchestration-selection.
6. Record plan.
7. Run: you = orchestrator. Obey orchestrator skill on new branch.
8. Deliver.
9. Close job. Tell requester result.
```

## Words

- Requester: User or agent that gave request.
- Session: Claude Code conversation that loaded builder. It has one session ID.
- Builder: Agent that obeys `orchestration-builder`.
- Job: One request that SuperRex handles. It has one job ID.
- Open job: Job that `rex job close` did not close. Session has max one.
- Classification: Four fields of job: work type, size, consequence, languages.
- Preference: Duration, budget, or autonomy that user stated in request.
- Merge agreement: Answer of requester: merge work to main, yes or no.
- Skill list: Skills this job uses. Output of stage 1.
- Task: One outcome of job, with evidence that shows it is complete.
- Work breakdown: Tasks of job, their dependencies, and groups that run in parallel. Output of stage 2.
- Role: Named part in orchestrator skill. Example: `engineer`.
- Grid agent: One of 20 agent definitions in `.claude/agents/`. Name: `model-<tier>-effort-<level>`.
- Session agent: `general-purpose` sub-agent that you start with no model. It runs at model + effort of session.
- Main checkout: Repo folder that holds `.claude/skills/` and `.rex/`. IF you work in Git worktree, THEN main checkout = parent folder of `git rev-parse --path-format=absolute --git-common-dir`.
- Role table: For one job: skills, inputs, and agent of each role. Output of stage 3.
- Orchestrator skill: Reusable file. It gives roles, order, skills of each role, and checks. Not about one task.
- Brief: Prompt that sub-agent gets.
- Tag lines: First lines of brief. They give job ID and role.
- Orchestrator, lead, worker: Level 1 = session. Level 2 = sub-agent of session. Level 3 = sub-agent of lead.
- Temp dir: scratchpad dir of session, or system temp dir.
- Paths: each path in this skill starts at main checkout.

Session ID: ${CLAUDE_SESSION_ID}

IF line above shows variable name and no ID, THEN read `$CLAUDE_CODE_SESSION_ID` in shell. Use that ID in each command.

## Rules

- [ ] Settle the merge agreement on each request, before work starts.
- [ ] Never do work on the main branch.
- [ ] Pass duration, budget, and autonomy only IF user stated them.
- [ ] Session has max one open job.
- [ ] Review and feedback: only IF requester asks. See `## On request`.
- [ ] Ask user only as `## Questions to user` says.

## Step 1 — Classify

Four fields. Classify from request + look at repo: `rex codebase`. No deep investigation first. Classification takes seconds.

Work type: what request asks you to deliver. Take first row that applies, from top.

| Value | Definition |
| --- | --- |
| `feature` | Adds behavior that software did not have. |
| `fix` | Corrects behavior that is wrong against its intent. |
| `refactor` | Changes code, not what code does. Includes faster or cleaner code. |
| `chore` | Changes repo, but not behavior and not code structure: documentation, tests, dependencies, build, CI, configuration, tooling. |
| `research` | Deliverable = answer: findings, comparison, recommendation, or plan. Request asks no change to product. |
| `other` | No row above applies. |

Size: how much separate work request holds. Module = one source file, or one folder of files with one purpose.

| Value | Definition |
| --- | --- |
| `small` | One outcome. You expect it to stay inside one module. |
| `medium` | One outcome across several modules, OR 2 or 3 outcomes that depend on each other. |
| `large` | Several independent outcomes, each one you can build and test alone. OR change that reaches most of codebase. |

Consequence: worst credible result of mistake anywhere in job. Test: does revert of commit undo harm?

| Value | Definition |
| --- | --- |
| `low` | Mistake is cheap: normal checks catch it, or revert of commit undoes it. |
| `medium` | Mistake causes meaningful rework or disruption: it can reach main unseen, break other work, or be hard to find. |
| `high` | Mistake can cause damage that revert of commit does not undo: real funds, production data, security or secrets, published release. |

Languages: programming languages of source files that job adds or changes. One or more values. Data, configuration, and markup formats (Markdown, TOML, YAML, JSON) do not count.

| Value | Definition |
| --- | --- |
| `<name>` | Lower-case name, no symbols: `rust`, `go`, `cpp`, `c`, `python`, `typescript`, `javascript`, `java`, `csharp`, `swift`, `kotlin`, `shell`, `sql`. Set is open. Spelling never changes. |
| `none` | Job writes no code. |

Fill rules:

1. Classify whole request as one job, also when it holds two kinds of work.
2. Work type: take first row that applies.
3. IF in doubt between two sizes or two consequence levels, THEN take higher.
4. Write classification one time. Never edit it.

## Step 2 — Merge agreement

IF request says whether to merge work to main, THEN use that answer. ELSE ask requester: merge work to main, yes or no.

Reason: A merge can need the approval of a person, or it can start a deployment.

- [ ] You record the answer on the job in step 3. The answer controls delivery.
- [ ] The answer applies to each repo that the job changes, unless the request gives a different answer for a repo.

## Step 3 — Open job

1. Write request text, unchanged, to file in temp dir.
2. Run:

```
rex job create --session ${CLAUDE_SESSION_ID} --request-file <path> \
  --type <work type> --size <size> --consequence <consequence> \
  --lang <language> [--lang <language>]... \
  [--duration <value>] [--budget <value>] [--autonomy <value>]
```

- [ ] `--lang` repeats, one for each language. Write `--lang none` out.
- [ ] Preference flags: only for preference that user stated. ELSE leave flag out. Values: `low`, `medium`, `high`.
- [ ] Also give merge agreement, who asked (user or agent), and model + effort of builder. `PENDING(C2)`: flags for these three.
- [ ] Output = job ID. Keep it. Each later step uses it.
- [ ] IF command reports that session has open job, THEN do `## Open job exists`.

## Open job exists

Signs: your brief starts with tag lines with job ID, OR `rex job create` reports open job. `PENDING(C2)`: what `rex job create` prints for open job.

Case (a): you are sub-agent inside that job.

1. Open no second job. Use that job ID.
2. Run `rex job show <job-id>`. Take preferences + merge agreement from it. `PENDING(C2)`: fields that it prints.
3. Do step 1 for your part of work. Record nothing.
4. Do step 4 with that job ID.
5. Do step 5, then step 7, for your part. Skip item 2 of step 7. Do the work on the branch that your brief gives. Tag lines of each brief give that job ID. You have fewer sub-agent levels below you: see levels in `orchestration-selection`.
6. Do NOT record plan, deliver, or close job. Owner of job does that. `PENDING(C2)`: whether nested builder records its roles.
7. Report result to requester.

Case (b): you are session, and request continues open job. Run `rex job show <job-id>`. Continue at first step not done.

Case (c): you are session, and request is new work. Close open job first with its true outcome (step 9). Then open new job (step 3). `PENDING(C2)`: whether `blocked` job counts as open job.

## Step 4 — Recall

Run `rex recall <job-id>`. `PENDING(C9)`: output form.

It returns:

- [ ] Lessons from similar jobs.
- [ ] Orchestrators that those jobs used, with result of each job.
- [ ] Orchestrators that never ran.

Use lessons in stages and in briefs where they apply. Give orchestrators to stage 3.

## Step 5 — Stages

Three stages, in sequence. For each stage: read its file IN FULL, do it, keep its output.

| Stage | File | Input | Output |
| --- | --- | --- | --- |
| 1 | `.claude/skills/nuance-selection/SKILL.md` | Request, classification, recall output, job ID | Skill list |
| 2 | `.claude/skills/work-breakdown/SKILL.md` | Request, classification, recall output, skill list | Work breakdown |
| 3 | `.claude/skills/orchestration-selection/SKILL.md` | Work breakdown, skill list, classification, preferences, recall output | Orchestrator skill (name, path, `new` or `reused`) + role table |

- [ ] Small job: stages are short. Output can be one task + one worker.
- [ ] Stage sub-agents that start before you record plan do not change job status.

## Step 6 — Record plan

Run `rex job plan <job-id>`. Give work breakdown, orchestrator skill, skill list, and role table. Job status becomes `planned`. `PENDING(C2)`: flags and input file form.

## Step 7 — Run

1. Now you = orchestrator. Read orchestrator skill IN FULL, at path from stage 3. Obey it. This is plain file read.
2. IF the job changes no file in a Git repo, THEN the job has no branch. ELSE read `.claude/skills/rex-deliver/SKILL.md` IN FULL. Make the job branch as that skill says, before the first change. Give it the job ID, the work type, a slug, and the merge agreement. `PENDING(C12)`: when to read it and what to pass.
3. Start each agent with `subagent_type` = grid agent of its role in role table.
4. IF role table says `session agent` for role, THEN start session agent.
5. Each brief starts with tag lines. Form: `orchestration-selection`.

## Step 8 — Deliver

IF the job changed files in a Git repo, THEN obey `rex-deliver`. Do one delivery for each repo that the job changed. ELSE skip this step. `PENDING(C12)`.
`rex-deliver` gives result of each delivery. Keep each result for step 9.

## Step 9 — Close job

Run `rex job close <job-id>`. Give outcome + one delivery for each repo. `PENDING(C2)`: flags. `PENDING(C12)`: form of delivery.

| Outcome | When |
| --- | --- |
| `delivered` | Work complete. Delivery says how: merged, pull request open, kept locally, or no changes to a repo. |
| `blocked` | Job waits for decision, or for something only user can give. Includes protected main. |
| `failed` | Work stopped. It cannot continue without redo. |
| `cancelled` | User stopped job. |

- [ ] Give true outcome.
- [ ] Deliveries can differ: one repo merged, other repo open or blocked. Record each delivery with what remains to do.
- [ ] Then tell requester result in few lines.

## Questions to user

- [ ] You are free agent. Ask user only IF something obvious is absent that only user can give.
- [ ] ELSE decide yourself, or get advice from other agent.
- [ ] IF you must have something from user, THEN reply and say what.
- [ ] IF work cannot continue without it, THEN close job as `blocked` (step 9).

## On request

- [ ] IF requester asks for review of pull request, THEN read `.claude/skills/rex-pr-review/SKILL.md` IN FULL and obey it.
- [ ] IF requester asks for feedback on job, THEN read `.claude/skills/rex-feedback/SKILL.md` IN FULL and obey it.
- [ ] Requester can give other instructions at any time. Obey them.
