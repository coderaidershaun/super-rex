---
name: rex-rules
description: >
  Router to the SuperRex skill library of the plugin, which Claude Code does
  not list. Points to four rulebooks that each code task reads, and to where
  other skills are.
when_to_use: >
  Use when you write, change, design, or review code in any language, or the
  user says "write code", "review code", "which rules", "coding standards",
  "code style", "rex rules".
disable-model-invocation: false
user-invocable: true
---

# Rex Rules

Rules for code are in skill library. Claude Code does not list library, and Skill tool cannot load it. Read each file with `rex_skill`, one file for each call.

IF brief of builder job has a `<skills>` block, THEN read those skills. Do not use this skill.

## Words

- Skill library: 24 skills of plugin `super-rex`. Read with `rex_skill`.
- Skill file: file of a skill. Text names it `<skill>/<file>.md`, or `<skill>` for its `SKILL.md`. `rex_skill`: `name` = `<skill>`, `file` = `<file>.md` (default `SKILL.md`).
- Repo skill: skill in `.rex/skills/<name>/` of repo. `rex_skill` finds it before a skill of same name in plugin.
- `HOME_DIR`: absolute path of main checkout root of repo. IF your shell is in linked Git worktree, THEN `HOME_DIR` = parent folder of `git rev-parse --path-format=absolute --git-common-dir`.

## Procedure

1. IF your tools hold no `mcp__super-rex__rex_skill`, THEN SuperRex mod is not loaded. Tell user to run `rex doctor`. Stop.
2. Task writes or reviews code, in any language: read `## Each task that writes code` in `orchestration-builder/stages/1-skills/assign.md`. Read IN FULL `SKILL.md` of each skill in its table.
3. Rulebook text is about Rust. IF language is not Rust, THEN apply intent of each rule in idiom of that language.
4. Run `rex skills list` in `HOME_DIR`. It prints each skill of repo and of user: `description`, `when_to_use`, `source`, and `path` of its `SKILL.md`.
5. Read `## By kind of work` in `assign.md` of step 2. That table is one home of which skills each kind of work gets.
6. For each row that fits each step of your work: read its skills + sibling files. Row gives each sibling file as `<skill>/<file>.md`.
7. For each row that fits only one case of your work: read its skills only IF work reaches that case. Same for sibling file after a case word: `Move:`, `Audit:`, `Real funds:`, `Plan for such feature:`, or other word + colon.
8. IF language is not Rust, AND gap skill `rex-gap-<language>` of that language exists, THEN read it. `<language>` = `--only` name of that language in `rex codebase` (`rex codebase --help` lists them), for example `rex-gap-go`, `rex-gap-ts`.
9. IF need fits no row, THEN read skill whose `description` + `when_to_use` fit need.
10. Read sibling file when a `SKILL.md` that you read sends you there.
11. IF you start sub-agent, THEN first read `rex-agent-prompting/SKILL.md`, then its tier file for model of that agent. Obey them in brief.

Each skill that you read stays in context, and cache gives it again on each later call. Read few.

- [ ] Skill name in a `SKILL.md` (example: list of sister skills) = map, not read order. Open that skill only IF a step above gives it.
