<p align="center">
  <img src="https://raw.githubusercontent.com/coderaidershaun/rex-skills/main/static/logo.png" alt="SuperRex logo" width="200">
</p>

# SuperRex

This is version 0.1.0 of SuperRex.

SuperRex is two things that work together inside your repo:

- **Skills.** You give Claude Code a request and ask for `orchestration-builder`. The skill works out how to do the job: which skills it needs, what can run in parallel, how the agents are organised, and which model and effort each one runs at. Then it runs the job with Claude Code's own sub-agents and delivers it.
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

The binary is `rex`. rex-skills installs a binary named `rex` too: if it is installed, run `cargo uninstall rex-skills` first, and make sure that the `rex` first on your `PATH` is this one (`rex --version` prints `rex 0.1.0`).

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
rex init: 64 written, 0 skipped
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
| `.claude/skills/` | The 28 skills (section "Skills and the agent grid"). | No |
| `.claude/agents/` | The 20 grid agents. | No |
| `.claude/settings.local.json` | The hooks and the safety settings, merged into your file. | No |
| `.rex/` | The store `rex.db`, and the empty library folder `orchestrators/`. | No |
| `.gitignore` | A block that starts with `# rex:ignore superrex` and ignores the four paths above and `/.claude/worktrees/`. | Yes |
| `AGENTS.md` | Two sections at the top, after the marker `<!-- rex:codebase -->`: how to use `rex codebase` and `rex smells`. | Yes |
| `.pre-commit-config.yaml` | Checks for large files, merge markers, TOML, YAML, private keys and white space; gitleaks at commit; trivy at push; `cargo fmt`, `cargo clippy` and (at push) `cargo test` for Rust files. Only if you have no such file. | Yes |

Then commit the three project files. A job works in the main checkout only when it is clean:

```sh
git add AGENTS.md .gitignore .pre-commit-config.yaml && git commit -m "chore: set up SuperRex"
```

**Safe to run twice.** A second run changes no file in the repo. `rex init` never replaces what you have:

- A skill or agent file that differs from this version of `rex` is skipped and counted (`(1 differs from this version; use --force to overwrite)`). `--force` overwrites those files, and only those: it never touches the settings file, `AGENTS.md`, `.gitignore` or the pre-commit config.
- Your settings file is merged: missing keys and list entries are added, your keys and order stay. A value of yours that differs is kept, with a warning.
- `AGENTS.md` and `.gitignore` get their block once, found by its marker. An existing pre-commit config is never replaced.
- Nothing is written through a symbolic link. Skills of your own, and an older rex-skills install, are left alone. `rex init` removes nothing, does not touch your user-level Claude Code settings, and writes no `CLAUDE.md`.
- The ignore block hides whole folders, so a skill of your own in `.claude/skills/` is hidden from Git too. To keep one in Git, run `git add -f <path>`.

**`--no-hooks`** leaves out the hooks, so no telemetry is recorded. Everything else is written. Hook entries from an earlier run are not removed: delete the entries that run `rex hook` to turn telemetry off.

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
| `sandbox.filesystem.allowWrite` | The five Cargo cache folders (not `~/.cargo/bin`), `~/.cache/pre-commit/patch*`, and the trivy cache (`~/Library/Caches/trivy` on macOS, `~/.cache/trivy` on Linux). |
| `sandbox.filesystem.denyRead: ["~/.aws"]` | AWS credentials cannot be read. |
| `permissions.additionalDirectories` | The parent folder of the repo, as an absolute path, so that agents can edit sibling repos. Left out, with a warning, when it holds your home folder. |
| `permissions.blockWritesOutsideWorkingDirectories: true` | The file tools cannot write outside the working folders. |
| `permissions.deny` | `Bash(sudo *)`, `Bash(git push --force*)`, `Bash(cargo publish*)`, `Bash(gh pr merge*--admin*)` (a protected main is never bypassed). |

