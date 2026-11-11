# Changelog

This file lists the changes to SuperRex that a user can see, newest first. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the version numbers follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Changed

- `rex oms` is version 2: a role gets the `xhigh` tier only when its consequence is `high`. With a lower consequence the tier stops at `high` and rule `T3` shows in `rules`. `rex job show` prints `oms_version=2` on each role. The builder now starts its decision agent, challenger, skill writer and pull request reviewer on a grid agent from `rex oms`, not on the session's model or a fixed agent, and a plan job gets one adversarial review only. The role form `"session": true` is gone from `rex job plan` and `rex job show`, and the store schema changed with it: delete `.rex/rex.db` and run `rex init` again.
- `rex job close` prints `extra_skills=<n>` and `extra_skill.<i>=<role> <skill> skill_md_words=<words>` lines after the missed-skill lines: the skills a run opened that its role was not given, with the length of each `SKILL.md`. The key `missed_skills_incomplete` is now `skills_incomplete` (and `"skills_incomplete"` in `--json`, which also gains `"extra_skills"`). Change any script that reads the old key. The work breakdown and the role table have an `Open when` column: `Skills` holds at most 6 skills that an agent reads first, and `Open when` holds the ones it opens only when its work reaches their case.
- Stage 3 caps the sub-agents of a job by its size: 5 for a small job, 6 for a medium one, 10 for a large one, shown as `AGENTS: <count> of max <max>` in the decision. The challenger reports a plan over the cap as a `MAJOR` finding. Stage 2 builds each code feature in one task, and splits code only where files and outcomes are independent.
- `rex recall` lines gain `cost_usd` and `cache_hit` on each lesson, and `cost_per_job_usd` and `cache_hit` on each orchestrator (`--json` too). A cost with unpriced requests prints as `≥…`. Orchestrator lines sort by `best_match`, `accepted`, `missed`, the lowest `cost_per_job_usd`, then `runs`. The decision agent prefers the cheaper of two orchestrators with equal results.
- `rex job close` also prints `long_messages=<n>` and a `long_message.<i>=<role> <agent ID> chars=<n>` line for each run whose final message has more than 2,000 characters (`"long_messages"` in `--json`). Sub-agents now write their full output to `.rex/plans/<job>/<role>.md` and return the path with a short summary. Parallel writers write `<role>--<unit>.md`.
- `rex status` prints a `warning:` line after the header of a created or planned job from 80% of its spend limit.
- `rex init` writes five hook entries, not three, and the allow rule `Bash(rex test-filter)`; it counts 149 files, not 148. The `--no-hooks` warning names `rex hook`, `rex gate` and `rex test-hook`. A repo that was set up before gets the new entries by running `rex init` again.
- Tokens show as four counts and the cache hit %: `in`, `cache-w`, `cache-r`, `out` and `hit`, in `rex watch`, `rex status` and `rex job show`, instead of one sum of the four. Cache reads are counted again on each call, so the sum was mostly cache reads. The job list of `rex watch` shows the cost and `hit`. `rex job show` prints `tokens_input`, `tokens_cache_write`, `tokens_cache_read`, `tokens_output` and `tokens_cache_hit` in place of `tokens`, and the same keys without `tokens_` on each run line.

### Added

