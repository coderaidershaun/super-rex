<p align="center">
  <img src="https://raw.githubusercontent.com/coderaidershaun/rex-skills/main/static/logo.png" alt="SuperRex logo" width="200">
</p>

# SuperRex

This is version 0.9.0 of SuperRex.

SuperRex plans and runs a job in your repo with Claude Code's own sub-agents, and keeps a record of how it went. It ships as three parts of one version:

- **The `super-rex` plugin.** It holds the skills that you start, such as `/super-rex:orchestration-builder <request>`, the skill library that agents read, and the 20 grid agents, one for each pair of model and effort. `rex init` installs it in your repo through the `claude` command.
- **The mod, inside the plugin.** It is code that Claude Code runs in every session of the repo. It sees each prompt, each new sub-agent, each model request and each tool call. It keeps the live record of the job, gives each agent its skills, writes the fixed parts of each brief, refuses a new agent over the spend limit, cuts the passing lines of test runs, and is the only way to merge. It adds the tools that the skills call (`rex_job_open`, `rex_skill`, `rex_merge` and others) and one command, `/rex`.
- **A small tool, `rex`.** It sets the repo up (`rex init`), picks the model and effort for each role with a fixed function (`rex oms`), checks skills, and, when you turn storage on, keeps the record of each job in a local store: what was asked, how it was done, what it cost, and whether it hit the mark. When you later say that a job missed, the `rex-feedback` skill reads that record, finds the cause, and stores a lesson that later jobs recall.

Earlier releases copied the skills into the repo, and `rex` hooks recorded the sub-agents. Now the plugin carries the skills, and the mod does the live work. If an earlier release set up your repo, read [the upgrade guide](docs/upgrade-0.8.md) first.

SuperRex supersedes rex-skills (Rex 2.0). Rex 2.0 ran one fixed process on every request. SuperRex lets the agent choose the process, the skills and the models for each request, and records the result so that the next choice is better.

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

The binary is `rex`. rex-skills installs a binary named `rex` too: if it is installed, run `cargo uninstall rex-skills` first, and make sure that the `rex` first on your `PATH` is this one (`rex --version` prints `rex 0.9.0`). The plugin and the binary must have the same version: the mod reads the binary's output and sends it batches of events in the form of one release. `rex doctor` and `/rex doctor` report a difference.

You do not install the plugin by hand. `rex init` installs it in each repo (see below).

| Need | Why |
| --- | --- |
| Rust 1.89 or later | The minimum the crate builds with. |
| Claude Code 2.1.323 or later | `rex init` refuses an older version. The mod is written against the mod API of that version. |
| macOS or Linux | Windows is not supported. |
| On Linux: `bwrap` (package `bubblewrap`) and `socat` | The Claude Code sandbox needs them. Install with `sudo apt-get install bubblewrap socat` or `sudo dnf install bubblewrap socat`. |
| `gh`, signed in | A pull request for a job whose work is not merged. Without it, `rex init` warns, and such a job keeps its branch without a pull request. |
| `pre-commit`, `gitleaks` and `trivy` | Only for the pre-commit config that `rex init` writes. Without `pre-commit` the hooks are not installed; without `gitleaks` every commit fails, without `trivy` every push fails, and `rex init` warns about each. |

## Set a repo up: `rex init`

Run it yourself, in your own shell, in the root of the main checkout (the folder that holds `.git`). Do not ask an agent to run it: the sandbox of a session refuses writes to the Claude Code settings files, and the pre-commit hook store cannot be filled there.

```console
$ cd ~/code/shop
$ rex init --with-storage
rex init: .claude/settings.local.json created
rex init: .rex/config.json created
rex init: plugin super-rex@super-rex installed at project scope from coderaidershaun/super-rex
rex init: AGENTS.md created
rex init: .gitignore created
rex init: .rex/rex.db created
rex init: .pre-commit-config.yaml created
rex init: pre-commit hooks installed
rex init: commit .rex/config.json, AGENTS.md, .gitignore and .pre-commit-config.yaml (git add + git commit); a job works in the main checkout only when it is clean
```

The flags:

| Flag | What it does |
| --- | --- |
| `--with-storage` | Keeps the record of jobs in `.rex/rex.db`. `rex jobs`, `rex job show`, recall, feedback, the cost of record and `/rex master resume` need it. See "Storage". Once on, a later run without the flag keeps it on. |
| `--without-storage` | Keeps no record of jobs, and asks nothing. With neither flag, `rex init` asks at a terminal, `keep a record of jobs in .rex/rex.db? … [Y/n]`, and an empty answer is yes. A run with no terminal, such as one an agent starts, asks nothing and keeps storage off. When storage stays off without this flag, `rex init` says so, names what needs the store, and says that `rex watch` then shows only the snapshots that sessions wrote in the last week. A store that is already on stays on, with either flag. |
| `--depth <levels>` | The most levels of sub-agents a session may start, written as `CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH`. Default 4, which a master run needs. A higher value of yours is kept; a lower one is kept with a warning that a master run needs 4. With the flag, the value you give is written. |
| `--plugin-source <path or owner/repo>` | Where the plugin's marketplace is. Default `coderaidershaun/super-rex`. A folder works too: in this repo, `rex init --plugin-source .` makes each session run the working copy of `plugin/`. |
| `--strict` | Never lets an agent run a command outside the sandbox. See "Commands outside the sandbox". |

What it writes:

| Path | What | In Git |
| --- | --- | --- |
| `.claude/settings.local.json` | The safety settings, the deny rules and the sub-agent depth, merged into your file. | No |
| `.rex/config.json` | What the mod reads when a session starts: `storage` (`on` or `off`), `depth`, `jobs_from_sdk` (whether a headless `claude -p` prompt may start a job, default `true`) and `min_claude_code`. | Yes |
| `.rex/rex.db` | The store. Only with `--with-storage`. | No |
| `.rex/orchestrators/` | The orchestrator library, empty at first. Jobs add the rest (see Files in "Reference"). | No |
| `.gitignore` | A block that starts with `# rex:ignore superrex`. It ignores `.claude/settings.local.json`, `/.claude/worktrees/`, `/.rex/rex.db*`, `/.rex/events.jsonl`, `/.rex/orchestrators/`, `/.rex/plans/`, `/.rex/worktrees/` and `/.rex/live/`. The rest of `.rex/` is tracked: `.rex/config.json`, which a fresh clone needs, and `.rex/skills/`, the skills of the repo itself. | Yes |
| `AGENTS.md` | Two sections at the top, after the marker `<!-- rex:codebase -->`: how to use `rex codebase` and `rex smells`. | Yes |
| `.pre-commit-config.yaml` | Checks for large files, merge markers, TOML, YAML, private keys and white space; gitleaks at commit; trivy at push; `cargo fmt`, `cargo clippy` and (at push) `cargo test` for Rust files. Only if you have no such file. | Yes |

The plugin is installed with `claude plugin marketplace add <source> --scope project` and then `claude plugin install super-rex@super-rex --scope project`. Claude Code keeps the record of a plugin of project scope in `.claude/settings.json`. When that file already turns `super-rex@super-rex` on, `rex init` runs neither command.

Then commit the project files. A job works in the main checkout only when it is clean:

```sh
git add .rex/config.json AGENTS.md .gitignore .pre-commit-config.yaml && git commit -m "chore: set up SuperRex"
```

**Safe to run twice.** A second run changes no file in the repo. `rex init` never replaces what you have:

- Your settings file is merged: missing keys and list entries are added, and your keys and order stay. A value of yours that differs is kept, with a warning.
- A store of rex 0.7 (version 1) in `.rex/rex.db` is migrated to version 2 by the code of `rex migrate`, after a whole copy to `.rex/rex.db.v1-backup`. Every row is carried over. A store with a row that names a row that is not there is left as it is: dropping that row is your choice, and the failure gives the command that lists it.
- `AGENTS.md` and `.gitignore` get their block once, found by its marker. An older release's block in `.gitignore` is replaced (see below). An existing pre-commit config is never replaced.
- Nothing is written through a symbolic link. Skills of your own are left alone. `rex init` does not touch your user-level Claude Code settings, and writes no `CLAUDE.md`.
- It removes only what would do harm or hide what the plugin ships:
  - the hook entries that the previous release wrote for `rex hook`, `rex gate` and `rex test-hook`, which are gone and would fail on every event;
  - the entries named under "The safety settings";
  - each unchanged copy that an older `rex init` (0.4 to 0.7) put in `.rex/skills/`, `.claude/skills/` or `.claude/agents/`. A copy in `.rex/skills/` would hide the plugin's skill, because `rex_skill` reads that folder first, and a copy in `.claude/` would sit beside the plugin's in the session's list;
  - the `.gitignore` block of an older release, which it replaces with this version's.

  Each removal is one line, such as `rex init: removed .claude/skills/rex-grill-me/: it was an unchanged copy from an older rex, and the plugin ships rex-grill-me now`. A copy that you changed is kept, with a warning that names what it does and what to do:

  ```console
  rex init: warning: kept .claude/skills/rex-rules/: it differs from the copy an older rex installed, so the session lists it beside the rex-rules skill that the plugin ships; to use the plugin's, move your changes out and delete that folder
  ```

  An old ignore block that you changed is kept too, and the warning lists each line to change by hand:

  ```console
  rex init: warning: kept the rex block in .gitignore, which differs from the block any rex wrote; to finish the upgrade, change it by hand:
  rex init:   delete the line /.rex/*
  rex init:   add the line /.rex/rex.db*
  ...
  ```

  Neither warning fails the run. A `.rex/skills/`, `.claude/skills/` or `.claude/agents/` that is a symbolic link is never looked into, because it may be shared with other repos.