The network stays open, but only through the sandbox's proxy. In bypass mode a host list restricts nothing, so `rex init` writes none. A program that uses the proxy, such as `curl`, `git`, `cargo` or `gh`, reaches any host. A program that opens its own connection, such as a WebSocket client, a database driver or plain `ssh`, cannot connect inside the sandbox.

Agents cannot edit the Claude Code settings files; `rex init` can, because you run it. A sandboxed command can still delete the repo itself: Git recovers the code, but the store in `.rex/` is not in Git.

### Commands outside the sandbox

Because of the proxy limit, an agent can run a command again outside the sandbox. The builder skill tells agents to do this only for a command that failed because its program opens its own network connection, such as a live test of a WebSocket, and to name each such command in their report.

A command outside the sandbox runs with your full access. In bypass mode Claude Code approves it without a prompt; in the default mode it asks you first. So the sandbox stops mistakes, but it does not stop an agent that chooses to leave it. The file tools still cannot write outside the working folders.

**`--strict`** writes `sandbox.allowUnsandboxedCommands: false`, and then no command leaves the sandbox. A live test that opens its own connection cannot run in a session, so the agent gives you the command to run yourself.

A stricter value of yours is never made weaker. If the settings file already holds `false`, for example from an earlier version of `rex`, a run without `--strict` keeps it and warns. Set it to `true` by hand to let agents run such commands. The other way round, a kept `true` fails a `--strict` run, like any weaker safety value.

## How a job runs

Start Claude Code in the main checkout and give it a request with `/orchestration-builder <request>`, or ask it to use `orchestration-builder`. The session then:

1. **Classifies** the request: work type, size, consequence, languages.
2. **Agrees on the merge.** If the request does not say whether the work may be merged to main, it asks you. This is the one rule every job keeps.
3. **Opens the job** with `rex job create`, which prints the job ID.
4. **Recalls** lessons from similar jobs and the orchestrators they used (`rex recall`).
5. **Lists the skills** in the repo and in your own skills folder (`rex skills list`).
6. **Stage 1, `orchestration-nuance-selection`:** chooses the skills for the job. It writes a new skill only when a three-part test passes.
7. **Stage 2, `orchestration-work-breakdown`:** splits the request into tasks and finds what can run in parallel.
8. **Stage 3, `orchestration-selection`:** reuses an orchestrator from `.rex/orchestrators/` or writes a new one, defines the roles, and rates the inputs that pick each role's model.
9. **Records the plan** with `rex job plan`. `rex` picks the grid agent for each role.
10. **Runs** the orchestrator on a job branch, never on main. Leads and workers are Claude Code sub-agents; the hooks record each one.
11. **Delivers** each changed repo with `rex-deliver`: pull request, and a squash merge if one was agreed.
12. **Closes** the job with `rex job close`, then tells you the result in a few lines: what was done, and for each repo whether it was merged, left as an open pull request, kept locally, or blocked, with what you must do.

The session is the orchestrator and runs at your model and effort. A small job can be one task and one worker. If an agent inside a job loads the builder again, no second job opens: its work is recorded under the open one.

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

**`rex job plan <job> --plan-file <path|-> [--json]`**: record the skills, the work breakdown, the orchestrator and the roles, and move the job to `planned`. A job is planned once. The orchestrator folder `.rex/orchestrators/<name>/` must exist. Each role gives four inputs, each `low`, `medium`, `high` or `none`, or `"session": true` to run at the session's model.

```console
$ rex job plan 0g51g6fykv --plan-file - <<'EOF'
{"orchestrator":"solo-engineer","skills":["rex-code-tdd"],"breakdown":"1. Reject zero in the parser; test.",
 "roles":[{"role":"engineer","intelligence":"medium","reliability":"high","consequence":"medium","verification":"high"},
          {"role":"skill-writer","session":true}]}
EOF
job=0g51g6fykv
status=planned
orchestrator=solo-engineer
orchestrator_version=1
role.engineer=model-high-effort-high
role.skill-writer=session
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
```

