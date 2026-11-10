# Changelog

This file lists the changes to SuperRex that a user can see, newest first. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the version numbers follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