**The preflight** runs before anything is written. These stop the run with exit 1 and write nothing: the folder holds no `.git` folder (or `.git` is a file: a linked worktree or submodule); `claude` is not on `PATH`; its version cannot be read; it is older than 2.1.323. These only warn: no `gh` on `PATH`; no `rex` on `PATH`, or another `rex` comes first (the mod calls `rex` by `PATH`). On Linux, a missing `bwrap` or `socat` is reported at the end with the install command, and the run exits 1. The settings are still written, and commands run with no sandbox in the repo until the tools are there (after `--strict`, Claude Code refuses to start instead).

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
| `sandbox.failIfUnavailable: false` | Claude Code still starts when the sandbox cannot, and runs commands with no sandbox. Some systems block the sandbox although its tools are installed, for example Ubuntu 24.04 with `kernel.apparmor_restrict_unprivileged_userns = 1`, where `bwrap` fails with `setting up uid map: Permission denied`. `--strict` writes `true`, and Claude Code then refuses to start there. |
| `sandbox.allowUnsandboxedCommands: true` | An agent can run a command again outside the sandbox. `--strict` writes `false`. See "Commands outside the sandbox". |
| `sandbox.enableWeakerNetworkIsolation: true` | `gh` works inside the sandbox. |
| `sandbox.network.allowLocalBinding: true` | A sandboxed command can listen on localhost, so a test server or a nested `claude -p` starts. |
| `sandbox.excludedCommands` | `git worktree`, `git merge`, `git pull`, `git switch`, `rex init` and `rex worktree` run outside the sandbox. Claude Code does this for a git command only when its paths are relative with no `..`, run from inside the repo: `git -C /abs/path …` or an absolute target stays in the sandbox. The sandbox never lets a command write `.claude/skills`, `.claude/agents` or the `.claude` settings files, and no setting narrows that, so these commands could not check out a repo that tracks those folders, or install them. They still ask for permission as before. `git commit` and `git push` stay in the sandbox, because their hooks run code that an agent can change. An entry `git *` (or `git:*`) of yours is taken out, with a notice: `rex init: removed "git *" from sandbox.excludedCommands in .claude/settings.local.json: it ran git and its hooks outside the sandbox, and rex now registers its worktrees, so git works inside it.` `--strict` writes none. |
| `sandbox.filesystem.allowWrite` | The five Cargo cache folders (not `~/.cargo/bin`), `~/.cache/pre-commit/patch*`, and the trivy cache (`~/Library/Caches/trivy` on macOS, `~/.cache/trivy` on Linux). |
| `sandbox.filesystem.denyRead: ["~/.aws"]` | AWS credentials cannot be read. |
| `permissions.additionalDirectories` | The parent folder of the repo, as an absolute path, so that agents can edit sibling repos. Left out, with a warning, when it holds your home folder. Then each job worktree in `<parent>/<repo>.worktrees/`, by its real path. Claude Code's sandbox makes `.git/worktrees/` read-only except for the worktrees of the session, and a linked worktree whose own path is listed here counts as one. So `git commit` works there inside the sandbox, while `.git/hooks` and `.git/config` stay read-only. `rex worktree add` and `rex worktree remove` keep the list; `rex init` adds the worktrees it finds and takes out those that are gone, with a notice for each. That a running session picks up a change of the list without a restart comes from Claude Code's code and was not tested live. |
| `permissions.blockWritesOutsideWorkingDirectories: true` | The file tools cannot write outside the working folders. |
| `permissions.allow` | `Bash(rex worktree add *)` and `Bash(rex worktree remove *)`, so that an agent makes the worktree of a sibling repo, and `/super-rex:rex-pr-review` makes and removes its review folder, with no prompt. The mod makes and removes the worktrees of the job repo itself. `rex worktree` checks its own arguments, and lists only a real linked worktree of this repo in `<repo>.worktrees/`. `--strict` writes neither. An entry `Bash(git *)` (or `Bash(git:*)`) of yours is taken out, and so is `Bash(cd <parent>/*)`, with a notice. |
| `permissions.deny` | `Bash(sudo *)`, `Bash(git push --force*)`, `Bash(cargo publish*)` and `Bash(gh pr merge*--admin*)`. Then the rules that keep merges and the record out of an agent's hands: `Bash(gh pr merge:*)`, `Bash(git push origin main:*)` and `Bash(git push origin HEAD:main:*)`, because a merge goes through the mod's `rex_merge`, which does not run in the shell; and `Bash(rex ingest:*)`, `Bash(rex migrate:*)` and `Bash(rex limit:*)`, because only the mod and you write the store and the spend limits. A deny rule holds even when a hook of the mod is skipped. |
| `env.CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH` | The sub-agent depth, `4` by default (see `--depth`). |

The network stays open, but only through the sandbox's proxy. In bypass mode a host list restricts nothing, so `rex init` writes none. A program that uses the proxy, such as `curl`, `git`, `cargo` or `gh`, reaches any host. A program that opens its own connection, such as a WebSocket client, a database driver or plain `ssh`, cannot connect inside the sandbox.

Agents cannot edit the Claude Code settings files; `rex init` can, because you run it. A sandboxed command can still delete the repo itself: Git recovers the code, but the store in `.rex/` is not in Git.

### Commands outside the sandbox

Because of the proxy limit, an agent can run a command again outside the sandbox. The builder skill tells agents to do this only for a command that failed because its program opens its own network connection, such as a live test of a WebSocket, and to name each such command in their report.

A command outside the sandbox runs with your full access. In bypass mode Claude Code approves it without a prompt; in the default mode it asks you first. So the sandbox stops mistakes, but it does not stop an agent that chooses to leave it. The file tools still cannot write outside the working folders.

**`--strict`** writes `sandbox.allowUnsandboxedCommands: false`, `sandbox.failIfUnavailable: true` and no `sandbox.excludedCommands`, and then no command runs outside the sandbox: Claude Code refuses to start where the sandbox cannot. A live test that opens its own connection cannot run in a session, so the agent gives you the command to run yourself.

A stricter value of yours is never made weaker. If the settings file already holds `sandbox.allowUnsandboxedCommands: false`, a run without `--strict` keeps it, adds no excluded commands, and warns. Set it to `true` by hand to let agents run such commands. A kept `sandbox.failIfUnavailable: true` gets a warning in the same way: set it to `false` by hand to let Claude Code start without its sandbox. The other way round, a kept weaker value of either fails a `--strict` run, like any weaker safety value.

## Storage

The mod keeps a live record of each job in memory: the **book**. What happens to it after the session depends on the storage mode in `.rex/config.json`.

| | Storage off (`--without-storage`, or no terminal) | Storage on (`--with-storage`, or yes to the question) |
| --- | --- | --- |
| Live record | The book, in memory. It survives a reload of the plugin, not the end of the session. | The same, and the mod sends each change to `rex ingest`, which writes it to `.rex/rex.db`. |
| After the session | The record is gone. `rex watch` shows the session's last snapshot in `.rex/live/` for a week. | The store holds it. `rex job show`, `rex jobs` and `rex watch` read it, and the Rust reader prices the tokens from the transcripts as the cost of record. |
| What works | The builder, the spend gate, the plan, skills, briefs, the close report, `/rex watch`, `/rex status`, `/rex tree`, `rex watch` in another terminal, which reads `.rex/live/`, and master runs, whose tree lives in the book for the session. | All of that, and recall, feedback, the orchestrator results, the cost of record, and `/rex master resume` of a tree in a new session. |
| Learning | None: lessons need the store. | Full. |

The mod never writes the SQLite file itself. `rex ingest` takes a batch only with a secret that the mod holds for its session, takes the same batch twice as one, and answers the last sequence number it holds, so a resumed session goes on numbering where it stopped. The mod sends a batch when an agent stops, when a job changes state, and every 10 seconds while there is news. A tool that needs the store answers `<tool>=off` and `reason=storage is off; run rex init --with-storage` when storage is off.

A job whose observer fails is not stopped. The failing part is skipped, and the job is marked `degraded` with the part's name, which `rex_status`, `/rex status` and the close report show.

## How a job runs

Start Claude Code in the main checkout and give it a request with `/super-rex:orchestration-builder <request>`. Only you can start the builder: no agent can load it with the Skill tool, and the mod lets the main session open a job only after your own prompt started the builder. A master starts a builder through `rex_build` (see "Master runs"). The session then:

1. **Checks the mod.** If the plugin's mod is not loaded, the builder stops at once and says so.
2. **Classifies** the request: work type, size, consequence, languages.
3. **Opens the job** with `rex_job_open`, which gives the job ID, the job's places (repo, branch `rex/<job ID>`, base and worktree), the repo's mode (`remote=origin` when `origin` holds the base, else `remote=none`), the count of uncommitted paths in the main checkout, its spend limit and whether storage is on. The mod reads the places from git and never asks you about them: the base is the branch `origin/HEAD` names, or in a repo with no `origin` the branch you have checked out (with HEAD detached, `main`, else `master`). A repo with no commit gets its first commit from the mod (`chore: first commit, the base of rex jobs`, every file `.gitignore` does not exclude, with your commit hooks), and the answer says `base_commit=made`. When a hook or git refuses that commit, the mod puts back what you had staged, opens no job, and its refusal gives the line that names the failure and the two commands to run yourself. The job starts from the last commit, so the builder tells you in one line when you have uncommitted changes it does not hold. The mod then makes the job's worktree `<repo>.worktrees/<job ID>` on the job branch, from `origin/<base>` after a fetch, or from the local base in mode `none`; a fetch that fails stops nothing while the repo knows `origin/<base>`, and the answer (`base_stale=yes`) and a status line say the base may be stale. A session root that is no git repo is refused: `rex_job_open: <root> is not a git repo, got <git's line>; run git init there, then start again`. A session has at most one open job for each builder; a second open gives the same job with `existing=yes` and makes nothing.
4. **Agrees on the merge.** If the request does not say whether the work may be merged to main, the builder asks you one question (the only one about delivery: none about branches, remotes or git set-up), `Merge this job to main when its checks pass?`, with the options `Yes, merge` and `No, keep the branch`. The mod records your answer from that dialog. It also takes `merge: yes` or `merge: no` from your own prompt. No tool argument can give a merge answer.
5. **Recalls** lessons from similar jobs and the orchestrators they used (`rex_recall`). With storage off, there is nothing to recall.
6. **Starts a decision agent.** A sub-agent makes the decision on the grid agent that `rex oms` picks for it, so the session does not carry the rules of the three stages while it runs the job. The stages are files inside the builder's skill, `orchestration-builder/stages/`:
   - **Stage 1, `1-skills/stage.md`:** chooses the skills for each task with `rex_assign`. The assignment table is data, `plugin/assign/table.json`: each row is a kind of work with its criteria, its skills, a priority and where its skills go. Every task that writes code gets `rex-code-philosophy`, `rex-code-commenting`, `rex-code-ergonomics` and `rex-code-error-writing`, whatever its language, and tests, new files, new dependencies, retries, designs, refactors and the rest each add their own. Code decides what a rule can decide, such as whether a task writes code or which language it uses; the decision agent answers the other criteria, yes or no, each with a reason. `rex_assign` then places the skills, at most 6 read first, by priority, and the rest as open-when skills with their case. A skill that a rule requires stays whatever the answers say. Each answer and each placed skill is recorded with its source (`rule`, `decision-agent` or `override`), its reason, and the hashes of the table and the overlay. A repo can change the table with an overlay, `.rex/assign.json`, that replaces, adds or removes rows. Stage 1 asks for a new skill only when a three-part test passes, and the session then has a sub-agent write it into `.rex/skills/`.
   - **Stage 2, `2-work-breakdown/stage.md`:** splits the request into tasks, gives each task its skills, and finds what can run in parallel. One task builds one feature whole: code is split into parallel tasks only where the files and outcomes are independent. The skills of a task are two lists: `Skills`, at most 6, which the agent gets first, and `Open when`, each with its case, which the agent opens only when its work reaches that case.
   - **Stage 3, `3-orchestration/stage.md`:** reuses an orchestrator from `.rex/orchestrators/` or writes a new one, gives each task a role, and rates the inputs that pick each role's model. It caps the sub-agents of the job by its size: 5 for a small job, 6 for a medium one, 10 for a large one, shown as `AGENTS: <count> of max <max>`.

   If only you can answer a question, the decision agent asks it through the session, at most twice.
7. **Challenges the decision.** A paired challenger, a fresh sub-agent that made no part of it, checks the skill list, the work breakdown, the orchestrator and the role table before any work starts. It looks most for important skills that were left out or not given to a task. The decision agent checks each finding against the real files and corrects what holds.
8. **Records the plan** with `rex_job_plan`. For each role the mod runs `rex oms` and names the grid agent, such as `super-rex:model-high-effort-high`, and it stores the skill manifest of each task with the hash of each file.
9. **Runs** the orchestrator in the job's worktree, on the job branch, never on main. Leads and workers are Claude Code sub-agents. A lead that starts parallel writers gets each unit worktree from `rex_worktree`; no agent runs a worktree command for the job repo. The author of each brief writes only the task: a first line `Task: <task ID>` and five sections, `Outcome:`, `Scope:`, `Context:`, `Acceptance:` and `Limits:`. The mod does the rest (see "The mod").
10. **Delivers** each changed repo with the steps of `rex-deliver`: one squash commit on the job branch, then `rex_merge` if a merge was agreed (a push to `origin`, or in a repo with no `origin` a fast-forward of your local base branch, so the files appear in your main checkout), else a pull request or a kept branch. The builder deletes no branch and removes no worktree of the job repo.
11. **Closes** the job with `rex_job_close`. The mod then removes the job's worktrees and each of its branches whose work is on the base, here and on `origin`, and keeps the rest with a reason (see "Delivery"). The builder tells you the result in a few lines: what was done, and for each repo whether it was merged, left as an open pull request, kept locally, or blocked, with what you must do, and each worktree or branch the mod kept.

The session runs at your model and effort; every sub-agent runs on a grid agent that `rex oms` picks for its role. A small job can be one task and one worker.

### Plan first, then build

A request for a plan, such as "plan how we add CSV export", becomes a **plan job**. It changes no file in the repo, so it needs no merge question and no branch. Its orchestrator is built for planning: optional researchers, one plan writer, a `plan-reviewer`, then a revision. The plan goes to `.rex/plans/<job-id>/plan.md`, with a goal, decisions, scope, a task table, risks, acceptance evidence and open questions.

Before the plan is final, `rex-codex-adversarial-review` challenges it with a second model. The OpenAI Codex CLI (`codex`, read-only) questions the approach and the assumptions first, and the agent running the review checks each finding against the real files. Codex then gets one round to concede or hold its position. If Codex is not installed or not logged in, a grid agent challenges the plan instead: `model-high-effort-high` for a low or medium consequence, `model-xhigh-effort-high` for a high one. Findings that hold change the plan, and contested ones go into its open questions.

You read the plan, then start the builder again with `/super-rex:orchestration-builder Execute plan <path of plan.md>`. That second job asks about the merge as usual and works from the plan's task table.

## The mod

The mod is the code below `plugin/hooks/`. Claude Code runs it in each session of a repo where the plugin is on. A part that fails never blocks a prompt, a tool call or a new agent: the job gets a `degraded` mark instead. A part that guards authority (who may merge, who may start a master, how deep agents go) refuses when it cannot decide.

**Your prompts.** The mod reads each prompt that you type, or send through Remote Control, and, while `jobs_from_sdk` is `true`, each prompt of a headless `claude -p` run. A prompt from any other source, such as a task notification or another plugin, passes untouched. `/rex prompts <off|observe|improve>` sets what it does; the default is `improve`:

- `observe` records each prompt unchanged in the job's record.
- `improve` also adds, to a prompt that starts a job, one restatement of the request as context: goal, acceptance, constraints, and which of merge answer, budget and consequence are missing. A Sonnet completion at low effort writes it. Your text is never changed: the transcript and the record keep what you typed, and the restatement says it is one reading of your words. If the completion fails, the prompt goes on without it, and a status line names the reason.
- `off` records nothing and adds nothing.

In every mode the mod reads a merge answer (`merge: yes`, `merge to main: no`) and a spend limit (`Set the spend limit of this job to 12 dollars`) from your words. Your latest word holds.

**Briefs.** For each new sub-agent of a job, the mod checks the author's brief once. A brief with a missing section, no evidence in `Acceptance:`, a phrase such as `think step by step`, or `never` outside `Limits:` is sent back once: `rex brief check (1 of 1): Acceptance: is missing. Fix the brief and start the agent again.` The second try goes through, marked. Then the mod builds the brief that the agent gets, in this order: the `Rex-Job` and `Rex-Role` lines; a `<skills>` block with the full text of each read-first skill and the hash of each file, and a line for each open-when skill and its case; the author's text, unchanged; the text that the agent's model and effort need, from `rex-agent-prompting`; the sandbox text; and a `Return:` line that names the report file `.rex/plans/<job>/<role>.md` (`<role>-2.md` for a second agent of that role, and so on) and asks for a final message of at most 2,000 characters. A resumed agent, a teammate and an agent outside any job get no assembly.

**Skills.** An agent reads a skill file by its name with `rex_skill`, never by a path, because the plugin's folder changes with each version. `rex_skill` looks in this order: the repo's `.rex/skills/` in the main checkout; then the job worktree's `.rex/skills/`, for a skill that only the job wrote; then the plugin's skills and library; then `~/.claude/skills/`. Each answer gives the source and the `sha256:` hash of the file, and the run's record keeps each skill it was given or opened.

**The spend gate.** Each job has a spend limit in US dollars at list prices. Once the job's sub-agents and main session have spent it, the mod refuses each new sub-agent, at the Agent call and again when the agent would start: `rex gate: job <id> has spent $X of its $Y limit; a person raises it with /rex limit <id> <usd>`. Agents already running keep spending, so a job can end a little over its limit. Before it refuses, the gate reads a limit that you set at a shell with `rex limit <job> --set <dollars>`. The default limit is in `rex limit`.

**Other guards.** A sub-agent at depth 4 starts only below a builder that a master started. A fork inside a job is refused, because a fork cannot change its model: the agent is told to start a grid agent instead. Inside a job, an Agent call starts its agent with `subagent_type`, `description` and `prompt` only: a call that also gives `name` or `team_name` (which would start a teammate that the mod does not record, gate or brief), `model`, `effort`, `mode`, `isolation` or `cwd` is refused before anything starts, with the argument it saw and what to do instead. A teammate or a workflow's agent that starts inside a job some other way is refused at its spawn. Agents outside any job are not touched. A shell command that would merge around `rex_merge` is refused.

**Test output.** When an agent runs a plain test command, such as `cargo test` or `pytest`, through the Bash tool, the mod cuts the lines that a runner prints for a passing test from the result before the agent reads it, and keeps every summary line. A failed run reaches the agent whole. A note at the end says how many lines were left out and to run the command again with `| cat` at its end to see them all.

**The band and the pane.** Every view names the levels of a run the same way: `master` (the master of a tree), `builder` (the agent that runs one job: the main session, or a builder a master started), `lead` (an agent the builder started), `worker` (an agent a lead started) and `helper` (anything deeper). A job with no tree has three levels; a tree has four.

While a tree runs, the rows above the prompt show the tree, then each open job and what its builder is doing, then the oldest question that waits for you:

```
rex master t0geh4lh3i running · 0/1 delivered · 2 agents open, deepest worker · $3.65/$150.00
  builder 0geh5fafkb planned · leads engineer-lead · 1 worker open · $3.25/$28.00
