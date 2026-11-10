# Changelog

This file lists the changes to SuperRex that a user can see, newest first. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the version numbers follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added

- `rex init --strict` writes `sandbox.allowUnsandboxedCommands: false`, so that no command leaves the sandbox.
- The `orchestration-builder` skill has a "Sandbox" section. It tells agents to run outside the sandbox only the one command that cannot reach the network, never a command that the sandbox refused for a write, and to name each such command in their report. It also tells them never to give the user a command that an agent can run.

### Changed

- `rex init` now writes `sandbox.allowUnsandboxedCommands: true`. A sandboxed command reaches the network only through the sandbox's proxy, so a program that opens its own connection, such as a WebSocket client or a database driver, cannot connect. An agent can now run such a command again outside the sandbox. Before, `rex init` wrote `false`, and the agent gave the command to the user.
- A settings file that already holds `false` keeps it, and `rex init` warns. Set it to `true` by hand to let agents run such commands. To get the new skill text in a repo that is already set up, run `rex init --force`.

### Fixed

- `rex` records no run for a stop event that Claude Code sends for a call of its own, such as the reply it suggests after a turn. Each of these showed in `rex watch`, `rex status` and `rex job show` as a finished run with no role, no model and no tokens.

## 0.1.0 - 2026-11-10

The first version. SuperRex supersedes rex-skills (Rex 2.0).

### Added

- The `rex` binary, with the commands `init`, `job` (`create`, `plan`, `close`, `show`), `hook`, `status`, `oms`, `skills` (`list`, `check`), `recall`, `jobs`, `watch`, `feedback`, `codebase`, `smells` and `commands`.
- `rex init`, which sets a repo up in one run that is safe to repeat: the skills and agents, the Claude Code sandbox settings and hooks, the store in `.rex/`, the rex sections of `AGENTS.md`, the block in `.gitignore`, and a pre-commit config.
- 28 skills: the builder `orchestration-builder` with its three stages, the delivery skills `rex-deliver`, `rex-pr-review` and `rex-feedback`, 18 `rex-code-*` rulebooks, `rex-grill-me`, `rex-mermaid-diagrams` and `rex-skill-language`.
- The grid of 20 agents, one for each pair of model tier and effort level, and `rex oms`, the fixed function that picks one from a role's ratings.
- A local record of each job in `.rex/rex.db`: the request, the plan, the roles, the sub-agent runs with their tokens, the deliveries and the feedback.
- Telemetry through three Claude Code hooks, with tokens summed from the transcripts.
- `rex watch`, a live view of jobs and their sub-agent runs.
- Lessons: `rex feedback` stores the cause and the lesson of a job that missed, and `rex recall` gives them to later jobs that are similar.

### Changed

- The installer of rex-skills, `rex skills`, is now `rex init`. `rex skills` now lists and checks skills.
