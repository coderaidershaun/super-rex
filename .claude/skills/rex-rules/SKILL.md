---
name: rex-rules
description: >
  Router to SuperRex skill library `.rex/skills/`, which Claude Code does not
  list. Points to four rulebooks that each code task reads, and to where
  other skills are.
when_to_use: >
  Use when you write, change, design, or review code in any language, or the
  user says "write code", "review code", "which rules", "coding standards",
  "code style", "rex rules".
disable-model-invocation: false
user-invocable: true
---

# Rex Rules

Rules for code are in skill library. Claude Code does not list library, and Skill tool cannot load it. Read each file with Read tool, one file for each call.

IF brief of builder job names your skill files, THEN read those. Do not use this skill.

## Words

- Skill library: folder `.rex/skills/` in `HOME_DIR`. One folder for each skill, with its `SKILL.md`.
- `HOME_DIR`: absolute path of main checkout root of repo. IF your shell is in linked Git worktree, THEN `HOME_DIR` = parent folder of `git rev-parse --path-format=absolute --git-common-dir`. Linked worktree can lack `.rex/`: Git ignores it in most repos.
- Paths: each path in this skill starts at `HOME_DIR`.

## Procedure

1. Find `HOME_DIR`.
2. Task writes or reviews code, in any language: read `## Each task that writes code` in `.claude/skills/orchestration-builder/stages/1-skills/assign.md`. Read IN FULL `SKILL.md` of each skill in its table. Its path = `.rex/skills/<skill>/SKILL.md`.
3. Rulebook text is about Rust. IF language is not Rust, THEN apply intent of each rule in idiom of that language.
4. Run `rex skills list` in `HOME_DIR`. It prints each skill: `description`, `when_to_use`, `source`, and `path` of its `SKILL.md`.
5. Read `## By kind of work` in `assign.md` of step 2. That table is one home of which skills each kind of work gets.
6. For each row that fits your work: read its skills + sibling files. Skill path = its `path=` line. Row gives path of each sibling file.
7. IF language is not Rust, AND list has gap skill `rex-gap-<language>` of that language, THEN read it. `<language>` = `--only` name of that language in `rex codebase`, for example `rex-gap-go`.
8. IF need fits no row, THEN read skill whose `description` + `when_to_use` fit need.
9. Read sibling file when a `SKILL.md` that you read sends you there.
