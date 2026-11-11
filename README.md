<p align="center">
  <img src="https://raw.githubusercontent.com/coderaidershaun/rex-skills/main/static/logo.png" alt="SuperRex logo" width="200">
</p>

# SuperRex

This is version 0.4.0 of SuperRex.

SuperRex is two things that work together inside your repo:

- **Skills.** You give Claude Code a request with `/orchestration-builder <request>`. The skill works out how to do the job: which skills it needs, what can run in parallel, how the agents are organised, and which model and effort each one runs at. Then it runs the job with Claude Code's own sub-agents and delivers it.
- **A small tool, `rex`.** It sets the repo up, picks a model for each role with a fixed function, and keeps a local record of what was asked, how it was done, what it cost, and whether it hit the mark. When you later say a job missed, the `rex-feedback` skill reads that record, finds the cause, and stores a lesson that later jobs recall.

SuperRex supersedes rex-skills (Rex 2.0). Rex 2.0 ran one fixed process on every request; SuperRex lets the agent choose the process, the skills and the models for each request, and records the result so that the next choice is better. The old installer `rex skills` is now `rex init`; `rex skills` now lists and checks skills.

## Install

```sh
cargo install super-rex
```

To build from source instead:

```sh
git clone https://github.com/coderaidershaun/super-rex
cd super-rex
cargo install --path .
```

The binary is `rex`. rex-skills installs a binary named `rex` too: if it is installed, run `cargo uninstall rex-skills` first, and make sure that the `rex` first on your `PATH` is this one (`rex --version` prints `rex 0.4.0`).

| Need | Why |
| --- | --- |
| Rust 1.89 or later | The minimum the crate builds with. |
| Claude Code 2.1.322 or later | `rex init` refuses an older version. |
| macOS or Linux | Windows is not supported. |
| On Linux: `bwrap` (package `bubblewrap`) and `socat` | The Claude Code sandbox needs them. Install with `sudo apt-get install bubblewrap socat` or `sudo dnf install bubblewrap socat`. |
| `gh`, signed in | Pull requests. Without it, `rex init` warns and delivery works without pull requests. |
| `pre-commit`, `gitleaks` and `trivy` | Only for the pre-commit config that `rex init` writes. Without `pre-commit` the hooks are not installed; without `gitleaks` every commit fails, without `trivy` every push fails, and `rex init` warns about each. |

## Set a repo up: `rex init`

Run it yourself, in your own shell, in the root of the main checkout (the folder that holds `.git`). Do not ask an agent to run it: inside a session the sandbox refuses writes to `.claude/skills/`, and the pre-commit hook store cannot be filled there.

```console
$ cd ~/code/shop
$ rex init
rex init: .claude/settings.local.json created
rex init: 149 written, 0 skipped
rex init: AGENTS.md created
rex init: .gitignore created
rex init: .rex/ created
rex init: .pre-commit-config.yaml created
rex init: pre-commit hooks installed
rex init: commit AGENTS.md, .gitignore and .pre-commit-config.yaml (git add + git commit); a job works in the main checkout only when it is clean
```

What it writes:

| Path | What | In Git |
| --- | --- | --- |
| `.claude/skills/` | The 6 skills that you start, or that a session finds by itself (section "Skills and the agent grid"). | No |
| `.claude/agents/` | The 20 grid agents. | No |
| `.claude/settings.local.json` | The hooks, the safety settings and one allow rule, merged into your file. | No |
| `.rex/skills/` | The skill library: the 22 skills that agents read by path. | No |
| `.rex/` | The store `rex.db`, the skill library `skills/`, and the empty library folder `orchestrators/`. Jobs add the rest (see Files in "Reference"). | No |
| `.gitignore` | A block that starts with `# rex:ignore superrex` and ignores `.claude/skills/`, `.claude/agents/`, `.claude/settings.local.json`, `/.rex/*` and `/.claude/worktrees/`. | Yes |
| `AGENTS.md` | Two sections at the top, after the marker `<!-- rex:codebase -->`: how to use `rex codebase` and `rex smells`. | Yes |
| `.pre-commit-config.yaml` | Checks for large files, merge markers, TOML, YAML, private keys and white space; gitleaks at commit; trivy at push; `cargo fmt`, `cargo clippy` and (at push) `cargo test` for Rust files. Only if you have no such file. | Yes |

Then commit the three project files. A job works in the main checkout only when it is clean:

```sh
git add AGENTS.md .gitignore .pre-commit-config.yaml && git commit -m "chore: set up SuperRex"
```

**Safe to run twice.** A second run changes no file in the repo. `rex init` never replaces what you have:

- A skill or agent file, in `.claude/` or in `.rex/skills/`, that differs from this version of `rex` is skipped and counted (`(1 differs from this version; use --force to overwrite)`). `--force` overwrites those files, and only those: it never touches the settings file, `AGENTS.md`, `.gitignore` or the pre-commit config.
- Your settings file is merged: missing keys and list entries are added, your keys and order stay. A value of yours that differs is kept, with a warning.
- `AGENTS.md` and `.gitignore` get their block once, found by its marker. An existing pre-commit config is never replaced.
- Nothing is written through a symbolic link. Skills of your own are left alone. `rex init` removes nothing, does not touch your user-level Claude Code settings, and writes no `CLAUDE.md`.
- The ignore block hides whole folders, so a skill of your own in `.claude/skills/` or `.rex/skills/` is hidden from Git too. To keep one in Git, run `git add -f <path>`. To keep the whole library in Git, add the line `!/.rex/skills/` after the block: it works because the block ignores what is in `.rex/` (`/.rex/*`), not the folder itself.

**`--no-hooks`** leaves out the hooks and the allow rule, so no telemetry is recorded, no spend limit is enforced and no test output is cut. Everything else is written. Hook entries from an earlier run are not removed: delete the entries that run `rex hook`, `rex gate` and `rex test-hook` to turn them off.

**The hooks.** `rex init` writes five entries, each with a timeout of 10 seconds: `rex hook` for `SubagentStart`, `SubagentStop` and `PostToolUse` (matcher `Agent|Task`), `rex gate` for `PreToolUse` (matcher `Agent|Task`), and `rex test-hook` for `PreToolUse` (matcher `Bash`). A repo that was set up before gets the two new entries from the next `rex init`. See `rex hook`, `rex gate` and `rex test-hook` under "The commands".

**The preflight** runs before anything is written. These stop the run with exit 1 and write nothing: the folder holds no `.git` folder (or `.git` is a file: a linked worktree or submodule); `claude` is not on `PATH`; its version cannot be read; it is older than 2.1.322. These only warn: no `gh` on `PATH`; no `rex` on `PATH`, or another `rex` comes first (the hooks and skills call `rex` by `PATH`). On Linux, a missing `bwrap` or `socat` is reported at the end with the install command, and the run exits 1; the settings are still written, so Claude Code refuses to start in the repo until the tools are there.

**A weaker safety value is a failure.** If your settings file already holds a different value for a safety key (anything under `sandbox`, or `permissions.blockWritesOutsideWorkingDirectories`), `rex init` keeps your value, finishes the rest, and exits 1:

```console
rex init: failed: kept sandbox.enabled = false in .claude/settings.local.json, so the protection it gives is off; set it to true to turn it on
error: rex init did not finish 1 step(s); each is named above
```

