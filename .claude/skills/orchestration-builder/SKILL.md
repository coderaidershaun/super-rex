---
name: orchestration-builder
description: >
  This is the entry skill of SuperRex. It classifies a request, settles with
  the requester whether to merge the work to main, and opens a job. It recalls
  experience, then runs three stages: skill selection, work breakdown, and
  orchestration selection. It records its plan, runs the orchestrator on a new
  branch, delivers the work, and closes the job.
when_to_use: >
  Use when a user or an agent requests substantial work, or work that must
  have a record. The work can be of any kind: code, research, or other work.
  Do not use this skill for a small change, such as a one-line fix.
disable-model-invocation: false
user-invocable: true
---

# Orchestration Builder

You = builder. After you record plan, you = orchestrator.
You run at model + effort of your session.

```
IF your brief starts with tag block that holds `Rex-Job:`, THEN do "Open job exists" first.
1. Classify request.
2. Settle merge agreement with requester.
3. Open job. IF output gives `existing=yes`, THEN do "Open job exists".
4. Recall + list skills: run `rex recall` and `rex skills list`.
5. Stages, in sequence: nuance-selection, then work-breakdown, then orchestration-selection.
6. Record plan.
7. Run: you = orchestrator. Obey orchestrator skill on job branch.
8. Deliver: END of `rex-deliver`, for each repo with job branch.
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
- `HOME_DIR`: Absolute path of main checkout root of home repo. Home repo = repo that holds store (`.rex/`) and skills (`.claude/skills/`) of session. IF your shell is in Git worktree of home repo, THEN `HOME_DIR` = parent folder of `git rev-parse --path-format=absolute --git-common-dir`.
- `JOB_DIR`: Absolute path where job branch is checked out. START of `rex-deliver` gives it.
- Role table: For one job: skills, inputs, and agent of each role. Output of stage 3.
- Orchestrator skill: Reusable file. It gives roles, order, skills of each role, and checks. Not about one task.
- Brief: Prompt that sub-agent gets.
- Tag block: First lines of brief: `Rex-Job: <job ID>`, `Rex-Role: <role>`, then one blank line.
- Orchestrator, lead, worker: Level 1 = session. Level 2 = sub-agent of session. Level 3 = sub-agent of lead.
- Paths: each path in this skill starts at `HOME_DIR`.

Store commands (`rex job …`, `rex recall`) start with `REX_STORE="$HOME_DIR/.rex"`. Then shell in Git worktree or in sibling repo finds store of job.
Shell keeps no variable between commands. In each command, write absolute path of home repo in place of `$HOME_DIR`.
Never write `$PWD` there: in job worktree or in sibling repo, `$PWD` gives wrong store.

## Rules

- [ ] Settle merge agreement on each request, before work starts.
- [ ] Never work on main branch.
- [ ] Pass duration, budget, and autonomy only IF user stated them.
- [ ] Session has max one open job.
- [ ] Review and feedback: only IF requester asks. See `## On request`.
- [ ] Ask user only as `## Questions to user` says.
- [ ] Read each skill file with Read tool, one file for each call. Shell command cuts long output.

## Step 1 — Classify

Four fields. Classify from request + look at repo: `rex codebase`. No deep investigation first. Classification takes seconds.

- Work type values: `feature`, `fix`, `refactor`, `chore`, `research`, `other`
- Size values: `small`, `medium`, `large`
- Consequence values: `low`, `medium`, `high`

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

- [ ] Give answer of job to `rex job create` in step 3. Answer controls delivery.
- [ ] `MERGE` = `yes` or `no`. One value for each repo: answer of job, unless request gives different answer for that repo.

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
- [ ] IF output gives `existing=yes`, THEN do `## Open job exists`.

## Open job exists

Signs:

1. Your brief starts with tag block that holds `Rex-Job:`. You are inside that job. Do not run `rex job create`.
2. `rex job create` prints `existing=yes`.

Case "inside open job": sign 1, OR sign 2 when you are sub-agent.

