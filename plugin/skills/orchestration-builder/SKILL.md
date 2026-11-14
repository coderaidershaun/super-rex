---
name: orchestration-builder
description: >
  SuperRex entry skill. It classifies a request, opens a job, and settles
  merge. A decision agent selects skills, tasks, and orchestration, and a
  challenger checks it. It records the plan, then runs, delivers, and closes
  the job.
when_to_use: >
  Use when a person runs `/super-rex:orchestration-builder` for substantial
  work, a plan, work that must have a record, or `Execute plan <path>`.
disable-model-invocation: true
user-invocable: true
---

# Orchestration Builder

You = builder. After you record plan, you = orchestrator.
You run at model + effort of your session, or of grid agent that a master started.

Person starts builder: `/super-rex:orchestration-builder <request>`, or `/super-rex:orchestration-builder Execute plan <path>` for saved plan. Master starts builder with `rex_build`. No agent can load it with Skill tool.

```
1. Check mod. IF mod is not loaded, THEN stop.
2. Classify request. IF job = plan job OR plan execution, THEN read `orchestration-builder/plan-jobs.md` also.
3. Open job: `rex_job_open`. Read `orchestration-builder/open.md` first. IF result gives `existing=yes`, THEN read `orchestration-builder/open-job.md`.
4. Settle merge answer: `orchestration-builder/open.md`. Its one question: `Merge this job to main when its checks pass?`, options `Yes, merge` + `No, keep the branch`.
5. Recall: `rex_recall`, as `orchestration-builder/open.md` says.
6. Decide: read `orchestration-builder/decide.md`. Decision agent does three stages. Paired challenger checks decision. Decision agent corrects it.
7. Record plan: `rex_job_plan`.
8. Run: read `orchestration-builder/run.md`. You = orchestrator. Obey orchestrator skill on job branch.
9. Deliver: for each repo with job branch, as `orchestration-builder/run.md` says.
10. Close job: `rex_job_close`. Tell requester result.
```

| File | Read when |
| --- | --- |
| `orchestration-builder/classify.md` | Step 2: definition of each value, fill rules. |
| `orchestration-builder/open.md` | Steps 3 to 5: open job, merge answer, recall. |
| `orchestration-builder/decide.md` | Steps 6 + 7. |
| `orchestration-builder/run.md` | Steps 8 to 10, after `rex_job_plan`. Also `## Sandbox` there: IF command fails because host name lookup or connection fails. |
| `orchestration-builder/open-job.md` | `rex_job_open` gives `existing=yes`. |
| `orchestration-builder/plan-jobs.md` | Job = plan job OR plan execution. |
| `orchestration-builder/on-request.md` | Requester asks for review of pull request, or for feedback. |

## Words

- Requester: User or agent that gave request.
- Session: Claude Code conversation that loaded builder. It has one session ID.
- Builder: Agent that obeys `orchestration-builder`. Main session, or grid agent that master started with `rex_build`.
- Master: Agent `super-rex:master`. It starts builders with `rex_build`. A person starts it with `/rex master`.
- Mod: Code of plugin `super-rex` in session. It records each agent, puts skills + tier text in each brief, and refuses spawns and merges that break its rules.
- Mod tool: Tool of mod. Full name `mcp__super-rex__rex_<name>`. This skill writes short name, for example `rex_job_open`.
- Job: One request that SuperRex handles. It has one job ID.
- Open job: Job that `rex_job_close` did not close. Builder has max one.
- Job places: `repo`, `branch`, `base`, `worktree` that `rex_job_open` gives. Job branch = `rex/<job ID>`.
- Classification: Four fields of job: work type, size, consequence, languages.
- Preference: Duration, budget, or autonomy that user stated in request.
- Merge answer: Answer of requester: merge work to main, yes or no. Mod records it from person or from policy of master.
- Skill list: Skills this job uses. Output of stage 1.
- Task: One outcome of job, with evidence that shows it is complete.
- Work breakdown: Tasks of job, their dependencies, and groups that run in parallel. Output of stage 2.
- Role: Named part in orchestrator skill. Example: `engineer`.
- Grid agent: One of 20 agent types of plugin `super-rex`. Name: `super-rex:model-<tier>-effort-<level>`. It has no Skill tool. `rex oms` prints it with no `super-rex:` prefix.
- Skill library: 24 skills of plugin `super-rex` that Claude Code does not list.
- Repo skill: skill in `.rex/skills/<name>/` of repo: job-written, gap skill, or fork that feedback made.
- Skill file: file of a skill. Text names it `<skill>/<file>.md`, or `<skill>` for its `SKILL.md`. Read it with `rex_skill`: `name` = `<skill>`, `file` = `<file>.md`.
- `HOME_DIR`: Absolute path of main checkout root of home repo = `repo` of `rex_job_open`. Home repo holds store (`.rex/`) of job. IF your shell is in Git worktree of home repo, THEN `HOME_DIR` = parent folder of `git rev-parse --path-format=absolute --git-common-dir`.
- `JOB_DIR`: Absolute path where job branch is checked out = `worktree` of `rex_job_open`. `rex-deliver/start.md` makes it.
- Role table: For one job: tasks, skills, inputs, and agent of each role. Output of stage 3.
- Orchestrator skill: Reusable file. It gives roles, order, skills of each role, and checks. Not about one task.
- Decision: Skill list + work breakdown + orchestrator skill + role table.
- Decision agent: Grid agent that does stages 1 to 3 and corrects decision. Role `decision-maker`.
- Decision block: Text form of decision that decision agent gives. Form: `orchestration-builder/decide.md`.
- Brief: Prompt that sub-agent gets. Author writes its first line `Task: <task ID>` + five sections. Mod adds the rest at spawn.
- Orchestrator, lead, worker: Level 1 = builder. Level 2 = sub-agent of builder. Level 3 = sub-agent of lead.
- Plan job: Job whose deliverable is a plan for later work. It changes no product.
- Plan file: Output of plan job: `.rex/plans/<job ID>/plan.md`. Store folder, not in Git.
- Plan execution: Later job whose request names a plan file. It does work of that plan.
- Adversarial review: Second model challenges plan, then one rebuttal round. Skill `rex-codex-adversarial-review`.
- Paths: each path that is not a skill file starts at `HOME_DIR`.