? 1 question waits on you · Merge job 0geh5fafkb (rex/0geh5fafkb) to main of /home/ana/code/shop? · /rex answer q1-553d90ce69d1 <1-2>
```

The question line is drawn in the warning color. A job with no tree has one line, `rex builder 0gaq820001 planned · 0/2 agents done · deepest worker · $0.01/$10.00 · 1 permission prompt`. When the band has few rows, the job lines of a tree are left out first, and the question line stays. Nothing shows while nothing runs.

`/rex watch` opens the pane. Its header gives the storage mode, the prompt mode, the jobs, the deepest level reached, the waiting questions and the versions of the mod, the binary and Claude Code. Its tabs are **tree**, **decisions** and **questions**, each with its letter key (`t`, `d`, `q`). The keys work once the pane has the keyboard: press ctrl+x tab, or click it. Until then the pane says so.

The **tree** tab walks the run one level at a time:

```
session › master t0geh4lh3i › builder 0geh5fafkb
level 2 of 5 · master › [builder] › lead › worker › helper
? builder 0geh5fafkb · planned · 1 question waits · 2 agents open, 2 done · $3.25 of $28.00 · 6m
u: up to master t0geh4lh3i
one level down, lead: press 1-2
1: ✓ lead decision-maker · done (answer) · $0.70 · 1m
2: ● lead engineer-lead · open · 1 agent open, 1 done · $0.60 · 3m
```

On top is the breadcrumb, the path from the session to the node in view, each part a button back to it; in a narrow pane the parts above the node in view shrink to their level. Below it the rail says how deep the node is and names the levels of this run with the current one in brackets. A job with no tree has three levels (`[builder] › lead › worker`), a tree four, and `helper` shows where an agent ran below a worker. A row names an agent by its role (`engineer T1` when its task differs), by its task before the job has a plan, else by its grid agent. A role that is the word of its level is not said again: `worker T1`, `lead`. Each child of the node is a row: its digit goes one level down, `u` goes one level up. Past nine rows, `n` shows the next nine. The rows come first, so a short inline pane still shows the way down. Then come the questions that wait on the node or below it, each with the command that answers it, then the facts of the node: of a tree its state, charter, spend against its cap, pinned policy and notes; of a job its status, phase, orchestrator, merge answer, spend against its limit, worktree, deliveries, done checks, degraded marks and report file; of an agent its id, role, task, the grid agent planned against the model and effort that ran, state, time, tokens, cost, brief check, skills given and opened, and report file. The pane keeps the node in view across redraws; a node that is gone falls back to its nearest parent.

The **decisions** tab shows, for each skill of each task and each role rating, the source and the reason that `rex_assign` recorded. The **questions** tab shows each question that waits for you, the oldest first. A digit answers the oldest one with that option; each other one has a button for each option, and the command `/rex answer <id> <option number>` answers any of them.

The mod also writes the same view to `.rex/live/<session id>.json` in the main checkout, in every storage mode, so `rex watch` in another terminal shows the run with no store: after each change (it looks every 2 seconds) and at least every 10 seconds while a job or a tree is open.

### The `/rex` command

`/rex <sub-command>` runs in the session at once and costs no model call. A sub-command marked "yours only" runs only from your own typing (or Remote Control): from any other origin it answers `rex <sub-command>: only your own typing may run this, got origin <kind>`.

| Sub-command | Yours only | What it does |
| --- | --- | --- |
| `/rex watch` | no | Opens the pane: the run as a tree you walk one level at a time. |
| `/rex status [job]` | no | The lines of `rex_status` for your job, or the job you name. With no job named, while a master tree runs and the main session builds no job, the tree in plain lines: the tree's line of the band, a line for each open job with the folder it builds in, and each question that waits for you with its recommendation and the command that answers it, or `nothing waits on you`. |
| `/rex tree` | no | The outline of the session, drawn with tree guides: each master tree, its jobs and their agents, one line each with its level and name, state, waiting questions, agents, spend, time and what it is about, cut to the width of the terminal. |
| `/rex jobs` | no | One line for each job: from the store with storage on, else from the book. |
| `/rex doctor` | no | The output of `rex doctor`, then `mod=<plugin version>`. |
| `/rex prompts <off\|observe\|improve>` | yes | Sets the prompt mode. |
| `/rex limit <job> [usd]` | yes | Shows a job's spend and limit, or sets the limit (source `set`). An amount must be a number from 0 to 9,000,000. |
| `/rex master <goal>`, `/rex master @<file>`, `/rex master resume` | yes | Starts a master tree from your goal or a charter file, or resumes one (resume needs storage). It answers `tree=`, `master=`, `merge=`, how to watch the tree and how to answer its questions. See "Master runs". |
| `/rex policy [show\|confirm\|set <key> <value>]` | yes | Shows, confirms or sets what a master may decide as you in this repo. A confirm wakes the master of each tree that the change paused. |
| `/rex answer [question id] <option>` | yes | Answers a parked question as you. The option is its number, from 1, or its text in any letter case. With one question waiting, the id can be left out. A refusal lists the options with their numbers. |

### The tools

The skills call these tools. Their full names are `mcp__super-rex__rex_<name>`. The mod reads the caller from Claude Code, never from an argument, and a caller that a tool does not name is refused. A builder's job is always its own open job: a `job` argument must name it. Each tool answers `key=value` lines. Only `rex_skill` and `rex_status` load at the start of a session; the others wait until an agent asks for them.

| Tool | Who may call it | What it does |
| --- | --- | --- |
| `rex_skill` | any agent | Gives one file of a skill by name (`SKILL.md` by default), with its source and hash. With no name, it gives the parts of the caller's brief that are missing. |
| `rex_status` | any agent | Where a job stands: status, phase, origin, open and finished agents, spend and limit, degraded marks, then `question.<id>=parked` or `answered` and `answer.<id>=<option>` for its questions, the `decision.<task or role>.<item>=<source>: <placement>; <reason>` lines that `rex_assign` recorded, its tree, and for a job of a tree `output=` (its report file), `merge_commit=` once merged, and the `done.<check>=` lines of a delivered one. A master gets the lines of its whole tree: `tree_state=`, its spend and cap, `job.<id>.status=` and `job.<id>.builder=` for each job, its notes, and a `policy.<key>=` line for each key of the policy it works under. The main session with no job of its own gets the same lines for each live tree of the session, with no policy lines. |
| `rex_job_open` | the builder, and the main session only after your prompt started a job | Opens the job, or gives the open one with `existing=yes`, and makes the job's worktree on its job branch from the base (a second open makes nothing). Answers the places, `remote=origin` or `remote=none`, and `uncommitted=<n>`; in a repo with no commit it makes the first commit and answers `base_commit=made`; `base_stale=yes` when `origin` could not be fetched and the job starts from the `origin/<base>` this repo last fetched. A folder that is no git repo is refused with `git init` to run. |
| `rex_recall` | the builder | The output of `rex recall` for the job; `recall=off` with storage off. |
| `rex_job_plan` | the builder | Records the plan and gives the grid agent of each role. It works once; a second call needs `amend: true`. |
| `rex_job_close` | the builder | Records the outcome and gives the close report: spend, runs, skills a role was given and never opened, extra skills, long final messages, briefs that were not assembled, degraded marks, and with storage on the cost of record. Then it cleans up (see "Delivery") and answers a `removed.` or `kept.` line for each worktree and branch of the job. Safe to repeat. A `merged` delivery is refused unless `rex_merge` merged that job. |
| `rex_worktree` | the builder, and every agent of its job | Makes the worktree of one unit of parallel work, `unit` (one to three lower-case words joined by hyphens, not `work` or `review`): `<repo>.worktrees/<job ID>--<unit>` on the new branch `<job branch>--<unit>`, from the tip of the job branch, listed for the sandbox. Answers `worktree`, `branch`, `start` (the commit) and `existing=no`; a second call for the same unit answers the same lines with `existing=yes`. The job is the caller's own, never an argument. |
| `rex_merge` | the builder | Merges the job: a push to `origin/<base>`, or in mode `none` a fast-forward of the local base. See "Delivery". |
| `rex_assign` | the builder, and the decision agent of the job | Applies the rules of the assignment table and gives `questions=<n>` with one `question.<n>=` line for each criterion that needs judgement. Called again with the answers, each `yes` or `no` with a reason, it gives the skills of each task (`task.<id>.skills=`, `task.<id>.open_when.<skill>=`), the source of each, and the grid agent of each role. An assignment that was answered before gives the stored answer. |
| `rex_build` | the master of a running tree | Opens one job of the tree, classified by its required `type`, `size`, `consequence` and `langs`, and starts its builder in the job's own worktree, with the fixed builder brief and the request. It is refused over the policy: the spend of the tree, the rate limit, the job counts, the consequence. It answers `job`, `builder`, `status=created` and `worktree`, `base_commit=made` when it made the first commit of a repo with none, and `base_stale=yes` as `rex_job_open` does. |
| `rex_continue` | the master | Sends a message to the builder of one job of the tree and wakes it, as the answer to its question or to go on. |
| `rex_note` | the master | Records one decision of the master in the tree: what it decided, why, and the option it did not take. |
| `rex_ask_owner` | the master | Parks a question for you, with 2 to 4 options, a recommendation and a category (`merge`, `scope`, `spend` or `other`), and returns at once with `question=<id>` and `state=parked`. |
| `rex_report` | the master | Ends the tree with the master's final report, as Markdown. It is refused while a job of the tree is open or a question of it waits. It writes `.rex/plans/<tree id>/final-report.md`, closes the tree as `done`, and answers `tree`, `state=done` and `report=<path>`. |

## Master runs

A master acts as you over a longer goal: it splits the goal into jobs, starts a builder for each, follows them, and asks you only what the policy does not settle. With storage off, the tree lives in the book for the session; with storage on, it is also stored, and `/rex master resume` can put it back in a new session.

**Start one** with `/rex master <goal>`, or `/rex master @<file>` to give a charter file from the repo. Only your own typing starts a master. A headless `claude -p` run is refused unless the policy sets `allow_headless: true`:

```
rex master: only your own typing may run this, got origin sdk
```

The mod stores the goal unchanged as the **charter**, and starts the master itself. Its first prompt is the charter as you wrote it, then a reading of the charter by another model, marked as such (goal, acceptance and constraints), then three blocks of data from the mod: `<policy>`, one `key=value` line for each key that bounds the master (`merge`, `merge_max_consequence`, `max_parallel_builders`, `max_jobs`, `consequence_ceiling`, `tier_cap`, `tree_usd`, `per_job_usd` and `raise_limits`), `<repo>`, the lines `repo=<path>`, `base=<branch>` and `remote=origin` or `remote=none` of the session root, and `<codebase>`, the output of `rex codebase --only code --with-context`, cut at 150 lines (left out when the command fails). No model writes the master's prompt, and no agent can start a master: the master type is offered to no model, and the mod denies every spawn of it but its own, with `rex master: only /rex master starts a master, got a spawn by <who>; run /rex master <goal>`. Before it starts, the mod checks these, and the first that fails is the whole answer:

| Check | Refusal |
| --- | --- |
| `rex --version` equals the plugin's version. | `rex master: binary <v> does not match plugin <v>; run rex doctor` |
| The depth is at least 4. | `rex master: depth is <n>; run rex init (it writes 4)` |
| A headless run has `CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS=0`. | `rex master: headless runs need CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS=0, or background agents stop after 10 minutes` |
| The policy of the repo is confirmed. The first interactive run asks you one question, `Merge each job of a master to its base branch when its checks pass?`, with the options `Yes, merge`, `No, keep the branches` and `Ask me for each job` (`merge` `allowed_if_checks_pass`, `never` or `ask`), and stores and confirms the policy, with `max_parallel_builders` at `auto`. | Headless: `rex master: no confirmed policy for this repo; run /rex policy confirm in an interactive session` |
| A headless run uses `bypassPermissions`, or every allow rule of the policy's `required_allow` is in your settings, because in `-p` each permission ask is a deny. This is checked when the master starts. | `rex master: headless mode <mode> asks no one, so each ask is a deny; run with bypassPermissions or list allow rules in policy required_allow, got <missing>` |

When it starts, `/rex master` answers:

```
tree=<tree id>
master=<agent id>
merge=<policy value>
watch=/rex watch here, or rex watch in another terminal
answer=/rex answer <question id> <option number>
```

**How it works.** The master has no Agent, Bash, Write or Edit tool. Its tools are `Read`, `Glob` and `Grep` (to look at the code and the jobs' reports), `ToolSearch`, `rex_build`, `rex_status`, `rex_continue`, `rex_note`, `rex_ask_owner`, `rex_report` and `rex_skill`, and it follows the procedure in `orchestration-builder/master/procedure.md`. It runs on the policy's `master_model` (default `opus`) at high effort. `rex_build` checks the policy, opens a job, makes the job's worktree with `rex worktree add`, and starts a builder on the grid agent that `rex oms` gives for the job's ratings, capped at the policy's `tier_cap`, with the fixed brief of `orchestration-builder/master/builder.md`. The charter's constraints go into every brief of the tree. Depth under a master: master and builder at 1, decision agent, challenger and leads at 2, workers at 3, a worker's own fan-out at 4. A job of `depends_on` must be delivered before a build starts (`rex_build: depends_on job <id> is not delivered, got <status>; build this job after that one delivers`), and `rex_merge` holds a job until each job of its `merge_after` is merged. The gate checks `tree-ledger` (the spend of the tree against `tree_usd`), `rate-limit-pause` and `max-jobs` stand before each `rex_build`, and `policy-merge` before each `rex_merge` of a tree job. Each of them refuses when it fails. At the tree's cap, `rex_build` tells the master to park a spend question with `rex_ask_owner` and end its turn. At the rate limit, it tells the master to end its turn, and the mod wakes it once, `retry.then_minutes` later, with `rex: plan usage was over the pause point <n> minutes ago. Call rex_build again for the steps that wait.`

**The policy** lives outside the working folders, in the plugin's own store in your Claude Code folder, keyed by the repo, so no agent can edit it. The mod pins its hash when the master starts. `/rex policy show` prints each key as a `key=value` line, and `/rex policy set <key> <value>` changes one (a value that is JSON, such as a number, a list or `true`, is read as JSON). A changed policy pauses a running tree: `rex_build` and `rex_merge` are refused with `rex: policy changed since this master started, got hash <new>; confirm it with /rex policy confirm`. `/rex policy confirm` in the same session pins the new hash and runs the tree again, with its master and builders, and wakes the master with `rex: the person confirmed the changed policy. The tree now:` and the lines of `rex_status`. The keys, with their defaults:

| Key | What it does |
| --- | --- |
| `merge`, `merge_max_consequence` | `never`, `ask` (default) or `allowed_if_checks_pass`, and `medium`. With `allowed_if_checks_pass`, a job at or under `merge_max_consequence` gets the merge answer `yes` from the policy; with `never`, `no`; with `ask`, none, so `rex_merge` parks a merge question for you. |
| `merge_checks` | Commands, each a list of words, that must exit 0 in the job's worktree before a merge that the policy allowed. With none (the default), the builder's own checks are the checks of the merge. |
| `always_ask_paths` | Globs, default `contracts/**`, `infra/**` and `**/*.env*`. A diff that touches one turns a policy answer into a question for you. |
| `per_job_usd`, `raise_limits`, `tree_usd` | The limit of each job (default none: the default limit of its size and budget), whether a limit may be raised, and the cap of the whole tree (default $150). With `raise_limits: allowed`, a job at its limit gets it doubled once, up to the tree's cap; with `ask` (default), the mod parks a question with the options `Raise` and `Keep the limit`; with `never`, the job stops at its limit. |
| `max_jobs`, `max_parallel_builders`, `consequence_ceiling` | How many jobs (default 8), how many builders at once (default `auto`: 1 for a large job, 2 for a medium one, 3 for a small one) and the highest consequence a job may have (default `high`). |
| `repos` | The repos `rex_build` may name besides the session's own (default `.`). |
| `pause_at_rate_limit_pct` | New builders wait when plan usage reaches this share of its limit (default 85). |
| `allow_headless`, `master_model`, `tier_cap`, `on_dead_builder`, `watchdog_minutes`, `stuck_ticks`, `retry`, `required_allow` | Headless masters (default `false`), the master's model (default `opus`), the highest tier of a builder (default `high`), what resume does with a builder that is gone (`ask`, the default, or `continue`), the watchdog period (5 minutes) and the ticks of flat spend that make a builder at work stuck (18, so 90 minutes), the retry schedule (`first_minutes` 2, `first_tries` 3, `then_minutes` 10, `max_tries` 12), and the allow rules a headless master needs. |

**Questions.** `rex_ask_owner` parks a question and returns at once, so the master goes on with other jobs. The mod parks questions of its own too: a merge question, a spend question, and a question when a job is blocked. Each shows in the band and as a row of the pane with an Answer button for each option. Each question, from `rex_ask_owner`, a merge, the spend limit, a blocked job or a resume, also shows at once as a status line, `rex: question <id> waits: <question> Answer with /rex answer <id> <1-n>`, and, in an interactive session, as a dialog. Answer it in the pane, in the dialog or with `/rex answer <id> <option number>`. The master reads the answer through `rex_status` (`answer.<id>=<option>`), and the mod wakes it with `rex: question <id> is answered: <option>. The tree now:` and the lines of `rex_status`. The answer to a merge question also wakes the job's builder with `rex: the person answered question <id>: <option>. Call rex_merge again.` A wrong answer changes nothing, for example `rex answer: <option> is not an option of <id>, got <option>; one of 1 (<option>), 2 (<option>)`. A message to the master from anyone but its own builders and the mod is refused: `rex: only this master's builders and the mod may message the master`. In `-p`, a job whose question is not answered when the session ends is `blocked`.

