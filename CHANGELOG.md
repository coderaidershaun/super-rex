# Changelog

This file lists the changes to SuperRex that a user can see, newest first. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the version numbers follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

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