- `rex init` writes `sandbox.network.allowLocalBinding: true`, and, except with `--strict`, `sandbox.excludedCommands` for `git worktree`, `git merge`, `git pull`, `git switch` (also as `git -C <dir> …`) and `rex init`. The sandbox never lets a command write `.claude/skills`, `.claude/agents` or the settings files, so these commands could not check out a repo that tracks those folders. A file that sets `sandbox.allowUnsandboxedCommands: false` gets no excluded commands. Run `rex init` again to add them.
- A spend limit for each job, in US dollars at list prices. `rex gate`, a `PreToolUse` hook on `Agent|Task` that `rex init` registers, refuses a new sub-agent once the job's spend (sub-agents and main session) has reached its limit, and tells the agent to report the job `blocked` with the command that raises it. The default limit comes from the job's budget and size, the agent cap of the job's size plus 2 for the main session, times $1.50, $4 or $8 for a low, medium or high budget, from $10.50 (low budget, small job) to $96 (high budget, large job); no stated budget uses the medium row (small $28, medium $32, large $48). A refusal keeps the job open: the builder asks the requester to run `rex limit <job> --set <dollars>`, then goes on with the same job; the job closes `blocked` only if the requester declines. `rex limit <job>` prints `limit_usd=`, `spent_usd=` and `source=default|set`, and `rex limit <job> --set <dollars>` sets a new limit. Run `rex init` again in a repo that was set up before to get the hook.
- A cut of test output for agents. `rex test-hook`, a `PreToolUse` hook on `Bash` that `rex init` registers, runs a plain test command (`cargo test`, `pytest`, `go test` and others) through `rex test-filter`. `rex test-filter` first prints the path of a file that holds every line of the run. The changed command is `set -o pipefail; <test command> 2>&1 | rex test-filter`, a form that Claude Code's check of a Bash command lets through in the default permission mode, with the allow rules `Bash(rex test-filter)` and `Bash(set -o pipefail)`. The filter shows every line except the lines in a runner's exact passing form, and always keeps the summary. A command that holds a `cd`, `&&`, `#`, a backslash or an output flag such as `--nocapture` is left unchanged. The file has mode 0600 in the temporary folder, and later runs remove files older than 24 hours. The command keeps its exit status. The grid agents still load `CLAUDE.md` and `AGENTS.md`. Run `rex init` again in a repo that was set up before to get the hook and the allow rule.
- The cost in US dollars of every run, of the main session and of each job, at Anthropic's list prices for the model each request ran on: a `cost` column in `rex watch`, `$…` in `rex status`, `cost_usd` and `unpriced_requests` in `rex job show`, and `cost` in the JSON of `rex job show` and `rex jobs`. Claude Code writes no cost per sub-agent, so `rex` works it out from each request's usage, with 5-minute and 1-hour cache writes, fast mode and the Claude Haiku 5.5 long-prompt rates. The store has two new columns: delete `.rex/rex.db` and run `rex init` again.
- The runs panel of `rex watch` names the job's orchestrator skill and version, such as `pipeline v1`.
- The main session's tokens: a `session` row first in the run tree of `rex watch`, a `session` line in `rex job show`, and the job total in the `rex status` header and the job panel of `rex watch`, so the rows add up to the job.

## 0.4.0 - 2026-11-11

### Added

- `rex-rules`, a router skill. A session that writes or reviews code without the builder reads the four core rulebooks through it, then the rows of the assignment table that fit its work.

### Changed

- The 22 skills that are only read by path, never started by a person, now live in `.rex/skills/`, the skill library: the 19 `rex-code-*` rulebooks, `rex-skill-language`, `rex-mermaid-diagrams` and `rex-codex-adversarial-review`. Claude Code no longer lists them in every session. `.claude/skills/` keeps the 6 that a person or a session starts: `orchestration-builder`, `rex-deliver`, `rex-feedback`, `rex-pr-review`, `rex-grill-me` and `rex-rules`. The rulebooks' `/name` commands are gone.
- `rex init` installs the library into `.rex/skills/`. `rex skills list` shows its skills as `source=library`. `rex skills check` B4 knows both folders and names the right one for a path to the wrong folder. A read below `.rex/skills/<name>/` counts as opening `<name>` in the job record. A new skill that stage 1 writes goes into the library.
- `rex-deliver` keeps the store out of Git with `/.rex/*` in `.git/info/exclude`, tested on `.rex/rex.db`, so a repo can still track `.rex/skills/`.

### Fixed

- `rex init` ignores `/.rex/*`, not `/.rex/`, like `rex-deliver`, so a repo can track `.rex/skills/` with `!/.rex/skills/`. This repo has its own ignore block, so `rex init` here no longer hides new skill and agent files from Git.
- A worker in a job worktree is told that skill paths start at the main checkout, and the parallel-writer brief lets it read its skill files outside its worktree.
- `skills_opened` is `[]` when the run's transcript was read and it opened no skill, and `null` when no transcript was read. `rex-feedback` compares it with each role's `skills`.
- `rex skills list` in a linked worktree also lists the main checkout's library. `rex skills check` takes the bundled skills as known only for a folder the repo does not have. `rex job plan` refuses a skill name that is not a folder name, such as `rex-code-tdd/mocking.md`.
- The skills: full paths in the assignment table, one home each for the core rulebooks, the `SLUG` form and the sandbox brief text, gap skills named `rex-gap-<language>`, the plan writer gets the skill list, a pull request review on request runs in a fresh agent, and text of earlier designs is gone (the builder branch for a sub-agent, "stage sub-agents", the unused mermaid `README.md`).

### Removed

- All backward compatibility, because no repo uses SuperRex yet. The store's schema is version 1 again, with `role.skills` part of it, and has no upgrade path: a store made by 0.3.0 is refused, so delete its `.rex/rex.db` and run `rex init`. `rex init` no longer names old stage-skill folders. `rex codebase --rust-only` is gone; use `--only rust`.

## 0.3.0 - 2026-11-11

### Added

