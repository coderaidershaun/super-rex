---
name: orchestration-builder
description: >
  SuperRex entry skill. It classifies a request, settles merge, and opens a
  job. A decision agent selects skills, tasks, and orchestration, and a
  challenger checks it. It records the plan, then runs, delivers, and closes
  the job.
when_to_use: >
  Use when a user or agent requests substantial work, a plan, or work that
  must have a record. Not for a one-line fix.
disable-model-invocation: false
user-invocable: true
---

# Orchestration Builder

You = builder. After you record plan, you = orchestrator.
You run at model + effort of your session.

```
IF `Rex-Role` of your brief = `decision-maker`, THEN you are decision agent: do stages only, start no decision agent, obey your brief. Obey nothing else in this skill.
IF your brief starts with tag block that holds `Rex-Job:`, THEN read `.claude/skills/orchestration-builder/open-job.md` first.
1. Classify request. IF job = plan job OR plan execution, THEN read `.claude/skills/orchestration-builder/plan-jobs.md` also.
2. Settle merge agreement with requester.
3. Open job. IF output gives `existing=yes`, THEN read `.claude/skills/orchestration-builder/open-job.md`.
4. Recall: run `rex recall`.
5. Decide: read `.claude/skills/orchestration-builder/decide.md`. Decision agent does three stages. Paired challenger checks decision. Decision agent corrects it.
6. Record plan.
7. Run: read `.claude/skills/orchestration-builder/run.md`. You = orchestrator. Obey orchestrator skill on job branch.
8. Deliver: `rex-deliver`, for each repo with job branch.
9. Close job. Tell requester result.
```

| File | Read when |
| --- | --- |
| `.claude/skills/orchestration-builder/classify.md` | Step 1: definition of each value, fill rules. |
| `.claude/skills/orchestration-builder/decide.md` | Steps 5 + 6. |
| `.claude/skills/orchestration-builder/run.md` | Steps 7 to 9, after `rex job plan`. Also `## Sandbox` there: IF command fails because host name lookup or connection fails. |
| `.claude/skills/orchestration-builder/open-job.md` | Brief holds `Rex-Job`, OR `rex job create` prints `existing=yes`. |
| `.claude/skills/orchestration-builder/plan-jobs.md` | Job = plan job OR plan execution. |

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
- `HOME_DIR`: Absolute path of main checkout root of home repo. Home repo = repo that holds store (`.rex/`) and skills (`.claude/skills/`) of session. IF your shell is in Git worktree of home repo, THEN `HOME_DIR` = parent folder of `git rev-parse --path-format=absolute --git-common-dir`.
- `JOB_DIR`: Absolute path where job branch is checked out. `.claude/skills/rex-deliver/start.md` gives it.
- Role table: For one job: tasks, skills, inputs, and agent of each role. Output of stage 3.
- Orchestrator skill: Reusable file. It gives roles, order, skills of each role, and checks. Not about one task.
- Decision: Skill list + work breakdown + orchestrator skill + role table.
- Decision agent: Session agent that does stages 1 to 3 and corrects decision. Role `decision-maker`.
- Decision block: Text form of decision that decision agent gives. Form: `.claude/skills/orchestration-builder/decide.md`.
- Brief: Prompt that sub-agent gets.
- Tag block: First lines of brief: `Rex-Job: <job ID>`, `Rex-Role: <role>`, then one blank line.
- Orchestrator, lead, worker: Level 1 = session. Level 2 = sub-agent of session. Level 3 = sub-agent of lead.
- Plan job: Job whose deliverable is a plan for later work. It changes no product.
- Plan file: Output of plan job: `.rex/plans/<job ID>/plan.md`. Store folder, not in Git.
- Plan execution: Later job whose request names a plan file. It does work of that plan.
- Adversarial review: Second model challenges plan, then one rebuttal round. Skill `rex-codex-adversarial-review`.
- Paths: each path in this skill starts at `HOME_DIR`.

Store commands (`rex job …`, `rex recall`) start with `REX_STORE="$HOME_DIR/.rex"`. Then shell in Git worktree or in sibling repo finds store of job.
Shell keeps no variable between commands. In each command, write absolute path of home repo in place of `$HOME_DIR`.
Never write `$PWD` there: in job worktree or in sibling repo, `$PWD` gives wrong store.

