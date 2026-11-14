# Run, deliver, close

Steps 8 to 10 of `orchestration-builder/SKILL.md`. Uses its words and rules. Read after `rex_job_plan`.

## Step 8 — Run

Now you = orchestrator. Hold these values under these exact names. `rex-deliver` uses same names.

| Name | Value |
| --- | --- |
| `JOB` | Job ID. |
| `WORK_TYPE` | Work type of classification. |
| `MERGE` | `yes` or `no`, for job repo (step 4). |
| `HOME_DIR` | See `## Words`. |
| `REPO_DIR` | Absolute path of main checkout root of each repo that job changes. Job repo: `repo` of `rex_job_open`. |
| `BRANCH`, `MAIN`, `JOB_DIR` | Job repo: `branch`, `base`, `worktree` of `rex_job_open`. |
| `CHECKS` | Commands that prove work of job in that repo: build, test. Empty IF repo has no such command. |

1. Read orchestrator skill IN FULL with Read tool, at path of `ORCHESTRATOR` in decision block. Obey it.
2. IF job changes no file in Git repo, THEN do not read `rex-deliver`. Job has no branch.
3. ELSE read `rex-deliver/SKILL.md`, then `rex-deliver/start.md`, with `rex_skill`, after plan, before first agent writes file in Git repo. Exception: new skill that repo tracks starts START before plan (`orchestration-builder/stages/1-skills/write-skill.md`). Skill tool cannot load `rex-deliver`: only a person starts it.
4. Do START one time for each repo, as soon as you know that repo changes. Never work on main.
5. Start each role with agent that `rex_job_plan` gave: line `role.<name>=super-rex:<grid agent>`. `subagent_type` = whole value after `=`.
6. Start no role as fork. Mod refuses fork inside job: a fork cannot change model.
7. Write each brief by template of `orchestration-builder/stages/3-orchestration/briefs.md`. Read it IN FULL before you write first brief. First line of brief: `Task: <task ID>`.
8. Mod adds rest of brief at spawn: tag block, `<skills>` block of that task, tier text, sandbox text, and Return part with output file. Do not write them.
9. IF mod refuses spawn with `rex brief check (1 of 1)`, THEN correct brief by each finding. Start agent again one time. Mod lets second start pass and marks it.
10. IF job works in job worktree, THEN put `JOB_DIR` in brief of each writer. Writer writes files only below it, and names it in each shell command.
11. Use no `isolation: "worktree"` for writers.
12. Lead that starts parallel writers: tell it to read `rex-deliver/SKILL.md`, then `rex-deliver/parallel-writers.md`, with `rex_skill`. Give it `JOB`, `HOME_DIR`, `REPO_DIR`, its branch, absolute path where that branch is checked out, and `CHECKS`.
13. Lead that starts workers: give it in its brief:
    - Rows of work breakdown of tasks that it leads: task ID, outcome, evidence, owns.
    - Role name + `role.<name>=` value of each worker role that it starts.
    - This rule: each worker brief obeys template of `orchestration-builder/stages/3-orchestration/briefs.md`, with first line `Task: <task ID of that worker>`. Lead reads that file with `rex_skill` before it writes first worker brief. Mod puts skills of that task in worker brief.
14. Mod names output file of each agent in its Return part. Agent writes its full report there. Its final message = path + short summary. Give next agent that path, never text of report. Read output file only IF your next step needs its content.
15. Depth. IF mod refuses spawn with `depth <n> is for master trees only`, THEN start that work one layer up. Facts: `orchestration-builder/stages/3-orchestration/limits.md`.
16. Spend limit. One home of this rule. IF mod refuses an agent with reason `job <id> has spent $<spent> of its $<limit> limit`, THEN:
    1. Start no agent. Lead that gets refusal reports `blocked` to you.
    2. Do not close job. Mod reads limit of open job only: after close, raise changes nothing.
    3. Master started you: end turn with state `blocked`, with spend + limit. Master or its policy decides. Go to item 5 only IF master tells you.
    4. ELSE ask requester (`## Questions to user` of `orchestration-builder/SKILL.md`). Give spend + limit: `spent_usd` + `limit_usd` of `rex_status`. Person raises limit with `/rex limit <job-id> <usd>`.
    5. IF limit goes up, THEN continue same job. Resume each stopped lead with SendMessage.
    6. IF requester declines, THEN close job `blocked`: obey `rex-deliver/stopped-or-blocked-job.md`. Work merged in job branch stays there.

## Sandbox