**`rex job show [<job>] [--json]`**: print one job's record: classification, preferences, merge answer, plan, roles with their model selection, deliveries, sub-agent runs, tokens, request, breakdown and the recalled experience. With no job, it shows the open job of `$CLAUDE_CODE_SESSION_ID`. `--json` adds the transitions, each run's brief and final message, and the feedback.

```console
$ rex job show 0g51g6fykv
job=0g51g6fykv
status=delivered
...
role.engineer=model-high-effort-high inputs=--M-HMH demand=model-high-effort-high rules=T1,T2,E1,E3,F1 conflict=none oms_version=1
delivery.1=merged branch=fix/reject-zero-quantity-0g51g6fykv pr=none merge_agreed=yes merge_commit=53622c9d5c38e5adfd47454e2cedcdc3f0a24feb repo=/home/ana/code/shop
request:
  Make the order parser reject a quantity of zero.
```

**`rex hook`**: called by Claude Code, not by people. It appends the hook event on stdin as one line to `.rex/events.jsonl`, prints nothing, and always exits 0. `rex init` registers it for `SubagentStart`, `SubagentStop`, and `PostToolUse` with matcher `Agent|Task`, each as this command with a timeout of 10 seconds:

```
rex hook >/dev/null 2>&1 || true
```

**`rex status [<job>]`**: print where a job stands, for agents: one header line, then the runs under `open`, `finished` and `unconfirmed`. With no job, every `created` or `planned` job; `no open job` if there is none.

```console
$ rex status 0g51gpfx9m
job 0g51gpfx9m  created  0 open  1 finished  0 unconfirmed  incomplete
finished
  a0c8cde396de066a3  reader  model-medium-effort-low  1m12s
```

Each run line gives the agent ID, the role, the agent type, the duration, `bg` for a background run, `<parent` when a sub-agent started it, and the tokens. This run had no transcript, so it has no tokens and the job shows `incomplete`.

**`rex oms`**: print the grid agent for a set of inputs. Flags `--duration`, `--budget`, `--intelligence`, `--autonomy`, `--reliability`, `--consequence`, `--verification`, each `low|medium|high|none` (a flag left out is `none`), and `--json`. `inputs` gives the seven ratings in that order, `-` for none.

```console
$ rex oms --budget low --autonomy high --intelligence high --reliability high --consequence high --verification low
agent=model-medium-effort-high
demand=model-xhigh-effort-max
rules=T1,T2,E1,E2,E4,C1,C2,F1
conflict=budget
inputs=-LHHHHL
version=1
```

**`rex skills`**: the group of `list` and `check`. On its own it prints its help on stderr and exits 2.

**`rex skills list [--json]`**: print each skill of the repo's `.claude/skills/` and of `~/.claude/skills/` as a block of six lines. `shadowed=yes` marks a repo skill that a user skill of the same name hides.

```console
$ rex skills list
name=orchestration-nuance-selection
description=This is stage 1 of the orchestration builder. It lists the skills that are available and selects the skills that a job uses. It writes a new skill only when a three-part test passes. The usual result is no new skill.
when_to_use=Use when `orchestration-builder` reaches stage 1, or when an agent must select skills for a piece of work.
source=repo
shadowed=no
path=/home/ana/code/shop/.claude/skills/orchestration-nuance-selection/SKILL.md
...
```

**`rex skills check <path>...`**: check skill folders or `SKILL.md` files against the rules in "Reference". It prints `ok <path>` or one line per finding, and exits 1 on any finding.

```console
$ rex skills check .claude/skills/shop-notes .claude/skills/rex-code-tdd
.claude/skills/shop-notes/SKILL.md: H4: description holds `Use when`; move the "use when" text into when_to_use
.claude/skills/shop-notes/SKILL.md: H5: when_to_use is missing
.claude/skills/shop-notes/SKILL.md: B4: .claude/skills/shop-style/SKILL.md is not a known skill
ok .claude/skills/rex-code-tdd
error: 3 findings in 1 of 2 skills
```

