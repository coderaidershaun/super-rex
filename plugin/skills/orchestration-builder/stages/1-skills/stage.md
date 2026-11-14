# Stage 1 — Skills

Settle which skills this job uses. Mod applies assignment table: `rex_assign`. You find only needs that table does not cover. Ask for new skill only IF test of step 3 passes. Default result: no new skill.

```
1. Run `rex skills list`.
2. Read assignment table. Find needs that its rows do not cover. IF skill in list covers need, THEN add that skill to skill list.
3. Test each need that no skill covers. IF test fails, THEN write no skill.
4. IF test passes, THEN put need under NEW SKILLS. Builder has grid agent of role `skill-writer` write it.
5. Builder does skill check on each new skill.
6. After work breakdown of stage 2: call `rex_assign`. Finish SKILL LIST from its result.
7. Give SKILL LIST + NEW SKILLS + OVERRIDES to builder.
```

| File | Read when |
| --- | --- |
| `orchestration-builder/stages/1-skills/assign.md` | Step 2 + step 6. Assignment table + `rex_assign`. Also stages 2 + 3 and challenger. |
| `orchestration-builder/stages/1-skills/write-skill.md` | Builder: decision block lists NEW SKILLS. Steps 4 + 5. |

## Words

- Skill: Folder with one `SKILL.md`. Its header gives `name`, `description`, and `when_to_use`.
- Session: Claude Code conversation that loaded builder. It has one session ID.
- Builder: Agent that obeys `orchestration-builder`.
- Decision agent: Sub-agent of builder that does stages 1 to 3. Its brief gives input of each stage.
- Requester: User or agent that gave request.
- Job: One request that SuperRex handles. It has one job ID.
- Classification: Four fields of job: work type, size, consequence, languages.
- Skill list: Skills this job uses. Output of stage 1.
- Gap skill: Skill for one language. It covers what Rust gives by default and that language does not. Name: `rex-gap-<language>` (`## Output`).
- Assignment table: `orchestration-builder/stages/1-skills/assign.md`. One home of which skills each kind of work gets. Data of plugin. `rex_assign` applies it.
- Answer: `yes` or `no` of decision agent to one question of `rex_assign`, with one-line reason. Mod records it.
- Override: skill that table does not give, OR correction of answer, with reason. Row of `OVERRIDES` of decision block.
- Skill library: 24 skills of plugin `super-rex`. Claude Code does not list them. Skill tool cannot load them. Agents read them with `rex_skill`.
- Repo skill: skill in `.rex/skills/<name>/` of `HOME_DIR`: job-written, gap skill, or fork that feedback made. `rex_skill` finds it before a skill of same name in plugin.
- Skill file: file of a skill. Text names it `<skill>/<file>.md`, or `<skill>` for its `SKILL.md`. Read it with `rex_skill`: `name` = `<skill>`, `file` = `<file>.md`.
- `HOME_DIR`: Absolute path of main checkout root of home repo. Home repo = repo that holds store (`.rex/`) and repo skills (`.rex/skills/`) of job. IF your shell is in Git worktree of home repo, THEN `HOME_DIR` = parent folder of `git rev-parse --path-format=absolute --git-common-dir`.
- Brief: Prompt that sub-agent gets.
- Paths: each path that is not a skill file starts at `HOME_DIR`.

## Input

From builder: request, classification, recall output, job ID. Output of `rex skills list`: step 1.

## Step 1 — List skills

Run `rex skills list` in `HOME_DIR`. Git worktree can lack `.rex/skills/`: Git ignores it in some repos.
It prints one block of six lines for each skill, in three folders: `.claude/skills/` of repo, repo skills (`.rex/skills/`, `source=library`), and user skill folder of Claude Code (`~/.claude/skills/`). It does not list skill library of plugin: assignment table names each skill of it.

```
name=<name>
description=<what skill is for>
when_to_use=<when to load skill>
source=<repo, library, or user>
shadowed=<yes or no>
path=<absolute path of SKILL.md>
```

- [ ] Decide from these lines. Do not open each file.
- [ ] Open skill file only IF its lines are not sufficient.
- [ ] Never list skills with `ls`: it gives no `when_to_use` line and no skill of user skill folder.
- [ ] Skill of skill library: read its header with `rex_skill` (`name` only) only IF question text of `rex_assign` is not sufficient.

## Step 2 — Find needs

