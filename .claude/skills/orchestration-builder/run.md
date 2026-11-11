# Run, deliver, close

Steps 7 to 9 of `.claude/skills/orchestration-builder/SKILL.md`. Uses its words and rules. Read after `rex job plan`. Exception: builder reads `## Sandbox` before, for brief of decision agent (`.claude/skills/orchestration-builder/decide.md`).

## Step 7 — Run

Now you = orchestrator. Hold these values under these exact names. `rex-deliver` uses same names.

| Name | Value |
| --- | --- |
| `JOB` | Job ID. |
| `WORK_TYPE` | Work type of classification. |
| `SLUG` | Words of request. Form: row `SLUG` of `## Inputs` in `.claude/skills/rex-deliver/SKILL.md`. |
| `MERGE` | `yes` or `no`, for each repo (step 2). |
| `HOME_DIR` | See `## Words`. |
| `REPO_DIR` | Absolute path of main checkout root of each repo that job changes. |
| `CHECKS` | Commands that prove work of job in that repo: build, test. Empty IF repo has no such command. |

You make `SLUG` only. You name no branch and no worktree.

1. Read orchestrator skill IN FULL, at path of `ORCHESTRATOR` in decision block. Obey it. This is plain file read.
2. IF job changes no file in Git repo, THEN do not read `rex-deliver`. Job has no branch.
3. ELSE read `.claude/skills/rex-deliver/SKILL.md`, then `.claude/skills/rex-deliver/start.md`, with Read tool, after plan, before first agent writes file in Git repo. Exception: new skill that repo tracks starts START before plan (`.claude/skills/orchestration-builder/stages/1-skills/write-skill.md`). Skill tool cannot load `rex-deliver`: only a person starts it.
4. Do START one time for each repo, as soon as you know that repo changes. Never work on main.
5. Start each role with agent that `rex job plan` printed. `role.<name>=<grid agent>`: `subagent_type` = that grid agent.
6. Each brief starts with tag block. Form: `.claude/skills/orchestration-builder/stages/3-orchestration/briefs.md`. Read it IN FULL before you write first brief.
7. IF job works in job worktree, THEN put `JOB_DIR` in brief of each writer. Writer writes files only below it, and names it in each shell command. Skill paths start at `HOME_DIR`.
8. Use no `isolation: "worktree"` for writers.
9. Lead that starts parallel writers: tell it to read `<HOME_DIR>/.claude/skills/rex-deliver/SKILL.md`, then `<HOME_DIR>/.claude/skills/rex-deliver/parallel-writers.md`. Give it `JOB`, `HOME_DIR`, `REPO_DIR`, its branch, absolute path where that branch is checked out, and `CHECKS`.
10. Lead that starts workers: give it in its brief:
    - `SKILL LIST` rows (name + `Path`) of skills of its tasks, and `Skills` + `Open when` of each task.
    - Role name + grid agent (`role.<name>=` of `rex job plan`) of each worker role that it starts.
    - Tag block rules, brief template, lead line of `## Briefs`, and `Output file` of `## Words`, all in `briefs.md`, and brief text of `## Sandbox`.
    - This rule: each worker brief names only skills + sibling files of that worker's task, by absolute path. Worker has no Skill tool. It reads `Skills` first with Read tool. It opens an `Open when` item only IF its work reaches that case. It also reads a sibling file when a `SKILL.md` that it read sends it there.
11. Brief of each role that runs commands holds brief text of `## Sandbox`.
12. Each brief names output file of that agent (`## Words` of `.claude/skills/orchestration-builder/stages/3-orchestration/briefs.md`). Agent writes its full report there. Its final message = path + short summary. Give next agent that path, never text of report. Read output file only IF your next step needs its content.
13. Spend limit. One home of this rule. IF spend limit refuses an agent (deny reason starts `SuperRex spend limit reached`), THEN:
    1. Start no agent. Lead that gets refusal reports `blocked` to you.
    2. Do not close job. Gate reads limit of open job only: after close, raise changes nothing.
    3. Ask requester (`## Questions to user` of `.claude/skills/orchestration-builder/SKILL.md`). Give spend, limit, and raise command. `REX_STORE="$HOME_DIR/.rex" rex limit <job-id>` prints `spent_usd=` + `limit_usd=`. Raise command: `rex limit <job-id> --set <dollars>`.
    4. IF requester raises limit, THEN continue same job. Resume each stopped lead with SendMessage.
    5. IF requester declines, THEN close job `blocked`: obey `.claude/skills/rex-deliver/stopped-or-blocked-job.md`. Work merged in job branch stays there.