**Unattended.** Each `watchdog_minutes`, a watchdog looks at each open job of the tree. A job with a run open, the builder's or an agent's below it, with no question waiting for the person, and whose spend stayed flat for `stuck_ticks` ticks (by default 90 minutes, room for a long test run) is stuck: its builder is woken with `Continue job <id> from the first step not done.` An agent that is alive spends, so a job whose agents spend is never woken, and an agent whose end never reached the mod cannot hold the tree for ever. A job that is idle, with no agent running and no question waiting for the person, is surer to be stuck: its builder is woken after 2 ticks with no spend (by default 10 minutes). When the builder of a stuck job has stopped, a stopped master hears of the first wake. Each next step waits the same period again: after 2 such wakes with no spend, the job is `blocked`, and the mod parks the question `Job <id> is blocked: … What should the master do?` with the options `Build it again` and `Leave it blocked`. With the default policy, an idle job's builder is woken 10 to 15 minutes after its last spend, again 10 minutes later, and the question comes 10 minutes after that; for a job with a run open, the steps come 90 minutes apart. A builder that ended on an API error is woken again on the `retry` schedule (by default 2 minutes three times, then 10 minutes, at most 12 tries), and then the job is `blocked` with the same question. When a builder ends its turn with no agent of its own running, or closes its job, the mod wakes the master with the end of the builder's last message; a builder that waits for its own agents wakes nobody. When a job of the tree closes `delivered`, the mod checks it in the job's repo and records each check as `done.<check>=pass|fail|not-set` in `rex_status`: `fetched`, `merged` (the merge commit is on `origin/<base>`, or in mode `none` on the local base), `base_current` (the local base equals `origin/<base>`; `fetched` and it are `not-set` in mode `none`), `branches_gone`, `worktrees_gone`, `output_exists` (the job's report file), and `checks_on_base` (`merge_checks` on the base; `not-set` with none). A `fail` keeps the job's step open, and the master decides.