Fix: set the value the message names in `.claude/settings.local.json`, then run `rex init` again. A settings file that is not valid JSON is left as it is, with the same kind of failure.

### The safety settings

The protection is Claude Code's own sandbox, enforced by the operating system. `rex init` writes the settings; it adds no code of its own.

| Setting | What it is for |
| --- | --- |
| `sandbox.enabled: true` | Shell commands run in the sandbox. |
| `sandbox.failIfUnavailable: true` | Claude Code refuses to start when the sandbox cannot, instead of running commands unsandboxed. |
| `sandbox.allowUnsandboxedCommands: true` | An agent can run a command again outside the sandbox. `--strict` writes `false`. See "Commands outside the sandbox". |
| `sandbox.enableWeakerNetworkIsolation: true` | `gh` works inside the sandbox. |
| `sandbox.network.allowLocalBinding: true` | A sandboxed command can listen on localhost, so a test server or a nested `claude -p` starts. |
| `sandbox.excludedCommands` | `git worktree`, `git merge`, `git pull`, `git switch` and `rex init` run outside the sandbox. Claude Code does this for a git command only when its paths are relative with no `..`, run from inside the repo: `git -C /abs/path …` or an absolute target stays in the sandbox. The sandbox never lets a command write `.claude/skills`, `.claude/agents` or the `.claude` settings files, and no setting narrows that, so these commands could not check out a repo that tracks those folders, or install them. They still ask for permission as before. `git commit` and `git push` stay in the sandbox, because their hooks run code that an agent can change. `--strict` writes none. |
| `sandbox.filesystem.allowWrite` | The five Cargo cache folders (not `~/.cargo/bin`), `~/.cache/pre-commit/patch*`, and the trivy cache (`~/Library/Caches/trivy` on macOS, `~/.cache/trivy` on Linux). |
| `sandbox.filesystem.denyRead: ["~/.aws"]` | AWS credentials cannot be read. |
| `permissions.additionalDirectories` | The parent folder of the repo, as an absolute path, so that agents can edit sibling repos. Left out, with a warning, when it holds your home folder. |
| `permissions.blockWritesOutsideWorkingDirectories: true` | The file tools cannot write outside the working folders. |
| `permissions.allow` | `Bash(rex test-filter)` and `Bash(set -o pipefail)`, so that a test command whose output `rex test-hook` cuts needs no extra permission. See `rex test-hook`. |
| `permissions.deny` | `Bash(sudo *)`, `Bash(git push --force*)`, `Bash(cargo publish*)`, `Bash(gh pr merge*--admin*)` (a protected main is never bypassed). |

The network stays open, but only through the sandbox's proxy. In bypass mode a host list restricts nothing, so `rex init` writes none. A program that uses the proxy, such as `curl`, `git`, `cargo` or `gh`, reaches any host. A program that opens its own connection, such as a WebSocket client, a database driver or plain `ssh`, cannot connect inside the sandbox.

Agents cannot edit the Claude Code settings files; `rex init` can, because you run it. A sandboxed command can still delete the repo itself: Git recovers the code, but the store in `.rex/` is not in Git.

### Commands outside the sandbox

Because of the proxy limit, an agent can run a command again outside the sandbox. The builder skill tells agents to do this only for a command that failed because its program opens its own network connection, such as a live test of a WebSocket, and to name each such command in their report.

A command outside the sandbox runs with your full access. In bypass mode Claude Code approves it without a prompt; in the default mode it asks you first. So the sandbox stops mistakes, but it does not stop an agent that chooses to leave it. The file tools still cannot write outside the working folders.

**`--strict`** writes `sandbox.allowUnsandboxedCommands: false` and no `sandbox.excludedCommands`, and then no command leaves the sandbox. A live test that opens its own connection cannot run in a session, so the agent gives you the command to run yourself.

A stricter value of yours is never made weaker. If the settings file already holds `false`, for example from an earlier version of `rex`, a run without `--strict` keeps it, adds no excluded commands, and warns. Set it to `true` by hand to let agents run such commands. The other way round, a kept `true` fails a `--strict` run, like any weaker safety value.

## How a job runs

Start Claude Code in the main checkout and give it a request with `/orchestration-builder <request>`. Only you can start the builder: Claude cannot load it by itself, and neither can any agent. The session then:

1. **Classifies** the request: work type, size, consequence, languages.
2. **Agrees on the merge.** If the request does not say whether the work may be merged to main, it asks you. This is the one rule every job keeps.
3. **Opens the job** with `rex job create`, which prints the job ID.
4. **Recalls** lessons from similar jobs and the orchestrators they used (`rex recall`).
5. **Starts a decision agent.** A sub-agent makes the decision, on the grid agent that `rex oms` picks for it (never on the `xhigh` tier unless the consequence is high), so the session does not carry the rules of the three stages while it runs the job. The decision agent lists the skills in the repo's `.claude/skills/`, in the skill library `.rex/skills/` and in your own skills folder (`rex skills list`), then works through three stages. Each stage is a file inside the builder's folder, `.claude/skills/orchestration-builder/stages/`, not a skill of its own:
   - **Stage 1, `1-skills/stage.md`:** chooses the skills for the job. It starts from one assignment table, `1-skills/assign.md`, which says which skills each kind of work gets: every task that writes code gets `rex-code-philosophy`, `rex-code-commenting`, `rex-code-ergonomics` and `rex-code-error-writing`, whatever its language, and tests, new files, new dependencies, retries, designs, refactors and the rest each add their own. A skill the table does not give, or one it gives that the job leaves out, needs a one-line reason. It asks for a new skill only when a three-part test passes, and the session then has a sub-agent write it (also on a grid agent from `rex oms`).
   - **Stage 2, `2-work-breakdown/stage.md`:** splits the request into tasks, gives each task the skills the table gives its kind of work, and finds what can run in parallel. One task builds one feature whole: code is split into parallel tasks only where the files and outcomes are independent. The skills of a task are two lists: `Skills`, at most 6, which the agent reads first, and `Open when`, each with its case, which the agent opens only when its work reaches that case. The sister skills that a skill names are a map, not a reading order.
   - **Stage 3, `3-orchestration/stage.md`:** reuses an orchestrator from `.rex/orchestrators/` or writes a new one, gives each task a role, defines the roles, and rates the inputs that pick each role's model. It caps the sub-agents of the job by its size, counting the decision agent, the challenger, leads, workers and reviewers, and each run of a role: 5 for a small job, 6 for a medium one, 10 for a large one. The decision shows the count as `AGENTS: <count> of max <max>`, and the challenger reports a plan over the cap as a `MAJOR` finding.

   If only you can answer a question, the decision agent asks it through the session, at most twice.
6. **Challenges the decision.** A paired challenger (also on a grid agent from `rex oms`), a fresh sub-agent that made no part of it, checks the skill list, the work breakdown, the orchestrator and the role table before any work starts (`stages/challenge/stage.md`). It checks the decision against the assignment table, and looks most for important skills that were left out or not given to a task or a role. The session passes the findings to the decision agent, which checks each one against the real files, corrects what holds, and records the check in the work breakdown.
7. **Records the plan** with `rex job plan`, with the skills of each role. `rex` picks the grid agent for each role.
8. **Runs** the orchestrator on a job branch, never on main. Leads and workers are Claude Code sub-agents; the hooks record each one. Each job has a spend limit in dollars (see `rex limit`): once the job has spent it, `rex gate` refuses every new sub-agent; the job stays open, and the session asks you to raise the limit. Each agent's brief names the read-first skills of its own task, and the open-when skills with their cases, by absolute path in the main checkout, since a Git worktree holds no file that Git ignores. A grid agent has no Skill tool, so it reads the files its brief names and no others.
9. **Delivers** each changed repo with the steps of `rex-deliver`, which the session reads by path: pull request, and a squash merge if one was agreed.
10. **Closes** the job with `rex job close`, then tells you the result in a few lines: what was done, and for each repo whether it was merged, left as an open pull request, kept locally, or blocked, with what you must do.