**`rex recall <job> [--json]`**: print up to 5 lessons from similar jobs, up to 3 orchestrators those jobs used with how they turned out, and every orchestrator in the library whose newest version never ran. A job is similar when at least 2 of work type, size, consequence and languages are equal. While the job is `created`, the result is also stored on the job (`recorded=yes`).

```console
$ rex recall 0g51gxzkf5
job=0g51gxzkf5
recorded=yes
lessons=1
lesson feedback=1 job=0g51g6fykv score=4 status=analyzed causes=missing-context text=Ask which inputs share the parser before changing its rules.
orchestrators=1
orchestrator name=solo-engineer version=1 best_match=4 runs=1 accepted=0 missed=1 path=/home/ana/code/shop/.rex/orchestrators/solo-engineer/SKILL.md
unused=1
unused name=research-panel version=1 path=/home/ana/code/shop/.rex/orchestrators/research-panel/SKILL.md description=Three readers compare options and one writer recommends.
```

**`rex jobs [--json] [--search <text>]... [--since <date>] [--until <date>]`**: list jobs, newest first, one line each. `--search` matches the request text, ignoring case; repeat it to require several. Dates are `YYYY-MM-DD`, UTC, both ends included. `--json` adds deliveries, run counts, tokens and `has_feedback`.

```console
$ rex jobs --search parser
0g51g6fykv analyzed fix small medium rust 2026-11-10T18:16:54.174Z Make the order parser reject a quantity of zero.
```

**`rex watch`**: a live dashboard for people, redrawn every 2 seconds. The first screen is the job list, with the columns of `rex jobs`, the run counts and the tokens. Enter on a job switches to the second screen, its tree of sub-agent runs with each run's role, model, effort, duration and tokens (`~` marks a provisional total); Enter switches back. Up and Down move the selection, `q` quits. Without a terminal it exits 1: `error: rex watch needs a terminal; run it without redirecting its output`.

**`rex feedback <job> --verdict accepted|rejected (--said <text> | --said-file <path>)`**: record what the user said about a finished job, and move its status. With `rejected` it also takes `--cause` (repeatable: `human-error`, `missing-context`, `agent-skill`, `agent-model-effort`, `agent-orchestration`), `--lesson <text>` or `--lesson-file <path>` (at most 600 characters), `--evidence <text>`, `--changed-skill <name>` and `--changed-orchestrator <name>`. A cause needs a lesson, and a lesson needs a cause. `--by <who>` defaults to `user`. The `rex-feedback` skill calls it; you rarely need to.

```console
$ rex feedback 0g51g6fykv --verdict rejected --said "It also rejected negative numbers in the import." \
    --cause missing-context --lesson "Ask which inputs share the parser before changing its rules."
job=0g51g6fykv
feedback=1
previous=delivered
status=analyzed
```

**`rex codebase`**: print a tree of the working folder, skipping what Git ignores. `--only code` (every language `rex` knows) or `--only rust|go|c|cpp|python|js|ts|shell` keeps source files; `--rust-only` is `--only rust`; `--with-context` adds the first sentence of each source file's opening comment; `--for-human` writes `CODEBASE.md` instead.

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

Three skills can also be called by hand: `/rex-deliver` (for example the clean-up after a person merged a blocked pull request), `/rex-pr-review` (an optional review of a pull request by an agent that wrote none of it; it posts one comment with the verdict PASS or FIX and changes nothing), and `/rex-feedback`.

## Reference

**Output.** stdout holds the result and nothing else; warnings and errors go to stderr. Plain text, `key=value` lines in a fixed order, the most needed value first. `--json`, where a command has it, prints one JSON line with at least the facts of the text form.

**Exit codes.** `0` done, also when there was nothing to do. `1` the command failed: the store, a job's state, a file, or a check that found something. `2` the command line is wrong. Every exit that is not 0 writes one line that starts with `error: ` on stderr, except `rex job` and `rex skills` with no sub-command, which print their help. `rex hook` always exits 0.