## Sandbox

Shell command runs in sandbox. Sandbox gives network only through its proxy.

- Program that uses proxy connects: `curl`, `git`, `cargo`, `gh`.
- Program that opens its own connection does not connect: WebSocket client, database driver, `ssh`.
- Sign: host name lookup fails or connection fails, and `curl` gets same host.

IF command fails for this cause, THEN run that one command again with `dangerouslyDisableSandbox: true` in Bash tool.

CAUTION: Run outside the sandbox only the one command that cannot reach the network. The CAUTION in `## Rules` of `.claude/skills/orchestration-builder/SKILL.md` applies also.

- [ ] Next command runs in sandbox again.
- [ ] Report names each command that ran outside sandbox.
- [ ] Never give user command to run IF you or sub-agent can run it.
- [ ] IF command outside sandbox fails same way, THEN repo is strict (`rex init --strict`). Only then give user command, with `! ` before it, and say which result proves work.

Brief text, for each role that runs commands:

```
Shell commands run in a sandbox that reaches the network only through its proxy. If a command fails because its program opens its own network connection (a host name lookup or a connection fails, but `curl` reaches the same host), run that one command again with `dangerouslyDisableSandbox: true`. Do this for no other cause, and never because the sandbox refused a write. In your report, name each command that ran outside the sandbox.
```

## Step 8 — Deliver

IF job has no job branch, THEN skip this step. ELSE do these steps for each repo that has job branch, before `rex job close`:

1. Make `TITLE`: `<type>: <what works now>`.
2. Make `BODY`: request, what changed, commands that prove it + their results, what job left out. Last line: `Rex-Job: <JOB>`.
3. Read `.claude/skills/rex-deliver/SKILL.md` IF it is not in your context. Then read `.claude/skills/rex-deliver/end.md`. Obey them.
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
| `blocked` | One delivery or more is `blocked`, OR requester declined to raise spend limit (step 7, item 13). Job closes and waits for something outside session. |
| `failed` | Work stopped. It cannot continue without redo. |
| `cancelled` | User stopped job. |

- [ ] Give true status.
- [ ] `failed`, `cancelled`, and declined spend limit: END does not run. Obey `.claude/skills/rex-deliver/stopped-or-blocked-job.md`.
- [ ] Blocked job later: record rest with `rex job close` on that job (`.claude/skills/rex-deliver/stopped-or-blocked-job.md`). It names only repos that changed.
- [ ] New agent work after block = new job.

Output, after delivery lines: skills that plan gave a role and no run of that role opened. Then skills that a run opened and that its role did not get, with words of each `SKILL.md`. Then runs whose final message has more than 2,000 characters.

```
missed_skills=<n>
missed_skill.<i>=<role> <skill>
role_not_run.<i>=<role>
extra_skills=<n>
extra_skill.<i>=<role> <skill> skill_md_words=<words>
skills_incomplete=yes
long_messages=<n>
long_message.<i>=<role> <agent ID> chars=<characters>
```

- [ ] `missed_skill.` line: run of that role did not read that skill. `role_not_run.` line: no run of that role.
- [ ] `extra_skill.` line: run of that role read that skill, and `skills` of that role in plan file do not name it. Role `-` = run with no `Rex-Role` tag. `skill_md_words=<words>`: words of that `SKILL.md` now, not words of files that run read. `skill_md_words=none`: no skill folder has that `SKILL.md`.
- [ ] Extra skill is no error. It is open-when skill that work reached, OR skill that agent found itself. Words = cost: run reads skill one time, then cache gives it again on each later call.
- [ ] `skills_incomplete=yes`: run record is incomplete. Lists can be wrong. Line is absent when record is complete.
- [ ] `long_messages=<n>`: count of runs with final message of more than 2,000 characters. Line is absent IF no run has final message. `long_message.` line: that run gave back its report as text, not path + summary. Each character stays in session context, and each later call pays for it again.
- [ ] Missed skills, extra skills, and long messages do not change status.
- [ ] Then tell requester result in few lines. Give also each line of `REPORT` of decision block, each missed skill with its role, each role that did not run, and each extra skill with its role + words, each long message with its role + characters. IF `skills_incomplete=yes`, THEN say that lists can be wrong. Same for plan job and plan execution.