**Resume.** `/rex master resume` in a new session reads the newest open tree from the store (`rex tree show --json`), puts back the tree, its jobs and the questions that wait, and starts a new master with the charter and a digest of the restart. A job whose builder is gone gets a new builder in the same worktree, made again on the job's branch at its tip when it is gone (`on_dead_builder: continue`), or the question `The builder of job <id> ended with the last session. Start a new builder for it?` (`ask`). With storage off it refuses: `rex master resume: needs storage, got storage off; run rex init --with-storage, or start a new tree with /rex master <goal>`. It refuses while the tree's master still runs in this session (`rex master resume: tree <id> is live in this session, got master <agent id>; use /rex policy confirm`), and when the store holds no open tree (`rex master resume: no open tree in the store of this repo, got none; start one with /rex master <goal>`).

**The main session.** The main session does not answer the turns of the mod's own agents. When the master or a builder of a live tree ends a turn, Claude Code's notice of it reaches you as one line, such as `rex: the builder of job <id> ended its turn (tree <id> runs; /rex watch shows it)`, and starts no turn of the main session. When the tree is closed, the master's last notice reaches the main session as `rex: master tree <id> is closed (<state>). Its final report is <path>. Read it and tell the person the result in a few lines.` Asked for its status, the main session gets the lines of each live tree from `rex_status`.

**The final report.** The master ends the tree with `rex_report`, once no job of the tree is open and no question of it waits. The mod writes `.rex/plans/<tree id>/final-report.md` and closes the tree as `done`. The report holds one line for each job (its status, request, merge commit, report file, done check and outcome) and the master's report, which the procedure asks to give the result, the evidence, and each item `not run`, `not proven` or `not built`, with the reason. A master turn that ends never closes the tree by itself. When the master stops while its tree is live, with no job open, no question waiting and no wake of the mod on the way, the mod wakes it with `rex: your turn ended with no open job and no waiting question. IF the goal of the charter is met, or cannot be met, THEN call rex_report with the final report. ELSE start the next step with rex_build, or park a question with rex_ask_owner.` After 2 such wakes in a row (a successful `rex_build` or `rex_ask_owner` starts the count again), the mod closes the tree as `done` itself, with the master's last message as the report, under a first line that says why.

## The commands

`rex commands` lists them all. Paths in the outputs below are shortened to `/home/ana/code/shop`.

**`rex init [--with-storage | --without-storage] [--depth <levels>] [--plugin-source <path or owner/repo>] [--strict]`**: set SuperRex up in a repo. See above.

**`rex migrate`**: move a store of version 1, which the previous release made, to version 2, in one transaction. Every other command refuses a version 1 store with `error: store is version 1; run rex migrate` and exit 2. `rex init --with-storage` runs the same migration itself, after a backup, so you rarely need this command. Each job is kept: a job asked by a person gets the origin `composer`, any other `sdk`, and a spend limit that was set moves into the job.

```console
$ rex migrate
store=v2 migrated=yes jobs=2
$ rex migrate
store=v2 migrated=no
```

A store of any other version is refused with exit 2, and nothing is written. Agents may not run it.

**`rex ingest`**: the mod's write path, not for people. It reads one batch of events as JSON on stdin and prints `last_sequence=<n>`. A refused batch changes nothing: exit 2 for a batch that does not fit (its form, a gap in the sequence, a conflict), 3 for a wrong secret, 4 when storage is off or there is no store. Agents may not run it.

**`rex doctor`**: check that the binary, the store, the config, the plugin and Claude Code fit. It prints `doctor=ok` or `doctor=problems`, then `binary=`, `store=`, `storage=`, `claude=` and `plugin=` (the versions that the newest session reported), and one `problem=` line for each thing to fix, such as a plugin and a binary of different versions, a Claude Code older than `min_claude_code`, or an open job of a session that is gone. It runs no `claude` command, and exits 0 either way.

```console
$ rex doctor
doctor=ok
binary=0.9.0
store=v2
storage=on
claude=2.1.323
plugin=0.9.0
```

**`rex job show [<job>] [--json]`**: print one job's record: origin, builder, tree, classification, preferences, merge answer and its source, places, limit, degraded marks, plan, roles with their model selection and skills, deliveries, sub-agent runs with the skills each opened and their tokens and cost, the main session's tokens and cost, the request, the breakdown and the recalled experience. With no job, it shows the open job of the main session of `$CLAUDE_CODE_SESSION_ID`. `status` is the shown status (`running` for a planned job with a sub-agent open), `stored_status` the status as stored. `--json` adds the transitions, each run's brief and final message, the questions, the decisions, the skill manifest and the feedback.

```console
$ rex job show 0gaq820001
job=0gaq820001
status=created
stored_status=created
origin=composer
builder=main
tree=none
type=fix
size=small
consequence=low
languages=rust
...
merge=yes
merge_source=person-prompt
repo=/home/ana/code/shop
branch=rex/0gaq820001
base=main
limit_usd=10.000000
limit_source=default
degraded=none
...
```

**`rex limit <job> [--set <dollars>] [--json]`**: print a job's spend and spend limit, or set a new limit. It is for you at a shell; agents may not run it, and in a session you use `/rex limit`. `--set` takes any positive amount, such as `12` or `7.50`, and replaces the old limit (it can also lower it); a bad amount exits 2. `source` is `default`, `set` or `policy`. The default limit comes from the job's budget and size, in US dollars at list prices: the agent cap of the job's size (5, 6 or 10) plus 2 for the main session, times the dollars for each agent, $1.50 for budget `low`, $4 for `medium` and $8 for `high`. A job with no stated budget uses the `medium` row.

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

Spend leaves out requests on a model with no price, so it is a lower bound. The dollars for each agent do not change with the model tier, so a job on the `xhigh` tier can pass its limit before it reaches its agent cap.

**`rex status [<job>]`**: print where a job stands, from the store: one header line, then the runs under `open`, `finished` and `unconfirmed`. With no job, every `created` or `planned` job; `no open job` if there is none.

```console
$ rex status 0g51gpfx9m
job 0g51gpfx9m  created  0 open  1 finished  0 unconfirmed  incomplete
finished
  a0c8cde396de066a3  reader  model-medium-effort-low  1m12s
```