- One assignment table, `.claude/skills/orchestration-builder/stages/1-skills/assign.md`, says which skills each kind of work gets. The decision agent starts from it and gives a reason for each skill it adds or leaves out, and the challenger checks the decision against it.
- `rex job plan`: a role in the plan file may give `"skills"`, the skills its tasks are assigned. Each one must also be in the job's `skills`. `rex job show` prints each role's skills.
- `rex job close` lists each skill that a role was assigned and that no run of that role opened (`missed_skill.<i>=`, `role_not_run.<i>=`; `missed_skills`, `roles_not_run` and `missed_skills_incomplete` in `--json`). The status and the exit code do not change. The builder tells the person about each one.
- `rex codebase --with-context` and `--only` support Java, Kotlin, Scala, C#, Swift, Objective-C, Dart, PHP, Ruby, Lua, Haskell, Elixir, R, SQL, Zig and Perl. `.m` is Objective-C and `.pl` is Perl.

### Changed

- `orchestration-builder` and `rex-deliver` start only from a person: `/orchestration-builder <request>`, `/orchestration-builder Execute plan <path>`, `/rex-deliver`. No agent can start them, and their descriptions no longer take up room in every session.
- The four stage skills are now files inside the builder, in `.claude/skills/orchestration-builder/stages/`: 31 skills become 27. `rex init` names any of the four old folders that an earlier install left in `.claude/skills/`; delete them by hand.
- The 20 model and effort agents have no Skill tool (`disallowedTools: Skill`). A worker reads only the files its brief names, and the list of every skill no longer fills its context.
- The Rust rulebooks apply to every language: an agent applies the intent of each rule in that language's idiom. The module comment and priority block rules are no longer tied to `//!`, and agents find the priority block with `git grep -n 'Priorities, in order:'`.
- The store's schema is now version 2 (`role.skills`). The first `rex` run in a version 1 store upgrades it in place. After that, an older `rex` refuses the store and asks for a newer one.

## 0.2.1 - 2026-11-11

### Changed

- Eleven places where two skills told an agent different things now agree, each settled by the skill whose whole subject it is. An error that is only logged is not handled (`rex-code-failure-policy`). Doc comments are for the public API of a library and never restate the name (`rex-code-commenting`). A file's top comment names no other file, and the crate root may add the priority block. A `SMELL` flag stays until the commit that fixes its problem. A `TODO` is fixed, deleted, or made a `SMELL` flag. "Seam" has one meaning. Prototype first when the domain is unknown, test first when the plan gives the interface. No mock of your own code. A sub-agent that runs TDD asks its caller, never the person. Pre-release crates are tried in a spike outside the repo. Unused crates are removed, found with `cargo machete`.
- "Core rules", which the orchestration skills used without defining, now names the skills: `rex-code-philosophy` and `rex-code-commenting` for every language, `rex-code-ergonomics` and `rex-code-error-writing` for Rust, and one skill for each kind of work. Stage 1 puts them in the skill list of a job that writes code.
- `rex-code-external-libraries` has the rule again that a crate's types stay out of your public API unless that crate is part of your contract.

## 0.2.0 - 2026-11-11

### Added

- The skill `rex-code-latency`, a reference for latency in Rust code that runs in nanoseconds to tens of microseconds. Its `SKILL.md` is a router: measure first, then a table from what you see to the one or two files to read. It covers the operating system, CPU caches and branches, allocation and reclamation, atomics and queues, the network receive path, and how to measure without changing the result. Each topic has a rules file of at most 900 words and a code file that an agent opens only when it writes that code.
- A paired challenger checks the builder's decision before any work starts. The new skill `orchestration-decision-challenge` has a fresh grid agent (`model-high-effort-high`, or `model-xhigh-effort-high` for a large or high-consequence job) look for important skills that were left out or not given to a role, and for gaps in the breakdown, the orchestrator and the ratings. The builder corrects what holds before `rex job plan`, and the work breakdown records the check.
- Plan jobs. A request for a plan writes `.rex/plans/<job-id>/plan.md` and changes no file in the repo. A later run of `orchestration-builder` with "Execute plan <path>" does the work. `orchestration-selection` now says how to build an orchestrator for planning.
- The skill `rex-codex-adversarial-review`. Every plan job runs it before its plan is final: the Codex CLI challenges the plan read-only, the runner checks each finding, and there is one rebuttal round. Without Codex, a grid agent of `model-high-effort-high` or above is the challenger.
- Two size rules in `rex skills check`. `H9`: `description` and `when_to_use` together have at most 60 words, because Claude Code lists every skill in every session. `B5`: the text of a `SKILL.md` after its header has at most 1,500 words, counted as `wc -w` counts them.

### Changed