## Rules

- [ ] Settle merge agreement on each request, before work starts.
- [ ] Never work on main branch.
- [ ] Pass duration, budget, and autonomy only IF user stated them.
- [ ] Session has max one open job.
- [ ] Review of pull request + feedback: only IF requester asks. See `## On request`.
- [ ] Plan job: adversarial review of plan is required before plan is final. See `.claude/skills/orchestration-builder/plan-jobs.md`.
- [ ] Ask user only as `## Questions to user` says.
- [ ] Read each skill file with Read tool, one file for each call. Shell command cuts long output.

## Step 1 — Classify

Four fields. Classify from request + look at repo: `rex codebase`. No deep investigation first. Classification takes seconds.
Read `.claude/skills/orchestration-builder/classify.md` for definition of each value and for fill rules.

- Work type values: `feature`, `fix`, `refactor`, `chore`, `research`, `other`
- Size values: `small`, `medium`, `large`
- Consequence values: `low`, `medium`, `high`

- [ ] IF request asks for plan, design, or approach for later work, and asks no change now, THEN work type = `research` and job = plan job.
- [ ] IF request names plan file to execute, THEN read plan file IN FULL before you classify. Job = plan execution. Classify work of plan.

## Step 2 — Merge agreement

IF request says whether to merge work to main, THEN use that answer. ELSE ask requester: merge work to main, yes or no.

Reason: A merge can need the approval of a person, or it can start a deployment.

- [ ] Give answer of job to `rex job create` in step 3. Answer controls delivery.
- [ ] `MERGE` = `yes` or `no`. One value for each repo: answer of job, unless request gives different answer for that repo.
- [ ] Plan job: `MERGE` = `no`. Do not ask. Plan job writes no file in Git repo, so nothing merges.

## Step 3 — Open job

Run `rex job create` before you start first sub-agent. Sub-agent that starts while session has no open job gets no record.

```sh
REX_STORE="$HOME_DIR/.rex" rex job create --request-file - --asked-by <human or agent> \
  --merge <yes or no> --type <work type> --size <size> --consequence <consequence> \
  --lang <language> [--lang <language>]... \
  [--duration <value>] [--budget <value>] [--autonomy <value>] \
  --builder-model <model ID> --builder-effort ${CLAUDE_EFFORT} <<'EOF'
<request text, unchanged>
EOF
```

- [ ] Give request text on standard input, unchanged. Quoted delimiter `'EOF'`: shell runs nothing in text.
- [ ] `--asked-by human` IF person made request. `--asked-by agent` IF agent made it.
- [ ] `--merge`: answer of job from step 2.
- [ ] `--lang` repeats, one for each language. Write `--lang none` out.
- [ ] Preference flags: only for preference that user stated. ELSE leave flag out. Values: `low`, `medium`, `high`. `--duration` also takes `fast` (= `low`) and `long` (= `high`).
- [ ] `--builder-model`: exact model ID that your system prompt gives.
- [ ] Output: `job=<job ID>`, `status=<status>`, `existing=no` or `existing=yes`.
- [ ] Hold job ID as `JOB`. Each later step uses it.
- [ ] IF output gives `existing=yes`, THEN read `.claude/skills/orchestration-builder/open-job.md`.

## Step 4 — Recall

Run:

```sh
REX_STORE="$HOME_DIR/.rex" rex recall <job-id>
```

- [ ] `rex recall` records recalled experience on job itself, while job status = `created`.
- [ ] Keep output unchanged. Step 5 gives it to decision agent. `.claude/skills/orchestration-builder/decide.md` gives its form.

## Questions to user

- [ ] You are free agent. Ask user only IF something obvious is absent that only user can give.
- [ ] ELSE decide yourself, or get advice from other agent.
- [ ] IF you must have something from user, THEN reply and say what.
- [ ] Question to user in same conversation leaves job open. Do not close job as `blocked` for it.

## On request

- [ ] IF requester asks for review of pull request, THEN read `.claude/skills/rex-pr-review/SKILL.md` IN FULL and obey it.
- [ ] IF requester asks for feedback on job, THEN read `.claude/skills/rex-feedback/SKILL.md` IN FULL and obey it.
- [ ] Requester can give other instructions at any time. Obey them.