The session is the orchestrator and runs at your model and effort; every sub-agent runs on a grid agent that `rex oms` picks for its role. A small job can be one task and one worker.

### Plan first, then build

A request for a plan, such as "plan how we add CSV export", becomes a **plan job**. It changes no file in the repo, so it needs no merge question and no branch. Its orchestrator is built for planning: optional researchers, one plan writer, a `plan-reviewer`, then a revision. The plan goes to `.rex/plans/<job-id>/plan.md`, with a goal, decisions, scope, a task table, risks, acceptance evidence and open questions.

Before the plan is final, `rex-codex-adversarial-review` challenges it with a second model. The OpenAI Codex CLI (`codex`, read-only) questions the approach and the assumptions first, and the agent running the review checks each finding against the real files. Codex then gets one round to concede or hold its position. If Codex is not installed or not logged in, a grid agent challenges the plan instead. `rex oms` picks it from intelligence high, reliability high, verification medium and the consequence of the job, so the size of the plan does not change it: `model-high-effort-high` for a low or medium consequence, `model-xhigh-effort-high` for a high one. Findings that hold change the plan, and contested ones go into its open questions.

You read the plan, then start the builder again with `/orchestration-builder Execute plan <path of plan.md>`. That second job asks about the merge as usual and works from the plan's task table.

## The commands

`rex commands` lists them all. Paths in the outputs below are shortened to `/home/ana/code/shop`.

**`rex init [--force] [--no-hooks] [--strict]`**: set SuperRex up in a repo. See above.

**`rex job`**: the group of the four job commands below. On its own it prints its help on stderr and exits 2.

**`rex job create`**: open a job and print its ID. Required: `--request-file <path|->`, `--asked-by human|agent`, `--merge yes|no`, `--type feature|fix|refactor|chore|research|other`, `--size small|medium|large`, `--consequence low|medium|high`, and `--lang <name>` once for each language, or `--lang none`. Optional: `--session` (default `$CLAUDE_CODE_SESSION_ID`), `--duration`, `--budget` and `--autonomy` (`low|medium|high`, only when the user stated one; `--duration` also takes `fast` and `long`), `--builder-model`, `--builder-effort` (default `$CLAUDE_EFFORT`), `--json`.

```console
$ echo "Make the order parser reject a quantity of zero." | rex job create --request-file - \
    --asked-by human --merge yes --type fix --size small --consequence medium --lang rust \
    --session demo-session-1 --builder-model claude-opus-5-5
job=0g51g6fykv
status=created
existing=no
```

A session has at most one open job (`created` or `planned`). A second create in the same session opens nothing, prints that job with `existing=yes`, notes it on stderr, and exits 0.

**`rex job plan <job> --plan-file <path|-> [--json]`**: record the skills, the work breakdown, the orchestrator and the roles, and move the job to `planned`. A job is planned once. The orchestrator folder `.rex/orchestrators/<name>/` must exist. Each role gives four inputs, each `low`, `medium`, `high` or `none`; `rex oms` turns them into the role's grid agent, and every role of a builder job goes through it. A role may also give `"skills"`, the skills its tasks are assigned: each is also in the job's `skills`, and is given once. `rex job close` reports each of them that no run of the role opened. Each skill name, of the job or of a role, is a skill folder name in the form of rule H3, so a sibling file such as `rex-code-tdd/mocking.md` is refused: give its skill, `rex-code-tdd`. A `conflict.<role>=budget`, `duration` or `budget,duration` line follows a role's line when the user's preference capped the agent that the role needed.

```console
$ rex job plan 0g51g6fykv --plan-file - <<'EOF'
{"orchestrator":"solo-engineer","skills":["rex-code-tdd","rex-code-philosophy"],"breakdown":"1. Reject zero in the parser; test.",
 "roles":[{"role":"engineer","intelligence":"medium","reliability":"high","consequence":"medium","verification":"high",
           "skills":["rex-code-tdd","rex-code-philosophy"]},
          {"role":"skill-writer","intelligence":"medium","reliability":"medium","consequence":"medium","verification":"medium"}]}
EOF
job=0g51g6fykv
status=planned
orchestrator=solo-engineer
orchestrator_version=1
role.engineer=model-high-effort-high
role.skill-writer=model-medium-effort-medium
```

**`rex job close <job> --status delivered|blocked|failed|cancelled --outcome <one line> [--deliveries-file <path|->] [--json]`**: record the outcome, with one delivery object per changed repository (`repo`, `branch`, `pr`, `merge_agreed`, `merge_commit`, `outcome`, `note`). Leave the file out when no repository changed.

```console
$ rex job close 0g51g6fykv --status delivered --outcome "parser rejects zero; merged" --deliveries-file - <<'EOF'
[{"repo":"/home/ana/code/shop","branch":"fix/reject-zero-quantity-0g51g6fykv","pr":null,"merge_agreed":true,
  "merge_commit":"53622c9d5c38e5adfd47454e2cedcdc3f0a24feb","outcome":"merged","note":null}]
EOF
job=0g51g6fykv
status=delivered
deliveries=1
delivery.1=merged /home/ana/code/shop
missed_skills=1
missed_skill.1=engineer rex-code-philosophy
extra_skills=1
extra_skill.1=engineer rex-code-smells skill_md_words=10
long_messages=1
long_message.1=engineer a0c8cde396de066a3 chars=3120
```

When a role has skills, the close compares them with the skills its runs opened: a run opens a skill when it loads it with the Skill tool, reads its `SKILL.md`, or reads another `.md` file below `.claude/skills/<name>/` or `.rex/skills/<name>/`. `missed_skills=<n>` counts the `missed_skill.<i>=<role> <skill>` lines that follow, one for each skill that no run of the role opened. A role with skills and no run at all gets one `role_not_run.<i>=<role>` line instead. A role run as the session is not checked, because the session's own reads are not recorded. An extra skill is one that a run opened and that is not in its role's `skills`, such as an open-when skill that the work reached. `extra_skills=<n>` counts the `extra_skill.<i>=<role> <skill> skill_md_words=<words>` lines that follow the missed-skill lines, one for each role and skill, so that you see what the work reached and what it cost. The lines show only when the transcript of at least one run was read. A run with no `Rex-Role` tag shows as role `-`. A role that is not in the plan (a plan writer, say) was given no skills, so every skill its runs opened is extra. `skill_md_words` is the number of words in the skill's `SKILL.md` as it is on disk when the job closes. It is the size of that file, not what the run read. It is `none` if no skills folder holds the skill. An extra skill is not an error. `skills_incomplete=yes` means the record of the runs misses facts, so either list may be wrong. `--json` gives the same as `"missed_skills": [{"role": ..., "skill": ...}]`, `"roles_not_run": [...]`, `"extra_skills": [{"role": ..., "skill": ..., "skill_md_words": ...}]` and `"skills_incomplete": true|false`. `long_messages=<n>` counts the `long_message.<i>=<role> <agent ID> chars=<chars>` lines that follow: one for each run whose final message has more than 2,000 characters. They print when a final message was read, and `--json` gives `"long_messages"`. The builder tells you each long message with its role and length. A missed or extra skill, or a long message, never changes the status or the exit code.