Shell command runs in sandbox. Sandbox gives network only through its proxy.

- Program that uses proxy connects: `curl`, `git`, `cargo`, `gh`.
- Program that opens its own connection does not connect: WebSocket client, database driver, `ssh`.
- Sign: host name lookup fails or connection fails, and `curl` gets same host.

IF command fails for this cause, THEN run that one command again with `dangerouslyDisableSandbox: true` in Bash tool.

CAUTION: Run outside the sandbox only the one command that cannot reach the network. The CAUTION in `## Rules` of `orchestration-builder/SKILL.md` applies also.

- [ ] Next command runs in sandbox again.
- [ ] Report names each command that ran outside sandbox.
- [ ] Never give user command to run IF you or sub-agent can run it.
- [ ] IF command outside sandbox fails same way, THEN repo is strict (`rex init --strict`). Only then give user command, with `! ` before it, and say which result proves work.

Brief text. Mod adds it to each brief at spawn. Write it yourself only in a brief to an agent outside a job:

```
Shell commands run in a sandbox that reaches the network only through its proxy. If a command fails because its program opens its own network connection (a host name lookup or a connection fails, but `curl` reaches the same host), run that one command again with `dangerouslyDisableSandbox: true`. Do this for no other cause, and never because the sandbox refused a write. In your report, name each command that ran outside the sandbox.
```

## Step 9 — Deliver

IF job has no job branch, THEN skip this step. ELSE do these steps for each repo that has job branch, before `rex_job_close`:

1. Make `TITLE`: `<type>: <what works now>`.
2. Make `BODY`: request, what changed, commands that prove it + their results, what job left out. Last line: `Rex-Job: <JOB>`.
3. Read `rex-deliver/SKILL.md` IF it is not in your context. Then read `rex-deliver/end.md`. Obey them.
4. Keep delivery object that `rex-deliver` gives for that repo.

## Step 10 — Close job

Call `rex_job_close`:

| Argument | Value |
| --- | --- |
| `status` | Status from table below. |
| `outcome` | One line. |
| `deliveries` | Delivery objects of all repos, one list. `[]` IF no delivery. |

| Status | When |
| --- | --- |
| `delivered` | Work complete. No delivery is `blocked`: each is `merged`, `pr-open`, or `kept-local`, or there is none. |
| `blocked` | One delivery or more is `blocked`, OR requester declined to raise spend limit (step 8, item 16). Job closes and waits for something outside session. |
| `failed` | Work stopped. It cannot continue without redo. |
| `cancelled` | User stopped job. |

- [ ] Give true status.
- [ ] Delivery `merged` needs a merge that `rex_merge` did for this job. ELSE mod refuses close.
- [ ] `failed`, `cancelled`, and declined spend limit: END does not run. Obey `rex-deliver/stopped-or-blocked-job.md`.
- [ ] Blocked job later: record rest with `rex_job_close` on that job (`rex-deliver/stopped-or-blocked-job.md`). It names only repos that changed.
- [ ] New agent work after block = new job.
- [ ] `rex_job_close` is safe to repeat with same input.

Result lines:

| Key | Meaning |
| --- | --- |
| `job`, `status` | Job + its status now. |
| `spent_usd` | Spend of job, at list prices. |
| `runs` | Count of sub-agents of job. |
| `missed.<role>=<skill>` | Plan gave role that skill. No run of that role opened it. |
| `extra.<role>=<skill>` | Run of that role opened that skill. Plan did not give it to role. |
| `long.<agent ID>=<characters>` | Run whose final message has more than 2,000 characters. |
| `not_assembled=<n>` | Count of runs whose brief mod did not build at spawn. |
| `degraded` | Parts of mod that failed in job, comma list, or `none`. |
| `cost_of_record_usd` | Storage on only: cost that store records for job. |

- [ ] Extra skill is no error. It is open-when skill that work reached, OR skill that agent found itself.
- [ ] `long.` line: that run gave back its report as text, not path + summary. Each character stays in session context, and each later call pays for it again.
- [ ] `degraded` is not `none`: record can miss facts. Missed + extra lists can be wrong.
- [ ] Missed skills, extra skills, long messages, and `not_assembled` do not change status.
- [ ] Then tell requester result in few lines. Give also each line of `REPORT` of decision block, each missed skill with its role, each extra skill with its role, each long message with its agent + characters. IF `degraded` is not `none`, THEN say that lists can be wrong. Same for plan job and plan execution.
- [ ] Master started you: your final message gives `status`, `outcome`, and path of your report. Master reads it.