- The builder's stage skills now carry the `orchestration-` prefix: `nuance-selection` is `orchestration-nuance-selection` and `work-breakdown` is `orchestration-work-breakdown`. A repo set up by an earlier `rex init` gets the new folders on its next run; delete the two old folders from `.claude/skills/` by hand.
- The three stage skills `orchestration-nuance-selection`, `orchestration-work-breakdown` and `orchestration-selection` no longer appear in the `/` menu (`user-invocable: false`). Only the builder runs them, because each stage needs the job's inputs; Claude can still load them.
- `rex watch` has a colour theme: coloured statuses and states with symbols, rounded panels, a top bar with counts, a details panel above the run tree, and tree lines.
- The bundled skills are split into a core and sibling files. Each `SKILL.md` is a short core that an agent always reads, with the rules it needs every time, and says when to read each sibling file in the skill's folder, which holds the text for one step or one case. A repo that already has the skills must run `rex init --force` to get the new `SKILL.md` files, because `rex init` does not overwrite a skill file that exists. Without `--force` it adds the sibling files and keeps the old, long `SKILL.md` files. `--force` replaces every skill and agent file that differs from this version, so a change you made to a bundled skill is lost.
- Rule `B4` of `rex skills check` also checks a path to a sibling file, such as `.claude/skills/rex-deliver/start.md`: the skill must be known, and the file must be in its folder or in a folder below it. A path with a placeholder, such as `.claude/skills/<name>/<file>.md`, is a template and is not checked.
- The builder hands its decision to a decision agent. A sub-agent with the role `decision-maker` runs at the session's model and effort, works through the three stages, and returns one decision block: the skills, the work breakdown, the orchestrator and the role table. The paired challenger checks that block, and the decision agent corrects it, so the session never reads the stage skills and does not carry them while it runs and delivers the job. Each task in the work breakdown now names the skills it needs, and a brief names only the skills of its own task, so a worker no longer reads every skill of its role. The four stage skills set `disable-model-invocation: true`, because only the builder flow reads them, by their paths.
- A read of any Markdown file in a skill's folder, at any depth, now counts as opening that skill in the job record, not only a read of its `SKILL.md`. That holds for the skills folders `.claude/skills/<name>/` and for the orchestrator library `.rex/orchestrators/<name>/`. A skill counts once however many of its files are read, and a Markdown file outside a skill's folder, such as `.claude/skills/README.md`, opens no skill.

## 0.1.0 - 2026-11-10

The first version. SuperRex supersedes rex-skills (Rex 2.0).

### Added

- The `rex` binary, with the commands `init`, `job` (`create`, `plan`, `close`, `show`), `hook`, `status`, `oms`, `skills` (`list`, `check`), `recall`, `jobs`, `watch`, `feedback`, `codebase`, `smells` and `commands`.
- `rex init`, which sets a repo up in one run that is safe to repeat: the skills and agents, the Claude Code sandbox settings and hooks, the store in `.rex/`, the rex sections of `AGENTS.md`, the block in `.gitignore`, and a pre-commit config.
- 28 skills: the builder `orchestration-builder` with its three stages, the delivery skills `rex-deliver`, `rex-pr-review` and `rex-feedback`, 18 `rex-code-*` rulebooks, `rex-grill-me`, `rex-mermaid-diagrams` and `rex-skill-language`.
- The grid of 20 agents, one for each pair of model tier and effort level, and `rex oms`, the fixed function that picks one from a role's ratings.
- A local record of each job in `.rex/rex.db`: the request, the plan, the roles, the sub-agent runs with their tokens, the deliveries and the feedback.
- Telemetry through three Claude Code hooks, with tokens summed from the transcripts. A stop event that Claude Code sends for a call of its own, such as the reply it suggests after a turn, gives no run.
- `rex watch`, a live view of jobs and their sub-agent runs.
- Lessons: `rex feedback` stores the cause and the lesson of a job that missed, and `rex recall` gives them to later jobs that are similar.
- Commands outside the sandbox. A sandboxed command reaches the network only through the sandbox's proxy, so a program that opens its own connection, such as a WebSocket client or a database driver, cannot connect. `rex init` writes `sandbox.allowUnsandboxedCommands: true`, so that an agent can run such a command again outside the sandbox, and the `orchestration-builder` skill says when an agent does that. `rex init --strict` writes `false`, so that no command leaves the sandbox.

### Changed

- The installer of rex-skills, `rex skills`, is now `rex init`. `rex skills` now lists and checks skills.

### If you used a build from before this release

- A settings file that such a build wrote holds `sandbox.allowUnsandboxedCommands: false`. `rex init` keeps that value and warns. Set it to `true` by hand to let agents run a command outside the sandbox.
- Run `rex init --force` in a repo that is already set up, to get the skill text of this version.
- Such a build recorded a run for each reply that Claude Code suggests after a turn. Those runs stay in the store. This version records no new ones.