Sub-agents of a job write their full report to `.rex/plans/<job>/<role>.md` (`<role>-<n>.md` for a repeat run) and return the path and a short summary, so the session passes paths between agents, not long text. A job that the spend limit stopped closes `blocked`; you get the spend, the limit and `rex limit <job> --set <dollars>` (rule in `rex-deliver/stopped-or-blocked-job.md`).

**`rex job show [<job>] [--json]`**: print one job's record: classification, preferences, merge answer, plan, roles with their model selection and skills (`skills=none`, or `[]` in `--json`, for none), deliveries, sub-agent runs with the skills each opened and their tokens and cost, the main session's tokens and cost (`session` line), request, breakdown and the recalled experience. A run's `skills_opened` in `--json` is `[]` when its transcript was read and it opened no skill, and `null` when no transcript was read; the text form prints `skills=none` for both. With no job, it shows the open job of `$CLAUDE_CODE_SESSION_ID`. `--json` adds the transitions, each run's brief and final message, and the feedback.

```console
$ rex job show 0g51g6fykv
job=0g51g6fykv
status=delivered
...
role.engineer=model-high-effort-high inputs=--M-HMH demand=model-high-effort-high rules=T1,T2,E1,E3,F1 conflict=none oms_version=2 skills=rex-code-tdd,rex-code-philosophy
role.skill-writer=model-medium-effort-medium inputs=--M-MMM demand=model-medium-effort-medium rules=T1,E1 conflict=none oms_version=2 skills=none
delivery.1=merged branch=fix/reject-zero-quantity-0g51g6fykv pr=none merge_agreed=yes merge_commit=53622c9d5c38e5adfd47454e2cedcdc3f0a24feb repo=/home/ana/code/shop
request:
  Make the order parser reject a quantity of zero.
```

**`rex hook`**: called by Claude Code, not by people. It appends the hook event on stdin as one line to `.rex/events.jsonl`, prints nothing, and always exits 0. `rex init` registers it for `SubagentStart`, `SubagentStop`, and `PostToolUse` with matcher `Agent|Task`, each as this command with a timeout of 10 seconds:

```
rex hook >/dev/null 2>&1 || true
```

**`rex gate`**: called by Claude Code before each sub-agent starts, not by people. It reads the `PreToolUse` event on stdin, finds the session's open job, and compares the job's spend (its sub-agents and its main session, at list prices) with the job's spend limit. Under the limit it prints nothing and exits 0, so the call goes on through the normal permission flow. At or over the limit it prints a refusal as JSON, with `permissionDecision` set to `deny`, and exits 0. The agent reads this reason: `SuperRex spend limit reached: job <id> has spent $X of its $Y limit. Start no more agents. Finish the work in hand without new agents, then report the state blocked with the spend ($X), the limit ($Y) and the command that raises the limit, for the person to run: rex limit <id> --set <2Y>`. Every failure (no store, no open job, a busy store, bad input) also ends in no output and exit 0: the gate never blocks by mistake. It stops new starts only: agents already running keep spending, so a job can end a little over its limit. `rex init` registers it for `PreToolUse` with matcher `Agent|Task`:

```
rex gate 2>/dev/null || true
```

**`rex limit <job> [--set <dollars>]`**: print a job's spend and spend limit, or set a new limit. `--set` takes any positive amount, such as `12` or `7.50`, and replaces the old limit (it can also lower it); a bad amount exits 2. The default limit comes from the job's `--budget` and `--size`, in US dollars at list prices. Each is the agent cap of the job's size (5, 6 or 10) plus 2 for the main session, times the dollars for each agent: $1.50 for budget `low`, $4 for `medium` and $8 for `high`. A job with no stated budget uses the `medium` row.

| budget | small | medium | large |
| --- | --- | --- | --- |
| `low` | 10.50 | 12 | 18 |
| `medium` or none | 28 | 32 | 48 |
| `high` | 56 | 64 | 96 |

```console
$ rex limit 0g6fesp4se
limit_usd=28.000000
spent_usd=0.000000
source=default
$ rex limit 0g6fesp4se --set 12
limit_usd=12.000000
spent_usd=0.000000
source=set
```

`source` is `default` or `set`. Spend leaves out requests on a model with no price, so it is a lower bound. When the gate refuses a new sub-agent, the job stays open: the builder starts no agent, tells you the spend, the limit and the `rex limit <job> --set <dollars>` command, and goes on with the same job (resuming each stopped lead) after you run it. Only if you decline does the job close `blocked`. The dollars for each agent do not change with the model tier, so a job on the `xhigh` tier can pass its limit before it reaches its agent cap.

**`rex test-hook`** and **`rex test-filter`**: they keep the long output of test runs out of an agent's context. `rex test-hook` is called by Claude Code, not by people. `rex init` registers it for `PreToolUse` with matcher `Bash`, and writes the allow rules `Bash(rex test-filter)` and `Bash(set -o pipefail)`:

```
rex test-hook 2>/dev/null || true
```

When the Bash command of any session in the repo (the main session or a sub-agent) is a plain test command, such as `cargo test`, `cargo nextest run`, `pytest -q` or `go test ./...`, the hook runs it as `set -o pipefail; cargo test 2>&1 | rex test-filter`. `rex test-filter` reads the output on stdin and always exits with 0, so `pipefail` ends the command with the test command's own status. The form has no braces, quotes or subshell, because Claude Code's check of a Bash command refuses those in the default permission mode. A command with a `cd`, a pipe, a redirect, `;`, `&`, `&&`, `#`, `$(`, a backtick or `exit`, and any command that is not a test command, runs as before. The person sees the changed command in a permission prompt and in the transcript.

```console
$ printf 'running 2 tests\ntest a ... ok\ntest b ... ok\n\ntest result: ok. 2 passed; 0 failed\n' | rex test-filter
rex test-filter: every line of this run goes to /tmp/claude-501/rex-test-output-73691-1794402279206894000.log as it comes.
running 2 tests

test result: ok. 2 passed; 0 failed
rex test-filter: left out 2 passing lines of 5. To see every line, read the file named on the first line.
```

`rex test-filter` first prints the path of a file and writes every line of the run there as it arrives, so you can read all of it. Then it shows every line except the lines that a runner prints in its exact passing form, whether the run passed or failed, so no line of a failure is cut. A summary line is always kept. The note at the end gives the count: `rex test-filter: left out 2 passing lines of 5.` Summary lines are known for cargo, cargo nextest, go, pytest, jest, vitest, mocha, rspec, mix, ctest, dotnet and Maven. Passing lines are known for cargo (test lines, and the build steps before the tests), nextest, go, pytest, jest, vitest, mocha and dotnet. For another runner, every line is kept. The file is named `<TMPDIR>/rex-test-output-<pid>-<nanos>.log`, has mode 0600 (only you can read it), and a later run removes such files older than 24 hours. A command is left unchanged, so it prints in full, when it holds a `cd`, a backslash, a pipe, a redirect, `;`, `&`, `$(`, a backtick or `exit`; an output flag (`--nocapture`, `--show-output`, `--list`, `--help`, `--version`, `-s`, `-v`, `--verbose`, `--collect-only`, `--no-run` and a few more); or when the call asks to run outside the sandbox. In auto mode, Claude Code can still deny a changed call if a hook changed an earlier call in the same response. The grid agents still load `CLAUDE.md` and `AGENTS.md` (`omitClaudeMd` stays off), so the rules of your repo reach them.