**Times** print as stored: UTC with milliseconds, `2026-11-10T18:16:54.174Z`. **Job IDs** are 10 lower-case letters and digits, such as `0g51g6fykv`; they sort by time.

**Files.**

| Path | What |
| --- | --- |
| `.rex/rex.db` | The SQLite store: jobs, roles, deliveries, runs, feedback, the orchestrator index. Read it with `sqlite3`, but write it only through `rex`. |
| `.rex/events.jsonl` | The hook event log. It is never truncated. |
| `.rex/orchestrators/<name>/` | The orchestrator library: one folder with a `SKILL.md` per orchestrator. A changed folder gets a new version at the next plan or recall. |
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
| B1 | There is text after the header. |
| B2 | `SKILL.md` has at most 500 lines. |
| B3 | Each relative link to a `.md` file points to a file inside the skills folder. |
| B4 | Each `.claude/skills/<name>/SKILL.md` named in the text is a known skill. |

## Skills and the agent grid

`rex init` installs 28 skills:

- **The builder:** `orchestration-builder` (the entry skill) and its three stages `orchestration-nuance-selection`, `orchestration-work-breakdown` and `orchestration-selection`.
- **Delivery:** `rex-deliver`, `rex-pr-review` and `rex-feedback`.
- **Rulebooks:** 18 `rex-code-*` skills for writing code, mostly Rust: philosophy, ergonomics, comments, errors and failure policy, crates, folders, architecture, smells, TDD and the four kinds of test, design sketch, spike, validation plan and priorities. Two utilities go with them: `rex-grill-me` and `rex-mermaid-diagrams`. Run `rex skills list` to see each one's description and when to use it.
- **`rex-skill-language`:** the language every skill and agent instruction is written in.

**The grid** is 20 agents in `.claude/agents/`, named `model-<low|medium|high|xhigh>-effort-<low|medium|high|xhigh|max>`. The tiers are `haiku`, `sonnet`, `opus` and `fable`. An agent runs with exactly the model and effort in its file.

**How `rex oms` picks one.** It is a fixed function of seven inputs, each low, medium, high or none (not relevant). Duration, budget and autonomy belong to the job and are given only when you state them; intelligence, reliability, consequence and verification are rated for each role. The tier starts from intelligence and goes up one step when the stakes (the higher of reliability and consequence) are high. The effort starts from the stakes, goes up one level when the result is hard to check and down one when it is easy, and shifts one level with autonomy. A low budget caps the pair at `model-medium-effort-medium` and a medium one at `model-high-effort-high`; a low duration caps the effort at medium and a medium one at high. With high stakes the result is never below `model-medium-effort-high`, and the cap that lost is printed as `conflict`. All inputs none gives `model-medium-effort-medium`. In the `rex oms` example above, the demand was `model-xhigh-effort-max`; the low budget capped it at `model-medium-effort-medium`, and the high-stakes floor raised its effort to high.

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

## Development

Build with `cargo build` and test with `cargo test`. The tests live in three folders: `tests/integration/` (the binary end to end), `tests/contract/` (the seams between modules), and `tests/fitness/` (rules of the whole crate, such as what the binary embeds). `tests/fixtures/` holds recorded Claude Code sessions, so no test needs a live Claude Code.

This repo has no CI service. Its checks are git hooks in `.pre-commit-config.yaml`; run `pre-commit install` once to set up both hooks. At commit: the file checks and trivy. At push: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, `cargo check` with the oldest Rust that `Cargo.toml` claims (the hook installs that Rust when it is missing), and `cargo package`, with the tests run again from inside the package. The push checks run on the work tree, not on the commits being pushed, so push from the branch that is checked out.

Two rules hold for every source file: at most 500 lines, and a module comment of one or two lines at the top that says what the file is for (`rex codebase --only code --with-context` shows them). The skills and agents in `.claude/` are source in this repo and are committed; the build embeds them in the binary, and `rex init` installs them from there.

**Licence:** MIT. See [LICENSE](LICENSE).