Store commands in shell (`rex job show`, `rex jobs`) start with `REX_STORE="$HOME_DIR/.rex"`. Then shell in Git worktree or in sibling repo finds store of job.
Shell keeps no variable between commands. In each command, write absolute path of home repo in place of `$HOME_DIR`.
Never write `$PWD` there: in job worktree or in sibling repo, `$PWD` gives wrong store.

`rex_skill` + `rex_status` load at start. Each other mod tool waits behind ToolSearch. Before first call of one, load it: ToolSearch `select:mcp__super-rex__<name>`.

## Rules

- [ ] Settle merge answer on each request, before work starts.
- [ ] Never work on main branch.
- [ ] Merge reaches main only through `rex_merge`. Mod refuses `gh pr merge` and each `git push` to main.
- [ ] Pass duration, budget, and autonomy only IF user stated them.
- [ ] Builder has max one open job.
- [ ] Review of pull request + feedback: only IF requester asks. See `## On request`.
- [ ] Plan job: adversarial review of plan is required before plan is final. See `orchestration-builder/plan-jobs.md`.
- [ ] Ask user only as `## Questions to user` says.
- [ ] Read each skill file with `rex_skill`, one file for each call. Read each other file with Read tool.
- [ ] CAUTION: Never run a command outside the sandbox because the sandbox refused a write. A command outside the sandbox can write to every folder of the user.

## Step 1 — Check mod

Call `rex_status` with no argument.

IF your tools hold no `mcp__super-rex__rex_status`, OR call fails with no such tool, THEN mod is not loaded. Do no other step. Tell requester this, and stop:

> The SuperRex mod is not loaded in this session, so no job can open. Run `rex doctor` in a shell to see what is missing. Then start a new Claude Code session in this repo with the plugin `super-rex` enabled, and run `/super-rex:orchestration-builder` again.

## Step 2 — Classify

Four fields. Classify from request + look at repo: `rex codebase`. No deep investigation first. Classification takes seconds.
Read `orchestration-builder/classify.md` for definition of each value and for fill rules.

- Work type values: `feature`, `fix`, `refactor`, `chore`, `research`, `other`
- Size values: `small`, `medium`, `large`
- Consequence values: `low`, `medium`, `high`

- [ ] IF request asks for plan, design, or approach for later work, and asks no change now, THEN work type = `research` and job = plan job.
- [ ] IF request names plan file to execute, THEN read plan file IN FULL before you classify. Job = plan execution. Classify work of plan.

## Questions to user

- [ ] You are free agent. Ask user only IF something obvious is absent that only user can give.
- [ ] ELSE decide yourself, or get advice from other agent.
- [ ] IF you must have something from user, THEN reply and say what.
- [ ] Question to user in same conversation leaves job open. Do not close job as `blocked` for it.
- [ ] Master started you: ask no person. Write question with options + recommendation in your final message. End turn with state `awaiting decision`. Master gets answer from owner and sends it to you.

## On request

- [ ] IF requester asks for review of pull request, OR for feedback on job, THEN read `orchestration-builder/on-request.md`.
- [ ] Requester can give other instructions at any time. Obey them.

## Fixed manifests

Before plan, mod reads `Task:` line of each brief. For these roles, it puts these skill files in `<skills>` block of brief. Each other spawn before plan gets no skills block.

| Role | Read-first skills |
| --- | --- |
| `decision-maker` | `orchestration-builder/stages/1-skills/stage.md`, `orchestration-builder/stages/2-work-breakdown/stage.md`, `orchestration-builder/stages/3-orchestration/stage.md` |
| `decision-challenger` | `orchestration-builder/stages/challenge/stage.md`, `orchestration-builder/stages/1-skills/assign.md` |
| `skill-writer` | `rex-skill-language` |