1. Record nothing: no plan, no START, no END, no close.
2. Run `REX_STORE="$HOME_DIR/.rex" rex job show <job-id>`. Job ID = `Rex-Job` of your brief, or `job=` of `rex job create`.
3. Take from output: `duration`, `budget`, `autonomy` (`none` = not stated), `merge_agreed`, `experience_recalled` (= recall output for stages).
4. Do step 1 for your part of work. Record nothing.
5. Do step 5 for your part. Then run your part with roles of stage 3.
6. Start each role with agent of `agent` line of `rex oms`: there is no plan.
7. Work on branch that your brief gives. Copy `Rex-Job` of your brief into tag block of each brief that you write.
8. IF you start parallel writers, THEN read only section "Parallel writers" of `.claude/skills/rex-deliver/SKILL.md`.
9. IF you must change repo that has no job branch, THEN write nothing there. Report that repo to agent that started you.
10. You have fewer sub-agent levels below you: see levels in `orchestration-selection`.
11. Report result to requester.

Case "session": no `Rex-Job` in your brief, and `rex job create` prints `existing=yes`.

1. Run `REX_STORE="$HOME_DIR/.rex" rex job show <job-id>`.
2. IF request continues that job, THEN continue at first step not done.
3. IF that job is from earlier request that will not continue, THEN close it. Then do step 3 again.

```sh
REX_STORE="$HOME_DIR/.rex" rex job close <job-id> --status cancelled --outcome "<why>"
```

- [ ] IF work of that job failed, THEN use `--status failed` in place of `cancelled`.
- [ ] `--outcome` also names each job branch of that job that stays (step 9).
- [ ] `blocked` job is not open job.

## Step 4 — Recall

Run both commands:

```sh
REX_STORE="$HOME_DIR/.rex" rex recall <job-id>
rex skills list
```

Output of `rex recall`:

```
job=<job ID>
recorded=yes
lessons=<n>
lesson feedback=<id> job=<id> score=<n> status=<status> causes=<causes> text=<lesson>
orchestrators=<n>
orchestrator name=<name> version=<v> best_match=<n> runs=<n> accepted=<n> missed=<n> path=<path>
unused=<n>
unused name=<name> version=<v> path=<path> description=<text>
```

- [ ] `lesson` lines: lessons from similar jobs. Use them in stages and in briefs where they apply.
- [ ] `orchestrator` lines: orchestrator skills that similar jobs used, with results of those jobs.
- [ ] `unused` lines: orchestrator skills whose newest version never ran.
- [ ] Give `orchestrator` lines and `unused` lines to stage 3.
- [ ] `rex recall` records recalled experience on job itself, while job status = `created`.
- [ ] `rex skills list` prints each skill of repo and of user skill folder, with its `when_to_use` line and its `path`.
- [ ] Give its output to stage 1.
- [ ] Never list skills with `ls`: it gives no `when_to_use` line and no skill of user skill folder.

## Step 5 — Stages

Three stages, in sequence. For each stage: read its file IN FULL, do it, keep its output.

| Stage | File | Input | Output |
| --- | --- | --- | --- |
| 1 | `.claude/skills/nuance-selection/SKILL.md` | Request, classification, recall output, output of `rex skills list`, job ID | Skill list |
| 2 | `.claude/skills/work-breakdown/SKILL.md` | Request, classification, recall output, skill list | Work breakdown |
| 3 | `.claude/skills/orchestration-selection/SKILL.md` | Work breakdown, skill list, classification, preferences, recall output | Orchestrator skill (name, path, `new` or `reused`) + role table |

- [ ] Small job: stages are short. Output can be one task + one worker.
- [ ] Stage sub-agents that start before you record plan do not change job status.

## Step 6 — Record plan

Give plan as JSON on standard input:

```sh
REX_STORE="$HOME_DIR/.rex" rex job plan <job-id> --plan-file - <<'EOF'
{"orchestrator": "<name>", "skills": ["<skill name>"], "breakdown": "<work breakdown, Markdown text>",
 "roles": [{"role": "<role>", "intelligence": "<v>", "reliability": "<v>", "consequence": "<v>", "verification": "<v>"},
           {"role": "<role>", "session": true}]}
EOF
```

- [ ] `orchestrator`: name from stage 3. Folder `.rex/orchestrators/<name>/` must exist.
- [ ] `skills`: names from skill list. Can be `[]`.
- [ ] `breakdown`: work breakdown from stage 2, as one JSON string. Write `\n` for each line end.
- [ ] `roles`: role table from stage 3. At least one role. Each `<v>` = `low`, `medium`, `high`, or `none`.
- [ ] Role has all four inputs, OR `"session": true` (session agent). Never both.
- [ ] IF stage 1 started skill writer, THEN add `{"role": "skill-writer", "session": true}`.
- [ ] `rex` selects agent of each role itself. Job gets one plan only.

Output:

```
job=<job ID>
status=planned
orchestrator=<name>
orchestrator_version=<n>
role.<name>=<grid agent>
role.<name>=session
conflict.<name>=<budget, duration, or budget,duration>
```

- [ ] One `role.` line for each role.
- [ ] `conflict.<name>` line comes right after its role line, only IF floor overrode cap (`budget`, `duration`, or both) of that role.

## Step 7 — Run

Now you = orchestrator. Hold these values under these exact names. `rex-deliver` uses same names.

| Name | Value |
| --- | --- |
| `JOB` | Job ID. |
| `WORK_TYPE` | Work type of classification. |
| `SLUG` | 2 to 5 words of `a-z 0-9` from request, joined by single hyphens. Max 40 characters. Pattern `^[a-z0-9]+(-[a-z0-9]+){1,4}$`. Same in every repo. |
| `MERGE` | `yes` or `no`, for each repo (step 2). |
| `HOME_DIR` | See `## Words`. |
| `REPO_DIR` | Absolute path of main checkout root of each repo that job changes. |
| `CHECKS` | Commands that prove work of job in that repo: build, test. Empty IF repo has no such command. |

You make `SLUG` only. You name no branch and no worktree.

1. Read orchestrator skill IN FULL, at path from stage 3. Obey it. This is plain file read.
2. IF job changes no file in Git repo, THEN do not read `rex-deliver`. Job has no branch.
3. ELSE read START of `.claude/skills/rex-deliver/SKILL.md` after plan, before first agent writes file in Git repo.
4. Do START one time for each repo, as soon as you know that repo changes. Never work on main.
5. Start each role with agent that `rex job plan` printed. `role.<name>=<grid agent>`: `subagent_type` = that grid agent.
6. `role.<name>=session`: start session agent.
7. Each brief starts with tag block. Form: `## Briefs` in `orchestration-selection`.
8. IF job works in job worktree, THEN put `JOB_DIR` in brief of each writer. Writer uses absolute paths below it, in each shell command and each file tool.
9. Use no `isolation: "worktree"` for writers.
10. Lead that starts parallel writers: tell it to read section "Parallel writers" of `rex-deliver`. Give it `JOB`, `REPO_DIR`, its branch, absolute path where that branch is checked out, and `CHECKS`.

## Step 8 — Deliver

IF job has no job branch, THEN skip this step. ELSE do these steps for each repo that has job branch, before `rex job close`:

1. Make `TITLE`: `<type>: <what works now>`.
2. Make `BODY`: request, what changed, commands that prove it + their results, what job left out. Last line: `Rex-Job: <JOB>`.
3. Read END of `.claude/skills/rex-deliver/SKILL.md`. Obey it.
4. Keep delivery JSON object that `rex-deliver` gives for that repo.

## Step 9 — Close job

You run `rex job close`. Put delivery objects of all repos in one JSON array, on standard input:

```sh
REX_STORE="$HOME_DIR/.rex" rex job close <job-id> --status <status> --outcome "<one line>" --deliveries-file - <<'EOF'
[<delivery object>, <delivery object>]
EOF
```

- [ ] No delivery: give no `--deliveries-file` and no standard input.

| Status | When |
| --- | --- |
| `delivered` | Work complete. No delivery is `blocked`: each is `merged`, `pr-open`, or `kept-local`, or there is none. |
| `blocked` | One delivery or more is `blocked`. Job closes and waits for something outside session. |
| `failed` | Work stopped. It cannot continue without redo. |
| `cancelled` | User stopped job. |

- [ ] Give true status.
- [ ] `failed` and `cancelled`: END does not run. Give no deliveries file. `--outcome` names each job branch that stays.
- [ ] Blocked job later: record rest with `rex job close` on that job. It names only repos that changed.
- [ ] New agent work after block = new job.
- [ ] Then tell requester result in few lines.

## Questions to user

- [ ] You are free agent. Ask user only IF something obvious is absent that only user can give.
- [ ] ELSE decide yourself, or get advice from other agent.
- [ ] IF you must have something from user, THEN reply and say what.
- [ ] Question to user in same conversation leaves job open. Do not close job as `blocked` for it.

## On request

- [ ] IF requester asks for review of pull request, THEN read `.claude/skills/rex-pr-review/SKILL.md` IN FULL and obey it.
- [ ] IF requester asks for feedback on job, THEN read `.claude/skills/rex-feedback/SKILL.md` IN FULL and obey it.
- [ ] Requester can give other instructions at any time. Obey them.