**`rex status [<job>]`**: print where a job stands, for agents: one header line, then the runs under `open`, `finished` and `unconfirmed`. With no job, every `created` or `planned` job; `no open job` if there is none.

```console
$ rex status 0g51gpfx9m
job 0g51gpfx9m  created  0 open  1 finished  0 unconfirmed  incomplete
finished
  a0c8cde396de066a3  reader  model-medium-effort-low  1m12s
```

Each run line gives the agent ID, the role, the agent type, the duration, `bg` for a background run, `<parent` when a sub-agent started it, and the tokens: `in`, `cache-w`, `cache-r`, `out`, `hit` and the cost in dollars (see [Tokens](#tokens)). The header gives the job's tokens and cost, runs and main session together. This run had no transcript, so it has no tokens and the job shows `incomplete`.

For a `created` or `planned` job, from 80% of its spend limit (see `rex limit`), a line follows the header:

```
warning: spend is 93% of the $0.200 limit; at 100% new sub-agents are refused; raise it with rex limit 0g51gpfx9m --set 0.40
```

At 100% and above the line reads `warning: spend is 186% of the $0.100 limit; new sub-agents are refused; raise it with rex limit … --set 0.20`.

**`rex oms`**: print the grid agent for a set of inputs. Flags `--duration`, `--budget`, `--intelligence`, `--autonomy`, `--reliability`, `--consequence`, `--verification`, each `low|medium|high|none` (a flag left out is `none`), and `--json`. `inputs` gives the seven ratings in that order, `-` for none.

```console
$ rex oms --budget low --autonomy high --intelligence high --reliability high --consequence high --verification low
agent=model-medium-effort-high
demand=model-xhigh-effort-max
rules=T1,T2,E1,E2,E4,C1,C2,F1
conflict=budget
inputs=-LHHHHL
version=2
```

The function is version 2. A role gets the `xhigh` tier only when its consequence is `high`: with a lower consequence, the tier stops at `high` and rule `T3` shows in `rules`. This is a role of consequence `low`:

```console
$ rex oms --intelligence high --reliability high --consequence low --verification high
agent=model-high-effort-high
demand=model-high-effort-high
rules=T1,T2,T3,E1,E3,F1
conflict=none
inputs=--H-HLH
version=2
```

**`rex skills`**: the group of `list` and `check`. On its own it prints its help on stderr and exits 2.

**`rex skills list [--json]`**: print each skill of the repo's `.claude/skills/`, of the repo's skill library `.rex/skills/` and of `~/.claude/skills/` as a block of six lines. `source` is `repo`, `library` or `user`, and `path` is the absolute path of the skill's `SKILL.md`. `shadowed=yes` marks a repo skill that a user skill of the same name hides; a library skill is never shadowed. In a Git worktree, the repo's skills folder is the worktree's own when it has one, else the main checkout's. The library is the worktree's own `.rex/skills/` and then the main checkout's, so the main checkout fills in the library skills that Git does not track in the worktree; a worktree's skill hides the main checkout's skill of the same name.

```console
$ rex skills list
name=rex-code-spike
description=Throwaway code in a scratch folder that answers one yes/no question before real code depends on it. Never enters the source tree.
when_to_use=Use when plan or sketch lists an unknown, an engineer doubts an approach, crate, or outside service, or user says "spike this", "prove it works first", "quick experiment", "scratch test", "will this even work".
source=library
shadowed=no
path=/home/ana/code/shop/.rex/skills/rex-code-spike/SKILL.md
...
```

**`rex skills check <path>...`**: check skill folders or `SKILL.md` files against the rules in "Reference". It prints `ok <path>` or one line per finding, and exits 1 on any finding.

```console
$ rex skills check .claude/skills/shop-notes .rex/skills/rex-code-tdd
.claude/skills/shop-notes/SKILL.md: H4: description holds `Use when`; move the "use when" text into when_to_use
.claude/skills/shop-notes/SKILL.md: H5: when_to_use is missing
.claude/skills/shop-notes/SKILL.md: B4: .claude/skills/shop-style/SKILL.md is not a known skill
ok .rex/skills/rex-code-tdd
error: 3 findings in 1 of 2 skills
```

**`rex recall <job> [--json]`**: print up to 5 lessons from similar jobs, up to 3 orchestrators those jobs used with how they turned out and what they cost, and every orchestrator in the library whose newest version never ran. A job is similar when at least 2 of work type, size, consequence and languages are equal. While the job is `created`, the result is also stored on the job (`recorded=yes`).

```console
$ rex recall 0g51gxzkf5
job=0g51gxzkf5
recorded=yes
lessons=1
lesson feedback=1 job=0g51g6fykv score=4 status=analyzed cost_usd=0.500000 cache_hit=85% causes=missing-context text=Ask which inputs share the parser before changing its rules.
orchestrators=1
orchestrator name=solo-engineer version=1 best_match=4 runs=1 accepted=0 missed=1 cost_per_job_usd=0.500000 cache_hit=85% path=/home/ana/code/shop/.rex/orchestrators/solo-engineer/SKILL.md
unused=1
unused name=research-panel version=1 path=/home/ana/code/shop/.rex/orchestrators/research-panel/SKILL.md description=Three readers compare options and one writer recommends.
```

`cost_usd` is what the lesson's job cost at list prices, in the form of `rex job show`. `cost_per_job_usd` is the mean cost of the orchestrator's similar jobs that have a stored cost. Both are `none` when nothing is stored, and print as `≥0.120000` when requests on a model with no price are left out. `cache_hit` is the cache hit % (`85%`), or `none`. The orchestrator lines come in this order: highest `best_match`, then most `accepted`, then fewest `missed`, then lowest `cost_per_job_usd` (a full cost first, then a `≥` cost, then none), then most `runs`, then name. When two orchestrators suit the work and have equal results, the decision agent prefers the cheaper one. `--json` gives `cost_usd` and `cache_hit` on each lesson, and `cost_per_job_usd` and `cache_hit` on each orchestrator, as numbers or `null`, with `unpriced_requests` beside a cost.

**`rex jobs [--json] [--search <text>]... [--since <date>] [--until <date>]`**: list jobs, newest first, one line each. `--search` matches the request text, ignoring case; repeat it to require several. Dates are `YYYY-MM-DD`, UTC, both ends included. `--json` adds deliveries, run counts, tokens, cost and `has_feedback`.

```console
$ rex jobs --search parser
0g51g6fykv analyzed fix small medium rust 2026-11-10T18:16:54.174Z Make the order parser reject a quantity of zero.
```

**`rex watch`**: a live dashboard for people, redrawn every 2 seconds. The first screen is the job list, with the columns of `rex jobs`, the run counts, the cost and the cache hit % (see [Tokens](#tokens)). Enter on a job switches to the second screen: the job's facts and its tokens in full, then the runs, titled with the job's orchestrator skill and version (such as `pipeline v1`): the main session's row and the tree of sub-agent runs with each run's role, model, effort, duration and `in`, `cache-w`, `cache-r`, `out`, `hit` and `cost` (`~` marks a provisional total). The rows add up to the job's tokens and cost. Enter switches back. Up and Down move the selection, `q` quits. Without a terminal it exits 1: `error: rex watch needs a terminal; run it without redirecting its output`.

**`rex feedback <job> --verdict accepted|rejected (--said <text> | --said-file <path>)`**: record what the user said about a finished job, and move its status. With `rejected` it also takes `--cause` (repeatable: `human-error`, `missing-context`, `agent-skill`, `agent-model-effort`, `agent-orchestration`), `--lesson <text>` or `--lesson-file <path>` (at most 600 characters), `--evidence <text>`, `--changed-skill <name>` and `--changed-orchestrator <name>`. A cause needs a lesson, and a lesson needs a cause. `--by <who>` defaults to `user`. The `rex-feedback` skill calls it; you rarely need to.

```console
$ rex feedback 0g51g6fykv --verdict rejected --said "It also rejected negative numbers in the import." \
    --cause missing-context --lesson "Ask which inputs share the parser before changing its rules."
job=0g51g6fykv
feedback=1
previous=delivered
status=analyzed
```

**`rex codebase`**: print a tree of the working folder, skipping what Git ignores. `--only code` (every language `rex` knows) or `--only rust|go|c|cpp|python|js|ts|shell|java|kotlin|scala|csharp|swift|objc|dart|php|ruby|lua|haskell|elixir|r|sql|zig|perl` keeps source files; `--with-context` adds the first sentence of each source file's opening comment; `--for-human` writes `CODEBASE.md` instead.

```console
$ rex codebase --only code --with-context
main.rs - Entry point of the shop binary.
src/
  order.rs - Parses the order file and checks each quantity.
```

**`rex smells`**: list every `SMELL` flag comment in the Rust files as `path:line`, then the count.

```console
$ rex smells
src/order.rs:3
Code smells: 1
```

**`rex commands`**: list every command with its full path and description.

```console
$ rex commands
rex init          — Set SuperRex up in a repo. Safe to run twice
rex job           — Open, plan, close and show a job
...
```

## The life of a job

| Status | Meaning |
| --- | --- |
| `created` | The request is recorded and classified. |
| `planned` | The orchestrator and the role table are recorded. |
| `running` | Shown, never stored: a `planned` job with sub-agents open. It needs the hooks. |
| `delivered` | The work is finished. How each repo was delivered is in its delivery. No verdict yet. |
| `blocked` | Closed, and waiting on something outside the session, such as a person's approval on a protected main. |
| `failed` | The work stopped and cannot go on without being redone. |
| `cancelled` | Stopped by the user. |
| `accepted` | The user confirmed it hit the mark. |
| `rejected` | The user said it missed. An analysis is due. |
| `analyzed` | The cause and the lesson are recorded. |

The status changes only through `rex` commands, so it works the same with telemetry off. Anything not in this table is refused with exit 1, and nothing is written.

| Command | Status before | Status after |
| --- | --- | --- |
| `rex job create` | (no job) | `created` |
| `rex job plan` | `created` | `planned`. Any other status: refused (a job is planned once). |
| `rex job close` | `created` | `blocked`, `failed` or `cancelled`. `delivered` is refused: record the plan first. |
| `rex job close` | `planned` or `blocked` | `delivered`, `blocked`, `failed` or `cancelled` |
| `rex job close` | any other | refused: a closed job is not closed again |
| `rex feedback`, any verdict | `created` or `planned` | refused: close the job first |
| `rex feedback --verdict accepted` | `delivered` or `accepted` | `accepted` |
| `rex feedback --verdict rejected`, no cause and lesson | `delivered` or `accepted` | `rejected` |
| `rex feedback --verdict rejected` with cause and lesson | `delivered`, `accepted` or `rejected` | `analyzed` |
| `rex feedback` without cause and lesson | `rejected` | refused: the analysis is due |
| `rex feedback`, any verdict | `blocked`, `failed`, `cancelled` or `analyzed` | unchanged; the feedback is still recorded |

Each delivery has one outcome: `merged` (one squash commit on main), `pr-open` (no merge agreed; the pull request waits for a person), `kept-local` (no merge agreed and no pull request possible; the branch is kept), or `blocked` (a merge was agreed but could not be completed). Every outcome but `merged` carries a note that says what remains; `merged` needs `merge_commit`, and `pr-open` needs `pr`. A delivery without `merge_agreed` takes the job's `--merge` answer. If any delivery is `blocked`, the job closes `blocked`, and `--status delivered` is refused. When the rest is done, for example after a person merged the pull request, a later `rex job close` on the same job records it; repos it does not name keep their deliveries.

Feedback is always started by you: run `/rex-feedback` and say what was wrong or right. The skill finds the job, reads the record, finds the cause, may change a skill or an orchestrator, and records it all with one `rex feedback` call.

## Telemetry

The hooks record three events: a sub-agent starts, a call to a sub-agent completes (in the parent), and a sub-agent stops. Claude Code runs the hook by itself, and the model never sees it, so it costs no tokens. `rex hook` only appends a line to `.rex/events.jsonl`; any `rex` command that reads a job takes the new lines into the store. Taking a line in twice changes nothing.

Claude Code also sends a stop event for each call it makes on its own, such as the reply it suggests after a turn. Such an event has an empty agent type, and `rex` records no run for it.

A run record holds the sub-agent's ID and its parent, its role (from the brief's tag block), the agent type, the model and effort that actually ran, the brief, the skills it opened, start and stop times, tool uses, lines added and removed, its final message, and its tokens. When an event is lost, `rex` fills the gaps from the transcript and its metadata file.

Tokens are summed from the transcripts, never from the hook: input, output, cache read and cache creation. A total is **provisional** (`~`) while it may still grow. It becomes final only after the job is closed and two later reads, at least 10 seconds apart, find the transcript unchanged. A job's total includes the main session's own share, counted from its requests between `rex job create` and the close.

A run is **open** while it shows activity in the last 15 minutes, **finished** when its stop is known, and **unconfirmed** when it is neither. A job is **incomplete** when its record may miss facts: a run is unconfirmed, a finished run has no tokens, a transcript was damaged or could not be read, or the main session's tokens are unknown.

### Tokens

Every API call sends the whole conversation again, and most of it comes from the prompt cache. So a long run reads its context again on each call, and cache reads make up most of its tokens. They cost about a tenth of the input price. `rex` keeps the four counts that Claude Code writes apart, and never adds them into one figure:

| Shown as | `rex job show` key | What it counts |
| --- | --- | --- |
| `in` | `input` | Prompt tokens sent without the cache. |
| `cache-w` | `cache_write` | Prompt tokens written to the cache. |
| `cache-r` | `cache_read` | Prompt tokens read from the cache, once for each call. |
| `out` | `output` | Tokens the model wrote. |
| `hit` | `cache_hit` | `cache-r` as a whole percent of all prompt tokens (`in` + `cache-w` + `cache-r`), cut, not rounded; `–` or `none` with no prompt token. |
| `cost` / `$…` | `cost_usd` | The tokens' cost in US dollars at Anthropic's list prices, each request at the price of the model that ran it. `≥` in front, and `unpriced_requests` above 0, when some requests ran on a model with no price in `rex`, so the figure leaves them out. |

A job's tokens and cost are its runs and its main session together. `--json` gives the four counts as `input`, `output`, `cache_read` and `cache_creation`, and the cost as `{"usd": …, "unpriced_requests": …}`.

The cost is what the requests would cost on the Claude API, whatever plan pays for them; on a subscription it is a measure, not a bill. The prices are in `src/capture/price.rs`: per model, the input and output price, the cache read price, and cache writes at 1.25 times the input price for 5 minutes and 2 times for 1 hour, as each request's usage splits them; fast mode doubles them, and Claude Haiku 5.5 has a second rate card above 100,000 prompt tokens. Claude Code writes no cost per sub-agent, so `rex` works it out from each request's usage. A test checks it against the cost Claude Code reports for a recorded session.

To turn telemetry off, run `rex init --no-hooks` in a new repo. In a repo that has the hooks, delete the three entries that run `rex hook` from `.claude/settings.local.json`. Jobs are still recorded; `running` is then never shown.

## Delivery

`rex-deliver` does every Git and GitHub step, once for each repo the job changes:

- One job branch per repo, named `<type>/<slug>-<job-id>` (for example `fix/reject-zero-quantity-0g51g6fykv`; `research` and `other` use `chore`). Parallel writers get unit branches `<job branch>--<unit>` in worktrees under `.rex/worktrees/<job-id>/`. The job works in the main checkout only when it is clean and on main; otherwise in `.rex/worktrees/<job-id>/root`.
- One pull request per repo. Its body, and the squash commit, end with the line `Rex-Job: <job-id>`.
- A squash merge only when a merge was agreed. With no merge agreed, the pull request is left open and nothing is removed.
- After a confirmed merge, a guarded clean-up: only branches and worktrees whose name carries the job ID are removed, a local branch only when its tip is part of what was merged, nothing is force-pushed, and a worktree with uncommitted changes is kept.
- A main that needs a person's approval is never bypassed: the outcome is `blocked`, and the note says what to do.
- Failing checks are fixed on the job branch and main is merged in (never a rebase), for at most 3 rounds, with at most 30 minutes of waiting for checks in each round. Past that the outcome is `blocked`.
- With no remote, or no `gh`, the same steps run without a pull request and end in a local squash merge, again only when a merge was agreed.

Only a person can start `/rex-deliver`, for example for the clean-up after a person merged a blocked pull request; the builder and the leads of parallel writers read its files by path. Two more skills can be called by hand: `/rex-pr-review` (an optional review of a pull request by an agent that wrote none of it; it posts one comment with the verdict PASS or FIX and changes nothing), and `/rex-feedback`. Ask the builder for a review and it starts a fresh grid agent that reads `rex-pr-review`.

## Reference

**Output.** stdout holds the result and nothing else; warnings and errors go to stderr. Plain text, `key=value` lines in a fixed order, the most needed value first. `--json`, where a command has it, prints one JSON line with at least the facts of the text form.

**Exit codes.** `0` done, also when there was nothing to do. `1` the command failed: the store, a job's state, a file, or a check that found something. `2` the command line is wrong. Every exit that is not 0 writes one line that starts with `error: ` on stderr, except `rex job` and `rex skills` with no sub-command, which print their help. `rex hook` always exits 0.

**Times** print as stored: UTC with milliseconds, `2026-11-10T18:16:54.174Z`. **Job IDs** are 10 lower-case letters and digits, such as `0g51g6fykv`; they sort by time.

**Files.**

| Path | What |
| --- | --- |
| `.rex/rex.db` | The SQLite store: jobs, roles, deliveries, runs, feedback, the orchestrator index. Read it with `sqlite3`, but write it only through `rex`. |
| `.rex/events.jsonl` | The hook event log. It is never truncated. |
| `.rex/skills/<name>/` | The skill library: one folder with a `SKILL.md` per skill that agents read by path. Claude Code does not list it. |
| `.rex/orchestrators/<name>/` | The orchestrator library: one folder with a `SKILL.md` per orchestrator. A changed folder gets a new version at the next plan or recall. |
| `.rex/plans/<job-id>/` | The plan that a plan job writes, `plan.md`. |
| `.rex/worktrees/<job-id>/` | Worktrees of a job, removed after its merge. |

**How `rex` finds the store.** Every command except `rex init` uses the first of these that holds a `rex.db`: `REX_STORE` (the path of a `.rex` folder, used as is and never skipped), `CLAUDE_PROJECT_DIR`, then the working folder. From a sub-folder or a Git worktree, `rex` walks up to the main checkout. Claude Code sets `CLAUDE_PROJECT_DIR` for hooks only, not in the shell an agent uses, so from a sibling repo set the variable for the command: `REX_STORE=/home/ana/code/shop/.rex rex job show 0g51g6fykv`.

**The tag block.** Every brief that starts a sub-agent begins with two lines and a blank line. `rex` reads the role from it; `Rex-Job` is a cross-check and never moves a run to another job. Nothing fails without it, but the run then has no role.

```
Rex-Job: 0g51g6fykv
Rex-Role: engineer

<the rest of the brief>
```

**The skill header.** A skill is a folder with a `SKILL.md` that opens with a YAML header:

```yaml
---
name: shop-notes
description: How this team writes release notes.
when_to_use: Use when you write release notes, or when the user says "release notes".
---
```

An agent reads all of `SKILL.md`, so it holds only the rules an agent needs every time and a table that says when to read each sibling file. A sibling file sits in the skill's folder and holds the text for one step or one case, and `SKILL.md` names it by its path, such as `.claude/skills/rex-deliver/parallel-writers.md`. An agent reads it only when it reaches that step or case. A skill can sit below its folder in subfolders too, as the builder's stage files do.

`rex skills check` applies these rules. The IDs never change meaning.

| Rule | Passes when |
| --- | --- |
| H1 | Line 1 is `---` and a later `---` line closes the header. |
| H2 | The header is valid YAML and a mapping. |
| H3 | `name` equals the folder name: lower-case letters and digits joined by single hyphens, at most 64 characters. |
| H4 | `description` is not empty and holds no "Use when" text. |
| H5 | `when_to_use` is not empty and starts with `Use when`. |
| H6 | A plain one-line value is not cut short by YAML (` #` starts a comment). |
| H7 | `description` and `when_to_use` together are at most 1,536 characters. |
| H8 | Every header key is one Claude Code knows. |
| H9 | `description` and `when_to_use` together have at most 60 words. |
| B1 | There is text after the header. |
| B2 | `SKILL.md` has at most 500 lines. |
| B3 | Each relative link to a `.md` file points to a file inside the skills folder. |
| B4 | Each `.claude/skills/<name>/<file>.md` or `.rex/skills/<name>/<file>.md` named in the text is a Markdown file of a known skill in that folder: its `SKILL.md`, or a file in its folder or below it. The known skills are those that `rex skills list` finds and those beside the checked skill. A repo with no `.claude/skills/`, or no `.rex/skills/`, gets the skills of this `rex`'s bundle for that folder instead, as `rex init` would install them. A path to a skill in the other folder is a finding. A path with a placeholder such as `<name>` is a template and is not checked. |
| B5 | The text after the header of `SKILL.md` has at most 1,500 words, counted as `wc -w` counts them. |

## Skills and the agent grid

`rex init` installs 28 skills in two folders. Claude Code lists each skill in `.claude/skills/` in every session, so only 6 are there: the ones that you start with `/<name>`, and one that a session must find by itself. The other 22 are the skill library in `.rex/skills/`, which Claude Code does not list, so they cost no context in a session. The decision agent finds them with `rex skills list`, and each brief names the ones its agent reads, by path.

In `.claude/skills/`:

- **The builder:** `orchestration-builder`, which only you start, with `/orchestration-builder <request>` or `/orchestration-builder Execute plan <path>`. Its three stages and the challenge of their decision are files in its folder, under `stages/`: `1-skills/` (with the assignment table `assign.md`), `2-work-breakdown/`, `3-orchestration/` and `challenge/`. Each stage file holds at most 1,500 words.
- **Delivery:** `rex-deliver` (only you start it), `rex-pr-review` and `rex-feedback`.
- **`rex-grill-me`:** it questions you about a plan or a design until the two of you agree on each decision. Start it with `/rex-grill-me`.
- **The router, `rex-rules`:** a session that writes or reviews code without the builder loads it by itself, or you start it with `/rex-rules`. It tells the session to read the four rulebooks that every task that writes code gets, then the rows of the assignment table that fit its work.

In the skill library, `.rex/skills/`:

- **Rulebooks:** 19 `rex-code-*` skills for writing code. Most are written for Rust, and they apply to every language: an agent applies the intent of each rule in that language's idiom, and a gap skill covers what the language lacks. A gap skill is named `rex-gap-<language>`, with the `--only` name of `rex codebase` (`rex-gap-go`, `rex-gap-cpp`), and a job that needs one and finds none can have it written in `.rex/skills/`. They cover philosophy, ergonomics, comments, errors and failure policy, crates, folders, architecture, smells, TDD and the four kinds of test, design sketch, spike, validation plan, priorities and latency. `rex-code-latency` is a reference for hot paths that run in nanoseconds to microseconds: an agent measures first, then reads only the one or two files for its problem.
- **Plan review:** `rex-codex-adversarial-review`, which every plan job runs before its plan is final. Outside a job, ask the session to read `.rex/skills/rex-codex-adversarial-review/SKILL.md` and run it on a plan or a code change.
- **`rex-mermaid-diagrams`:** how to draw diagrams in Mermaid.
- **`rex-skill-language`:** the language every skill and agent instruction is written in.

Run `rex skills list` to see each skill's description and when to use it. No library skill has a `/<name>` command; to use one outside a job, ask the session to read its `SKILL.md`.

**The grid** is 20 agents in `.claude/agents/`, named `model-<low|medium|high|xhigh>-effort-<low|medium|high|xhigh|max>`. The tiers are `haiku`, `sonnet`, `opus` and `fable`. An agent runs with exactly the model and effort in its file. Each file also holds `disallowedTools: Skill`, so a grid agent has no Skill tool and no list of skills: it reads only the skill files its brief names.

**How `rex oms` picks one.** It is a fixed function of seven inputs, each low, medium, high or none (not relevant). Duration, budget and autonomy belong to the job and are given only when you state them; intelligence, reliability, consequence and verification are rated for each role. The tier starts from intelligence and goes up one step when the stakes (the higher of reliability and consequence) are high. That step reaches the top tier, `xhigh`, only when consequence itself is high. Otherwise the tier stops at `high`, and rule `T3` is printed. The effort starts from the stakes, goes up one level when the result is hard to check and down one when it is easy, and shifts one level with autonomy. A low budget caps the pair at `model-medium-effort-medium` and a medium one at `model-high-effort-high`; a low duration caps the effort at medium and a medium one at high. With high stakes the result is never below `model-medium-effort-high`, and the cap that lost is printed as `conflict`. All inputs none gives `model-medium-effort-medium`. In the `rex oms` example above, the demand was `model-xhigh-effort-max`; the low budget capped it at `model-medium-effort-medium`, and the high-stakes floor raised its effort to high.

## Known limits

- A Git worktree that an earlier session made cannot be removed by a later session inside the sandbox. `rex-deliver` keeps it and its branch, records them in the delivery note, and gives two commands to run in your own shell:

  ```sh
  git -C <repo> worktree remove <path>    # if Git says the path is gone: git -C <repo> worktree prune
  git -C <repo> merge-base --is-ancestor <branch> <pr head> && git -C <repo> branch -D <branch>
  ```

- Linux without `bwrap` or `socat` was not tested live; only the detection is tested. `sandbox.failIfUnavailable` (Claude Code refuses to start without its sandbox) comes from the Claude Code documentation and was not tested on any system, and neither was the trivy cache path on Linux.
- A main branch that needs a person's approval was not tested live.
- From a sibling repo, `rex` needs `REX_STORE` (see "How `rex` finds the store").
- A request that Claude Code bills but writes to no transcript is not counted in the tokens, so a total can be lower than Claude Code's own figure.
- A session started in a worktree outside the main checkout's folder is not tested. Start the session in the main checkout.
- `rex smells` reads Rust files only.
- A filtered test run ends when its output ends. A test that leaves a child process running with the output still open makes the run wait for that child, as `cargo test | tail` would.
- An existing `.rex/rex.db` from an earlier build has the old role table: delete it and run `rex init` again.
- The spend limit stops new sub-agents only. Agents that already run keep spending, so a job can end over its limit. A job whose requests all ran on models with no price never reaches it. The default limits are estimates from list prices, not measured costs. `rex gate` and `rex test-hook` come from the hooks in `.claude/settings.local.json`: a `rex` on `PATH` older than your repo's setup has neither command, and then the hooks do nothing.
- The changed test command was run with `claude -p` in the default permission mode, with an allow rule for the test command: it ran with no prompt. `set -o pipefail` is set in the shell of that one Bash call. If `rex` is on the hook's `PATH` but not on the shell's, the changed command fails with 127 and its output is lost.

## Development

Build with `cargo build` and test with `cargo test`. The tests live in three folders: `tests/integration/` (the binary end to end), `tests/contract/` (the seams between modules), and `tests/fitness/` (rules of the whole crate, such as what the binary embeds). `tests/fixtures/` holds recorded Claude Code sessions, so no test needs a live Claude Code.

This repo has no CI service. Its checks are git hooks in `.pre-commit-config.yaml`; run `pre-commit install` once to set up both hooks. At commit: the file checks and trivy. At push: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, `cargo check` with the oldest Rust that `Cargo.toml` claims (the hook installs that Rust when it is missing), and `cargo package`, with the tests run again from inside the package. The push checks run on the work tree, not on the commits being pushed, so push from the branch that is checked out.

Two rules hold for every source file: at most 500 lines, and a module comment of one or two lines at the top that says what the file is for (`rex codebase --only code --with-context` shows them). The skills and agents in `.claude/skills/`, `.claude/agents/` and `.rex/skills/` are source in this repo and are committed: this repo uses the same layout that `rex init` installs. The build embeds them in the binary, and `rex init` installs them from there. Each skill is a short `SKILL.md` and the sibling files it points to, and `cargo test` checks every bundled skill with the rules of `rex skills check`, each stage file of the builder for at most 1,500 words, and each grid agent for `disallowedTools: Skill`.

**Licence:** MIT. See [LICENSE](LICENSE).