Each run line gives the agent ID, the role, the agent type, the duration, `bg` for a background run, `<parent` when a sub-agent started it, and the tokens: `in`, `cache-w`, `cache-r`, `out`, `hit` and the cost in dollars (see [Tokens](#tokens)). The header gives the job's tokens and cost, runs and main session together. From 80% of the spend limit of a `created` or `planned` job, a line follows the header:

```
warning: spend is 93% of the $0.200 limit; at 100% new sub-agents are refused; raise it with rex limit 0g51gpfx9m --set 0.40
```

**`rex tree show [<tree>] [--json]`**: print a master tree with its jobs and its questions; with no tree, the newest tree of the repo that is not closed. `--json` gives `{"tree": … or null, "jobs": [...], "questions": [...]}`, which `/rex master resume` reads.

**`rex oms`**: print the grid agent for a set of inputs. Flags `--duration`, `--budget`, `--intelligence`, `--autonomy`, `--reliability`, `--consequence`, `--verification`, each `low|medium|high|none` (a flag left out is `none`), and `--json`. `inputs` gives the seven ratings in that order, `-` for none. The names are bare; the mod adds `super-rex:` when it hands one to an agent.

```console
$ rex oms --budget low --autonomy high --intelligence high --reliability high --consequence high --verification low
agent=model-medium-effort-high
demand=model-xhigh-effort-max
rules=T1,T2,E1,E2,E4,C1,C2,F1
conflict=budget
inputs=-LHHHHL
version=2
```

The function is version 2. A role gets the `xhigh` tier only when its consequence is `high`: with a lower consequence, the tier stops at `high` and rule `T3` shows in `rules`.

**`rex skills`**: the group of `list` and `check`. On its own it prints its help on stderr and exits 2.

**`rex skills list [--json]`**: print each skill of the repo's `.claude/skills/`, of the repo's `.rex/skills/` and of `~/.claude/skills/` as a block of six lines (`name`, `description`, `when_to_use`, `source`, `shadowed`, `path`). `source` is `repo`, `library` or `user`. It does not list the plugin's skills: the binary knows no plugin. `/` in a session lists those.

**`rex skills check <path>...`**: check skill files, skill folders, or a folder of skill folders against the rules in "Reference". It prints `ok <path>` or one line per finding, and exits 1 on any finding.

```console
$ rex skills check .rex/skills
.rex/skills/shop-notes/SKILL.md: H4: description holds `Use when`; move the "use when" text into when_to_use
.rex/skills/shop-notes/SKILL.md: H5: when_to_use is missing
.rex/skills/shop-notes/SKILL.md: B4: rex-shop-style/SKILL.md names `rex-shop-style`, which is no skill in plugin/skills, plugin/library or .rex/skills
ok .rex/skills/shop-style
error: 3 findings in 1 of 2 skills
```

**`rex recall <job> [--json]`**: print up to 5 lessons from similar jobs, up to 3 orchestrators those jobs used with how they turned out and what they cost, and every orchestrator in the library whose newest version never ran. A job is similar when at least 2 of work type, size, consequence and languages are equal. While the job is `created`, the result is also stored on the job (`recorded=yes`). The builder gets it through `rex_recall`.

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

`cost_usd` is what the lesson's job cost at list prices. `cost_per_job_usd` is the mean cost of the orchestrator's similar jobs that have a stored cost. Both are `none` when nothing is stored, and print as `≥0.120000` when requests on a model with no price are left out. `cache_hit` is the cache hit % (`85%`), or `none`. The orchestrator lines come in this order: highest `best_match`, then most `accepted`, then fewest `missed`, then lowest `cost_per_job_usd`, then most `runs`, then name. When two orchestrators suit the work and have equal results, the decision agent prefers the cheaper one.

**`rex jobs [--json] [--search <text>]... [--since <date>] [--until <date>]`**: list jobs, newest first, one line each. `--search` matches the request text, ignoring case; repeat it to require several. Dates are `YYYY-MM-DD`, UTC, both ends included. `--json` adds the origin, the builder, the tree, deliveries, run counts, tokens, cost and `has_feedback`.

```console
$ rex jobs --search parser
0gaq820001 created fix small low rust 2026-11-13T20:00:02.000Z Fix the parser. Merge to main: yes
```

**`rex watch`**: a live dashboard for people at a shell, beside a Claude Code session, redrawn every 2 seconds. It reads two sources and shows them as one: the live snapshot that each session with the plugin writes to `.rex/live/<session>.json` of the main checkout in every storage mode, and the store `.rex/rex.db` when the repo has one. A tree or a job comes from a live session while one holds it (`●`), else from the store, else from the last snapshot of a session that ended or has been idle for 30 seconds (`○`). It never shows less than it knows: a job from the store with no cost of record yet shows the live tally of the newest snapshot that holds one, and a tree from the store spends what its jobs show plus its master's own spend from a snapshot; a spend with a tally in it starts with `≈` and says `live tally, not the cost of record` among the facts. With storage off it shows what the snapshots hold, so a job of an ended session stays until its snapshot is a week old, and `rex watch` removes such snapshots when it starts; it leaves alone any other file in `.rex/live/`, a snapshot of another version among them. It works from any folder of the repo or of a worktree. The home screen lists each master tree and each job of no tree, newest first. A tree shows its state, charter, spend of its cap (its master's spend and that of every job; the store holds only the jobs'), its master with its own cost, and its notes, then its jobs; a job shows its status, request, orchestrator, merge answer, spend of its limit, branch, deliveries, done checks and tokens, then every agent below it as an indented tree; an agent shows its role, task, the grid agent planned against the model and effort that ran, tokens, hit %, cost, brief check, skills and report, then the agents it started. An agent is named by its role and its task (`engineer T1`), by its task while it has no role, and by its grid agent when it has neither. A role that is the word of its level is not said again: `worker T1`, `lead`. The top line is the path from home to what is in view, and the line below says how deep that is and names the levels of the run with the current one marked: `level 2 of 5   master › [builder] › lead › worker › helper` in a tree, `level 1 of 3   [builder] › lead › worker` for a job of no tree; `helper` shows only when the run goes that deep. A narrow terminal cuts the path from home first and drops the row columns detail, hit, time and model in that order; a short one keeps half the screen for the rows and says how many facts it hides. A question that waits shows on its tree and its job, first among the facts, with its options numbered and the command to type in its session, `/rex answer <id> <option number>`. Up and Down (or `k` and `j`) move, Enter (or Right, `l`) goes in, Esc (or Left, `h`, Backspace) goes up one level, Home or `g` goes home, `?` shows the keys, `q` or Ctrl-C quits; the footer names the keys of the screen, the source and the age of the last read. `rex watch --once` prints the whole view once as a plain-text tree, one line per node, and needs no terminal. Without `--once` and without a terminal it exits 1. In a session, `/rex watch` opens the pane, which shows the live book.

**`rex feedback <job> --verdict accepted|rejected (--said <text> | --said-file <path>)`**: record what the user said about a finished job, and move its status. With `rejected` it also takes `--cause` (repeatable: `human-error`, `missing-context`, `agent-skill`, `agent-model-effort`, `agent-orchestration`), `--lesson <text>` or `--lesson-file <path>` (at most 600 characters), `--evidence <text>`, `--changed-skill <name>` and `--changed-orchestrator <name>`. A cause needs a lesson, and a lesson needs a cause. `--by <who>` defaults to `user`. The `rex-feedback` skill calls it; you rarely need to.

```console
$ rex feedback 0g51g6fykv --verdict rejected --said "It also rejected negative numbers in the import." \
    --cause missing-context --lesson "Ask which inputs share the parser before changing its rules."
job=0g51g6fykv
feedback=1
previous=delivered
status=analyzed
```

**`rex worktree add <repo> <dir> <commit> [--branch <name>]`** and **`rex worktree remove <repo> <dir>`**: the mod's own commands for a job's worktrees (a person can run them too). They make or remove a job's worktree in `<repo>.worktrees/`, beside the main checkout, and list or unlist its real path in `permissions.additionalDirectories` of `.claude/settings.local.json`, so that git commits there inside the sandbox. `<repo>` is the main checkout, `<dir>` an absolute path under `<parent>/<repo>.worktrees/`, and `<commit>` the full hash the worktree starts from. With `--branch` the worktree gets that new branch, or checks out that branch when it exists already and `<commit>` is its tip (a branch at another commit is refused); with none it is detached. `remove` never uses `--force`, and keeps a worktree with uncommitted work, and its entry. Each refuses a bad argument with exit 2 before anything is made, such as a path outside that folder or a commit that is not a full hash:

```console
$ rex worktree add /home/ana/code/shop /home/ana/code/shop.worktrees/0g51g6fykv 5f2c… --branch rex/0g51g6fykv
worktree=/home/ana/code/shop.worktrees/0g51g6fykv
settings=added
$ rex worktree add /home/ana/code/shop /home/ana/code/shop.worktrees/x abc123
error: abc123 is not a full commit hash (40 or 64 hex digits); give the output of git rev-parse
$ rex worktree remove /home/ana/code/shop /home/ana/code/shop.worktrees/0g51g6fykv
worktree=/home/ana/code/shop.worktrees/0g51g6fykv
settings=removed
```

The mod calls it for every job worktree and unit worktree of the job repo: `rex_job_open` and `rex_build` make the job worktree, `rex_worktree` makes a unit worktree, and `rex_job_close` removes them. The mod runs it outside the sandbox, so no agent of a job runs it for the job repo. An agent runs it only for a sibling repo, and `/super-rex:rex-pr-review` for its review folder, each as one plain command. You run it yourself only to remove a worktree that the mod kept and named in a `kept.worktree=` line.

**`rex test-filter`**: read a test run's output on stdin and print only its summary and its failures, for a test that you run at a shell (`cargo test 2>&1 | rex test-filter`). In a session the mod cuts test output itself.

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
rex migrate       — Move the store to the current version. Safe to run twice
...
```

## The life of a job

| Status | Meaning |
| --- | --- |
| `created` | The request is recorded and classified. |
| `planned` | The orchestrator and the role table are recorded. |
| `running` | Shown, never stored: a `planned` job with sub-agents open. |
| `delivered` | The work is finished. How each repo was delivered is in its delivery. No verdict yet. |
| `blocked` | Closed, and waiting on something outside the session, such as a person's approval on a protected main, a spend limit, or a question that was not answered. |
| `failed` | The work stopped and cannot go on without being redone. |
| `cancelled` | Stopped by the user. |
| `lost` | Closed: the session that held the job ended, and the job cannot go on. |
| `accepted` | The user confirmed it hit the mark. |
| `rejected` | The user said it missed. An analysis is due. |
| `analyzed` | The cause and the lesson are recorded. |

The builder moves a job with the mod's tools: `rex_job_open` makes it `created`, `rex_job_plan` makes it `planned` (once, unless `amend`), and `rex_job_close` closes it `delivered`, `blocked`, `failed` or `cancelled`. A close that repeats the job's closed status changes nothing. A closed job is not closed again with another status, except that a `blocked` job can still be closed when the rest is done. The feedback moves are those of `rex feedback`:

| Command | Status before | Status after |
| --- | --- | --- |
| `rex feedback`, any verdict | `created` or `planned` | refused: close the job first |
| `rex feedback --verdict accepted` | `delivered` or `accepted` | `accepted` |
| `rex feedback --verdict rejected`, no cause and lesson | `delivered` or `accepted` | `rejected` |
| `rex feedback --verdict rejected` with cause and lesson | `delivered`, `accepted` or `rejected` | `analyzed` |
| `rex feedback` without cause and lesson | `rejected` | refused: the analysis is due |
| `rex feedback`, any verdict | `blocked`, `failed`, `cancelled`, `lost` or `analyzed` | unchanged; the feedback is still recorded |

Each delivery has one outcome: `merged` (one squash commit on the base, through `rex_merge`), `pr-open` (no merge agreed; the pull request waits for a person), `kept-local` (no merge agreed and no pull request possible; the branch is kept), or `blocked` (a merge was agreed but could not be completed). Every outcome but `merged` carries a note that says what remains.

Feedback is always started by you: run `/super-rex:rex-feedback` and say what was wrong or right. The skill finds the job, reads the record, finds the cause, may change a skill or an orchestrator, and records it all with one `rex feedback` call. A shipped skill is never edited in the plugin, because an update replaces it: on its first change, the skill is copied into `.rex/skills/<name>/`, which `rex_skill` reads first, and the change is committed there.

## The record

With storage on, the mod sends the store an event for each prompt it acted on, each job change, each run that starts or stops, each skill that a run opened, each decision, each question and each merge. A run record holds the sub-agent's ID and its parent, its role (from the plan, by ancestry), the agent type that was planned and the one that ran, the brief and its hash, whether the mod assembled it and how its check went, the skills it was given and opened, start and stop times, its final message and its report file.

The store finds the transcripts in two ways. Every classic hook event of Claude Code but `PreToolUse` names the session's main transcript, and the first one that comes after the session's book exists makes the mod send that path once, in an event of its own (`session.transcript`); the store keeps the first path of a session. When no path reached the store, `rex` looks where Claude Code keeps a session's transcript: `<config folder>/projects/<folder>/<session id>.jsonl`, where the config folder is `CLAUDE_CONFIG_DIR`, else `~/.claude`, and `<folder>` is the folder of the session's jobs with each character that is not an ASCII letter or digit written as `-` (`/Users/me/my.repo` gives `-Users-me-my-repo`). Each sub-agent's transcript is beside the main one, in `<session id>/subagents/agent-<agent id>.jsonl`.

Tokens are summed from the transcripts, never from the mod's live tally: input, output, cache read and cache creation. The mod keeps its own tally for the band and the gate, and the transcripts are the figure of record. A total is **provisional** (`~`) while it may still grow. It becomes final only after the job is closed and two later reads, at least 10 seconds apart, find the transcript unchanged. A job's total includes the main session's own share.

A run is **open** while it shows activity in the last 15 minutes, **finished** when its stop is known, and **unconfirmed** when it is neither. A job is **incomplete** when its record may miss facts: a run is unconfirmed, a finished run has no tokens, a transcript was damaged or could not be read, or the main session's tokens are unknown.

### Tokens

Every API call sends the whole conversation again, and most of it comes from the prompt cache. So a long run reads its context again on each call, and cache reads make up most of its tokens. They cost about a tenth of the input price. Claude Code writes the counts of each request into the transcript of the agent that made it, the main session's or a sub-agent's, found as "The record" says. `rex` keeps the four counts apart, and never adds them into one figure:

| Shown as | `rex job show` key | What it counts |
| --- | --- | --- |
| `in` | `input` | Prompt tokens sent without the cache. |
| `cache-w` | `cache_write` | Prompt tokens written to the cache. |
| `cache-r` | `cache_read` | Prompt tokens read from the cache, once for each call. |
| `out` | `output` | Tokens the model wrote. |
| `hit` | `cache_hit` | `cache-r` as a whole percent of all prompt tokens (`in` + `cache-w` + `cache-r`), cut, not rounded; `–` or `none` with no prompt token. |
| `cost` / `$…` | `cost_usd` | The tokens' cost in US dollars at Anthropic's list prices, each request at the price of the model that ran it. `≥` in front, and `unpriced_requests` above 0, when some requests ran on a model with no price in `rex`, so the figure leaves them out. |

The cost is what the requests would cost on the Claude API, whatever plan pays for them; on a subscription it is a measure, not a bill. The prices are in `src/capture/price.rs`, and the mod's live tally uses the same table (`fixtures/shared/tables/prices.json`): per model, the input and output price, the cache read price, and cache writes at 1.25 times the input price for 5 minutes and 2 times for 1 hour; fast mode doubles them, and Claude Haiku 5.5 has a second rate card above 100,000 prompt tokens. The live tally prices every cache write at the 5-minute rate, because a step's usage does not split it, so the band can show a little less than the cost of record.

## Delivery

`rex-deliver` does every Git and GitHub step, once for each repo the job changes:

- One job branch per repo, `rex/<job ID>`. The job works in its worktree `<repo>.worktrees/<job ID>`, beside the main checkout, which the mod makes when the job opens. Parallel writers get unit branches `<job branch>--<unit>` in worktrees `<repo>.worktrees/<job ID>--<unit>`, which the mod makes when a lead calls `rex_worktree`. The mod runs `rex worktree add` for each one, outside the sandbox, so no agent of the job runs a worktree command for the job repo. A sibling repo's worktree is the one the builder still makes, with one plain `rex worktree add`.
- The base is merged into the job branch before delivery, never rebased, and the checks run again on the result.
- **A merge only when one was agreed, only in the job's own repo, and only through `rex_merge`.** The work becomes one squash commit over the base, and the branch `<job branch>--work` keeps the commits it was made from. `rex_merge` takes no repo, branch or base: it reads them from the caller's own job, and reads the repo's mode from git. It takes the repo's merge lock, checks that the branch is one squash commit over the base, checks the merge answer (and, for a master job, the policy), and moves the base as a fast forward. In mode `origin` (a remote `origin` that holds the base) it fetches `origin/<base>`, checks against it, and pushes the branch to it. In mode `none` (no `origin`, or one that holds no base yet) it checks against the local base, with no fetch and no push, and fast-forwards the local base: with `git merge --ff-only` in the worktree that has the base checked out, normally your main checkout, so the files appear there, or with a guarded `git update-ref` when no worktree has it (only while the base is still an ancestor of the job branch). When your main checkout holds changes or untracked files that the fast-forward would overwrite, `rex_merge` refuses with git's error and the paths, and asks you to commit or remove them. The delivery outcome is `merged` in both modes. A job of yours with no merge answer is refused; a master job with none parks a merge question and answers `merged=no` with its ID. The builder tries at most 3 rounds; past that the outcome is `blocked`.
- With no merge, the job branch is pushed and a pull request is opened when the repo is on GitHub and `gh` works (`pr-open`), else the branch is kept (`kept-local`). A sibling repo that the job changed gets a pull request or a kept branch, never a merge.
- A guarded clean-up by the mod, when `rex_job_close` has recorded the close, for every status. It removes each worktree of the job with `rex worktree remove`, which never forces, so a worktree with uncommitted or untracked files is kept. It deletes each local branch of the job (`rex/<job ID>`, `--work`, `--<unit>`) whose tip is on the base (`origin/<base>` after a `git fetch --prune origin`, or the local base), or below the tip of `--work` when that tip holds the same tree as the squash commit that `rex_merge` merged, or that never got a commit of its own. A branch with work that is not on the base is kept: it is the work of a job that did not merge. In mode `origin` it deletes the job's branches on `origin` that pass the same guard, with one `git push origin --delete` that names them; nothing is force-pushed. The close answers `removed.worktree=`, `removed.branch=`, `removed.remote_branch=` and `kept.worktree=`, `kept.branch=`, `kept.remote_branch=` lines, each kept one with its reason, and the builder tells you each kept one. A step that fails keeps its thing and never fails the close. After a merged job, nothing of it is left: no worktree and no job branch, here or on `origin`.

Only a person can start `/super-rex:rex-deliver`, for example for the clean-up after a person merged a kept branch; there, outside any job, the skill runs the guarded clean-up itself. Two more skills can be called by hand: `/super-rex:rex-pr-review` (an optional review of a pull request by an agent that wrote none of it; it posts one comment with the verdict PASS or FIX and changes nothing), and `/super-rex:rex-feedback`.

### Branch protection

The deny rules that `rex init` writes stop an agent's own `gh pr merge` and its pushes to main, and the mod refuses other spellings of a merge that it sees. But text rules can be got round, for example with `gh api` or a script. The real boundary is your Git host. For a repo where it matters, protect main on the host, and let only the credential that `rex_merge` pushes with change it. SuperRex does not set this up for you, and does not check it. A main that needs a person's approval is never bypassed: the job keeps its branch, and the note says what to do.

## Reference

**Output.** stdout holds the result and nothing else; warnings and errors go to stderr. Plain text, `key=value` lines in a fixed order, the most needed value first. `--json`, where a command has it, prints one JSON line with at least the facts of the text form. The mod's tools answer `key=value` lines too, and a refusal has the form `<tool>: <what is wrong>, got <value seen>; <what to do>`.

**Exit codes.** `0` done, also when there was nothing to do. `1` the command failed: the store, a job's state, a file, or a check that found something. `2` the command line is wrong, or the store has a version this `rex` cannot use. Every exit that is not 0 writes one line that starts with `error: ` on stderr, except `rex job`, `rex skills` and `rex tree` with no sub-command, which print their help, and `rex ingest`, whose refusals the mod reads (exit 2, 3 or 4, with no `error: ` prefix).

**Times** print as stored: UTC with milliseconds, `2026-11-10T18:16:54.174Z`. **Job IDs** are 10 lower-case letters and digits, such as `0g51g6fykv`; they sort by time.

**Files.**

| Path | What |
| --- | --- |
| `.rex/config.json` | The storage mode, the depth, `jobs_from_sdk` and the oldest Claude Code. In Git. |
| `.rex/rex.db` | The SQLite store, version 2: jobs, roles, deliveries, runs, decisions, skill manifests, questions, trees, feedback, the orchestrator index and the events the mod sent. Read it with `sqlite3`, but write it only through `rex`. |
| `.rex/skills/<name>/` | Skills of the repo itself: skills a job wrote, gap skills, and shipped skills that feedback changed. In Git. `rex_skill` reads them first. |
| `.rex/orchestrators/<name>/` | The orchestrator library: one folder with a `SKILL.md` per orchestrator. A changed folder gets a new version at the next plan or recall. |
| `.rex/live/<session id>.json` | The live view of one session for `rex watch`: its trees, jobs, runs and questions, rewritten after each change and at least every 10 seconds while a job or a tree is open. A tree's spend is its master's and every job's together. It holds no prompt, brief or ingest secret, and cuts a charter or a request at 1,000 characters. Ignored by Git. |
| `.rex/plans/<job-id>/` | The reports of a job's agents, and the plan that a plan job writes, `plan.md`. A master tree writes `final-report.md` in `.rex/plans/<tree id>/`. |
| `<repo>.worktrees/<job-id>` | The worktree of a job, beside the main checkout, which `rex_job_close` removes when it holds no uncommitted work. |

**How `rex` finds the store.** Every command except `rex init` uses the first of these that holds a `rex.db`: `REX_STORE` (the path of a `.rex` folder, used as is and never skipped), `CLAUDE_PROJECT_DIR`, then the working folder. From a sub-folder or a Git worktree, `rex` walks up to the main checkout. From a sibling repo, set the variable for the command: `REX_STORE=/home/ana/code/shop/.rex rex job show 0g51g6fykv`.

**The tag block.** The mod puts two lines at the top of every brief that it assembles. They are for the agent to read; the mod never takes them as the agent's identity.

```
Rex-Job: 0g51g6fykv
Rex-Role: engineer
```

**The skill header.** A skill is a folder with a `SKILL.md` that opens with a YAML header:

```yaml
---
name: shop-notes
description: How this team writes release notes.
when_to_use: Use when you write release notes, or when the user says "release notes".
---
```

An agent reads all of `SKILL.md`, so it holds only the rules an agent needs every time and a table that says when to read each sibling file. A sibling file sits in the skill's folder and holds the text for one step or one case. `SKILL.md` names it as `<skill>/<file>.md`, such as `rex-deliver/clean-up.md`, and an agent reads it with `rex_skill` only when it reaches that step or case. A skill can sit below its folder in subfolders too, as the builder's stage files do.

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
| B3 | Each relative link to a `.md` file points to a file of the same skill: `rex_skill` reads only inside one skill. |
| B4 | Each code span `<skill>/<file>.md` names a Markdown file of a skill in `plugin/skills` or `plugin/library` of the plugin, in the repo's `.rex/skills`, or beside a checked skill. A span that names a skill file by its folder (`.claude/skills/`, `.rex/skills/`, `plugin/skills/`, `plugin/library/`) is a finding. A span with a placeholder is a template. A span whose first folder is no skill and does not start with `rex-` is a path in the repo and is not checked. |
| B5 | The text after the header of `SKILL.md` has at most 1,500 words, counted as `wc -w` counts them. |

## Skills and the agent grid

The plugin holds 30 skills in two folders. Claude Code lists the 6 in `plugin/skills/` as commands of the plugin; it does not list the 24 of the library, `plugin/library/`, so they cost no context in a session. Each brief gets its read-first skills from the mod, and an agent opens the others with `rex_skill`.

The skills that you start:

- **The builder:** `/super-rex:orchestration-builder <request>` or `/super-rex:orchestration-builder Execute plan <path>`. Only you start it. Its three stages and the challenge of their decision are files in its folder, under `stages/`: `1-skills/`, `2-work-breakdown/`, `3-orchestration/` and `challenge/`. Each stage file holds at most 1,500 words.
- **Delivery:** `rex-deliver` (only you start it), `rex-pr-review` and `rex-feedback`.
- **`rex-grill-me`:** it questions you about a plan or a design until the two of you agree on each decision.
- **The router, `rex-rules`:** a session that writes or reviews code without the builder loads it by itself, or you start it. It tells the session to read the four rulebooks that every task that writes code gets, then the rows of the assignment table that fit its work.

The skill library:

- **Rulebooks:** 19 `rex-code-*` skills for writing code. Most are written for Rust, and they apply to every language: an agent applies the intent of each rule in that language's idiom, and a gap skill covers what the language lacks. A gap skill is named `rex-gap-<language>`, with the `--only` name of `rex codebase`. The library ships `rex-gap-ts` for TypeScript, and a job that needs another and finds none can have it written in `.rex/skills/`. The rulebooks cover philosophy, ergonomics, comments, errors and failure policy, crates, folders, architecture, smells, TDD and the four kinds of test, design sketch, spike, validation plan, priorities and latency.
- **Plan review:** `rex-codex-adversarial-review`, which every plan job runs before its plan is final.
- **`rex-mermaid-diagrams`:** how to draw diagrams in Mermaid.
- **`rex-skill-language`:** the language every skill and agent instruction is written in.
- **`rex-agent-prompting`:** how a brief fits the model and effort of the agent that gets it. `SKILL.md` holds the rules for every brief, and one file for each grid tier (`haiku.md`, `sonnet.md`, `opus.md`, `fable.md`) holds the text for each effort level, which the mod adds to each brief it assembles.

**The grid** is 20 agents in `plugin/agents/`, named `super-rex:model-<low|medium|high|xhigh>-effort-<low|medium|high|xhigh|max>`. The tiers are `haiku`, `sonnet`, `opus` and `fable`. An agent runs with exactly the model and effort in its file. Each file also holds `disallowedTools: Skill`, so a grid agent has no Skill tool and no list of skills: it gets its skills in its brief and through `rex_skill`.

**How `rex oms` picks one.** It is a fixed function of seven inputs, each low, medium, high or none (not relevant). Duration, budget and autonomy belong to the job and are given only when you state them; intelligence, reliability, consequence and verification are rated for each role. The tier starts from intelligence and goes up one step when the stakes (the higher of reliability and consequence) are high. That step reaches the top tier, `xhigh`, only when consequence itself is high. Otherwise the tier stops at `high`, and rule `T3` is printed. The effort starts from the stakes, goes up one level when the result is hard to check and down one when it is easy, and shifts one level with autonomy. A low budget caps the pair at `model-medium-effort-medium` and a medium one at `model-high-effort-high`; a low duration caps the effort at medium and a medium one at high. With high stakes the result is never below `model-medium-effort-high`, and the cap that lost is printed as `conflict`. All inputs none gives `model-medium-effort-medium`. In the `rex oms` example above, the demand was `model-xhigh-effort-max`; the low budget capped it at `model-medium-effort-medium`, and the high-stakes floor raised its effort to high.

## Known limits

- Without storage, a session's record ends with the session. A resumed or restarted session does not go on with a job of an earlier one.
- After `rex init --strict`, an agent's `rex worktree` runs inside the sandbox, which cannot write `.git/worktrees/`. The worktrees of the job repo are not touched by this: the mod makes and removes them itself, outside the sandbox. A sibling repo's job worktree, which the builder makes, cannot be made then; the builder gives you the plain `rex worktree add` command to run in your own shell.
- `rex_merge` merges by a squash push to `origin`, or in a repo with no `origin` by a fast-forward of the local base. A repo whose main takes changes only through pull requests keeps the job's branch.
- In a repo with no `origin`, a shell command could still move the local base around `rex_merge`. The mod refuses the plain spellings (a `git merge` of the job branch or of its `--work` branch, a `git branch` that forces, renames, copies or deletes the base, `git update-ref refs/heads/<base>`, a push or fetch from `.` into the base), but it is a text check and does not catch every spelling.
- The spend limit stops new sub-agents only. Agents that already run keep spending, so a job can end over its limit. A job whose requests all ran on models with no price never reaches it. The default limits are estimates from list prices, not measured costs.
- A request that Claude Code bills but writes to no transcript is not counted in the tokens, so a total can be lower than Claude Code's own figure.
- From a sibling repo, `rex` needs `REX_STORE` (see "How `rex` finds the store").
- A session started in a worktree outside the main checkout's folder is not tested. Start the session in the main checkout.
- `rex smells` reads Rust files only.

### What the live runs showed, and what they did not

Headless sessions on Sonnet with the plugin ran the builder end to end, with storage on and with storage off. Each opened a job, ran the decision agent and the challenger, had a brief sent back once by the check, planned, made the fix in a job worktree, merged it with `rex_merge` to a bare remote, and closed the job `delivered`. A session without the plugin stopped at step 1. The mod's token tally for the Opus runs of one session was equal to Claude Code's own count, token for token and to the nanodollar.

On 15 November and early on 16 November, interactive sessions of Claude Code 2.1.325 ran in a terminal with this release's plugin and `rex`, in scratch repos that `rex init` set up. The first two repos had no commit, the branch `master` and no remote:

- **The builder, storage off.** `/super-rex:orchestration-builder please build a hello world app …`. The mod made the first commit. The builder asked the one merge question and nothing about git. The decision agent, the challenger, an engineer and a verifier ran, `rex_merge` answered `merged=yes` on `master`, and the main checkout held the new file. In that run the builder still removed its own worktree, and the sandbox refused its compound command; the mod removes the worktrees now, as the run at every level below shows.
- **The master, storage on.** `/rex master this is a test. Please build both a hello world and goodbye world app …`. The first run asked the one policy question in a dialog. `rex_build` made the first commit and the job's worktree, and the mod started the master and the builder itself. The builder delivered one squash commit, merged on `master`. The mod woke the master at the delivery; the master read the done check and the job's report and closed the tree with `rex_report`. The main session dropped the turn notices, got the one closing message, and told the person the result. The band showed the tree, and `rex watch` showed it from the live snapshot.
- **A master tree at every level, storage on, with a bare repo on disk as `origin` and the base `main`.** `/rex master This is a test of the harness at every level. … Do it as one job. In that job the builder must use one lead that starts one worker for each app, in parallel, and each worker must start one helper agent that writes the test of its app. …`. After the one policy question, the master noted its plan and called `rex_build` once. The engine recorded the master and the builder at depth 1, the decision agent, the challenger and one lead at depth 2, two workers at depth 3 and a helper below each worker at depth 4, each on the grid agent of its plan role (Opus, Sonnet and Haiku). The master was not woken while the builder waited for its agents, and was woken once, at the delivery. `rex_merge` pushed the one squash commit to `origin/main`, and the local `main` was current. The mod's clean-up at `rex_job_close` left no job worktree and no job branch, here or on `origin`. The done checks `fetched`, `merged`, `base_current`, `branches_gone`, `worktrees_gone` and `output_exists` passed, and `checks_on_base` was `not-set`, as the policy named no `merge_checks`. The master closed the tree with `rex_report`, and the main session told the person the result. The store held the session's transcript path, from the `session.transcript` event, and `rex job show` gave the cost of record, `cost_usd=6.441092` with `incomplete=no`, against a live tally of $6.79 for the whole tree, its master included. `rex watch` showed the tree at every level, with the rail `level 2 of 5   master › [builder] › lead › worker › helper` on the job's screen.
- **A master with storage off, in a repo with commits and no remote (16 November).** `/rex master This is a small test of the harness. Add one command-line app to this Rust crate …`. The person answered the policy question with `Ask me for each job` (`merge=ask`). One job ran its decision agent, challenger, an engineer and a reviewer. `rex_merge` parked the merge question, which came up as a dialog. The person dismissed the dialog, and the band then showed `? 1 question waits on you · Merge job … · /rex answer q1-… <1-2>`. The person typed `/rex answer 1`, with no question id, which answered `answer=Merge` and woke the builder and the master. The builder merged on the local `main`, the mod cleaned up, the master closed the tree with `rex_report`, and the main session told the person the result. The tree cost $3.50.
- **`rex watch` with storage off** showed the running job and its agents from `.rex/live/` alone, with its drill-down, level rail and facts on a real terminal. The pane and the band showed, docked and inline.
- **The cost of record** of the first master run, whose store had no transcript path, read from its real transcripts on a copy of its store: with the copied transcripts where Claude Code keeps them, `rex job show` gave $1.736290 for 94 input, 144,519 cache-write, 2,736,898 cache-read and 23,297 output tokens, equal to the mod's live tally token for token, and `rex watch --once` showed it, with the tree at the job's cost plus its master's own tally.

**Not proven live.** These parts pass their tests, but no live run has shown them:

- `/rex master resume`.
- The watchdog's wake of an idle or a stuck builder, and the question after it; the retry of a builder after an API error.
- A question answered by a press in the pane, and a question that the master parks with `rex_ask_owner` and that is then answered. A run on older code showed the dialog of such a question, but not on this release's code.
- A tree of more than one job, with `depends_on` and `merge_after`.
- A repo with a remote on a real Git host, and a main branch there that needs a person's approval.
- A headless master (`claude -p`).
- That a running session picks up a worktree that `rex worktree add` lists in `permissions.additionalDirectories` after the session started, so that a sandboxed `git commit` works there. The headless runs listed the paths, but their own sandbox could not start inside the sandbox they ran in, so this rests on Claude Code's code.
- The fast-mode rates in the mod's live tally: a step does not say whether it ran in fast mode, so the tally prices each step at the standard rate. The cost of record, from the transcripts, is not affected.
- That a `.catch` of a hook that answers a refusal refuses: the closed checks of the mod refuse inside the hook instead, and the `.catch` form is shown in the tests only.
- The count of permission prompts in the band.
- That the prompt origin `bridge` (Remote Control) reaches the mod as Claude Code's documentation says, that the book survives a reload of the plugin, that `$.settings.read` gives the merged allow list for the headless permission check of a master, and that a skill typed by its bare name is taken as a job start.
- That `claude plugin marketplace add` and `claude plugin install --scope project` work as `rex init` runs them, and that a project-scope install records the plugin in `.claude/settings.json`. The tests use a stand-in `claude`.
- Linux without `bwrap` or `socat`: only the detection is tested. `sandbox.failIfUnavailable: true` was not tested on any system, and neither was the trivy cache path on Linux.

**Not run.** These could not run inside the sandbox of the agent that tested this release. You can run them yourself:

- A master run of two jobs in `-p` in which one job must ask you, so that the run ends `blocked`. It needs a policy that you confirm yourself. In a scratch repo after `rex init --with-storage`, run `/rex policy set allow_headless true`, `/rex policy set always_ask_paths ["LICENSE"]` and `/rex policy confirm` in an interactive session, then:

  ```sh
  CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS=0 claude -p '/rex master Two jobs. Job 1: fix the spelling of "recieve" in NOTES.md. Job 2: add the line "Copyright the authors." to the end of LICENSE. Each job merges to main when its checks pass.' --model sonnet --permission-mode bypassPermissions
  ```

  Job 1 merges (`merged=yes`). Job 2 parks a merge question (`merged=no`, `question=<id>`) and is `blocked` when the session ends.
- The five eval cases in `plugin/evals/`: `smoke-mod-answers`, `canary-notes-ask-for-a-job`, `builder-small-fix-storage-on`, `builder-small-fix-storage-off` and `master-two-jobs-one-escalates`. Each loads, but a run cannot start inside a sandbox. Run one from your own shell in the repo root, with this `rex` first on `PATH`, for example:

  ```sh
  claude plugin eval plugin --case smoke-mod-answers --ablation none --runs 1 --model sonnet --judge-model sonnet --scaffold --allow-tools mcp__super-rex__rex_status --no-publish
  ```

  The master case also needs the confirmed policy above, and `CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH=4` and `CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS=0` in the shell.
- Resuming a finished sub-agent with `SendMessage`, which needs its transcript. The builder's fallback, a new decision agent, worked in both runs.

**Not built.**

- The done check does not look for a commit of the job's branch on the base that `rex_merge` did not record. `rex_merge` itself refuses a merge outside it.
- The report file of a brief is the lowest free name among the runs that this session's book knows and the spawns still starting (`worker.md`, then `worker-2.md`), so two agents of one role get two files, but a file that an earlier session of the same job left can get a second report.
- The tier text that the mod adds to briefs is a hand copy of the four files of `rex-agent-prompting`, not one generated from them.
- The mod's check for a merge reads only the text of a command. A push that the text does not hold (a script, a git alias, a program that pushes by itself, a remote's `push` setting) or a tool other than `Bash` and `Monitor` that runs code is not seen. Your Git host is the boundary (see "Branch protection").

## Development

Build with `cargo build` and test with `cargo test`. The Rust tests live in three folders: `tests/integration/` (the binary end to end), `tests/contract/` (the seams between modules), and `tests/fitness/` (rules of the whole crate, such as that the plugin and the crate name one version). `tests/fixtures/` holds recorded Claude Code sessions, so no test needs a live Claude Code.

The mod is TypeScript below `plugin/hooks/`. Check it with `tsc -p plugin`, test it with `claude plugin test plugin`, and check the plugin with `claude plugin validate --strict plugin`. Run a session on the working copy with `claude --plugin-dir plugin`, or set this repo up with `rex init --plugin-source .`. The seams between the mod and the binary are files in `fixtures/shared/`: each call the mod makes to `rex`, with its expected output, and the tables both sides use. A Rust test runs the binary on each file, and a kit test checks the mod's side against the same file. `plugin/hooks/book/shared.ts` is a copy that `python3 scripts/gen-shared-fixtures.py` writes; `--check` fails when it is out of date.

This repo has no CI service. Its checks are git hooks in `.pre-commit-config.yaml`; run `pre-commit install` once to set up both hooks. At commit: the file checks and trivy. At push: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, `cargo check` with the oldest Rust that `Cargo.toml` claims (the hook installs that Rust when it is missing), and `cargo package`, with the tests run again from inside the package. For the mod, at commit and at push, a check that `plugin/hooks/book/shared.ts` equals `fixtures/shared`; at push, `tsc` of the mod and of its evals, and `claude plugin test plugin`. The push checks run on the work tree, not on the commits being pushed, so push from the branch that is checked out.

Run `scripts/check-claude-release.sh` when a new Claude Code comes out. It type-checks the mod and its evals, checks the shared copy, and runs `claude plugin validate --strict plugin` and `claude plugin test plugin`. `--renew-types` first copies the declarations of the Claude Code on `PATH` over `typings/claude-code/index.d.ts`. The committed file stays at the oldest Claude Code the plugin supports (2.1.323), so put it back after the checks with `git checkout -- typings/claude-code/index.d.ts`. `--live` also runs one headless session on Sonnet with this plugin and this tree's `rex`, in a scratch repo, capped at $0.50.

Two rules hold for every source file: at most 500 lines, and a module comment of one or two lines at the top that says what the file is for (`rex codebase --only code --with-context` shows them). The skills and agents in `plugin/skills/`, `plugin/library/` and `plugin/agents/` are source in this repo. The build embeds them in the binary for `rex skills check` and the tests, and `cargo test` checks every bundled skill with the rules of `rex skills check`, each stage file of the builder for at most 1,500 words, each grid agent for `disallowedTools: Skill`, and that each tool name and key that a skill names is in the mod's tool table.

**Licence:** MIT. See [LICENSE](LICENSE).