From request, classification, lessons of recall output, and look at repo: find what this job needs that changes what agents do.

- [ ] Read assignment table IN FULL. Never place its skills by hand: step 6 does that.
- [ ] For each need that table does not cover: find skill in list that covers it.
- [ ] IF skill covers need, THEN add that skill to skill list + one row of `OVERRIDES` for each task that needs it. Write nothing new.
- [ ] IF no skill covers need, THEN do step 3 for it.
- [ ] For each language in classification that is not `rust`: do `## Gap skill`.
- [ ] Skill that table does not give: one-line reason in `Why`, starts `Not in table:`.

Examples of need: ergonomics are very important in this repo. Repo is very large, and architecture must get attention after work. Skill in list can cover each one already. Then add nothing new.

Work can be code, research, or other work. Pre-installed rulebooks are for code. For other work, user supplies skills of that field.

## Step 3 — Test

New skill only IF all three are true:

- [ ] It changes what agents do.
- [ ] No skill in list covers it.
- [ ] It is important after this job also.

IF one is false, THEN write no skill. Content that only this job needs goes in brief.
IF in doubt, THEN write no skill. Add skill only IF it is necessary.
Never add rule to skill that exists. Only `rex-feedback` does that.

IF all three are true, THEN put need under NEW SKILLS (`## Output`). Do not write skill yourself. Builder does steps 4 + 5 (`orchestration-builder/stages/1-skills/write-skill.md`), and gives you path + check result of each new skill.

## Step 6 — Assign

Work breakdown of stage 2 has its tasks. Obey `## Call rex_assign` of `orchestration-builder/stages/1-skills/assign.md`.

- [ ] Skill list = each skill of `task.<id>.skills` + each skill of `task.<id>.open_when.` lines, once, + each skill of `OVERRIDES`.
- [ ] Skill of skill list: `Why` = its source + reason of answer that gave it.
- [ ] Row that fits task: its skills come with it. You do not leave one out. IF task must not get skill, THEN reason under `REPORT`.

## Gap skill

IF job writes code in language that is not `rust`, THEN find skill `rex-gap-<language>` of that language in list.

- [ ] IF list has it, THEN add it to skill list.
- [ ] ELSE gap skill = need. It must pass test of step 3 also.
- [ ] Content: each protection that Rust gives by default and that language does not give, as rule that gives same protection.
- [ ] Code examples: Rust (reference) beside that language.
- [ ] Go: error handling. C++: `const` correctness and memory safety, no chance of segmentation fault.

## Output

SKILL LIST: one table. Columns, in this sequence: `Skill / Path / Why this job needs it`.

- [ ] Mark each new skill in `Skill` column: `<name>` (`new`).
- [ ] `Path` column: `path=` line of that skill in output of `rex skills list`. Skill of skill library: `plugin`. New skill: path of its `SKILL.md`, OR `pending` until builder gives it.
- [ ] Default: no row is `new`.
- [ ] Skill list holds each skill that a task reads first or opens in one case. `rex_assign` divides them in `Skills` + `Open when` of each task.
- [ ] Builder records skill list with plan.

OVERRIDES: one table, OR `none`. Columns, in this sequence: `Subject / Item / Value / Reason`.

- [ ] Subject: `task:<id>`. Item: `skill` (read first, value = skill name), `open_when.<skill>` (value = case), OR `file.<skill>/<file>.md` (sibling file, value = case).
- [ ] Reason: one line. Skill that table does not give: starts `Not in table:`.
- [ ] Default: `none`.

NEW SKILLS: one table, OR `none`. One row for each need that passes test of step 3. Columns, in this sequence: `Name / Need / Reason / Gap language`.

- [ ] Name: short lower-case words with hyphens. Name = folder name. No skill in list of step 1 has that name.
- [ ] Need: what skill must make agents do. Reason: why this job needs it, and why later jobs need it also.
- [ ] Gap language: language of gap skill, OR `—`.
- [ ] Gap skill name: `rex-gap-<language>`. `<language>` = `--only` name of that language in `rex codebase` (`rex codebase --help` lists them: `ts`, not `typescript`; `js`, not `javascript`). Examples: `rex-gap-go`, `rex-gap-python`, `rex-gap-cpp`. IF `rex codebase` has no `--only` name for it, THEN `<language>` = its classification value.
- [ ] Default: `none`.
