# Stage 1 — Skills

Settle which skills this job uses. Start from assignment table. Ask for new skill only IF test of step 3 passes. Default result: no new skill.

```
1. Run `rex skills list`.
2. Read assignment table. Add skills of each row that fits job. Find other needs. IF skill in list covers need, THEN add that skill to skill list.
3. Test each need that no skill covers. IF test fails, THEN write no skill.
4. IF test passes, THEN put need under NEW SKILLS. Builder has session agent write it.
5. Builder does skill check on each new skill.
6. Give SKILL LIST + NEW SKILLS to builder.
```

| File | Read when |
| --- | --- |
| `.claude/skills/orchestration-builder/stages/1-skills/assign.md` | Step 2. Assignment table. Also stages 2 + 3 and challenger. |
| `.claude/skills/orchestration-builder/stages/1-skills/write-skill.md` | Builder: decision block lists NEW SKILLS. Steps 4 + 5. |

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
- Assignment table: `.claude/skills/orchestration-builder/stages/1-skills/assign.md`. One home of which skills each kind of work gets.
- Session agent: `general-purpose` sub-agent that you start with no model. It runs at model + effort of session.
- Skill library: Folder `.rex/skills/` in `HOME_DIR`. Skills that agents read by path. Claude Code does not list them. Skill tool cannot load them.
- `HOME_DIR`: Absolute path of main checkout root of home repo. Home repo = repo that holds store (`.rex/`) and skills (`.claude/skills/`, `.rex/skills/`) of session. IF your shell is in Git worktree of home repo, THEN `HOME_DIR` = parent folder of `git rev-parse --path-format=absolute --git-common-dir`.
- Brief: Prompt that sub-agent gets.
- Paths: each path in this skill starts at `HOME_DIR`.

## Input

From builder: request, classification, recall output, job ID. Output of `rex skills list`: step 1.

## Step 1 — List skills

Run `rex skills list` in `HOME_DIR`. Git worktree can lack `.rex/skills/`: Git ignores it in most repos.
It prints one block of six lines for each skill, in three folders: `.claude/skills/` of repo, skill library, and user skill folder of Claude Code (`~/.claude/skills/`):

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

## Step 2 — Find needs

From request, classification, lessons of recall output, and look at repo: find what this job needs that changes what agents do.

- [ ] Read assignment table IN FULL. For each row that fits job: add its skills to skill list.
- [ ] For each other need: find skill in list that covers it.
- [ ] IF skill covers need, THEN add that skill to skill list. Write nothing new.
- [ ] IF no skill covers need, THEN do step 3 for it.
- [ ] For each language in classification that is not `rust`: do `## Gap skill`.
- [ ] Skill that table does not give: one-line reason in `Why`, starts `Not in table:`.
- [ ] Skill that table gives and job does not get: one `Left out` line (`## Output`).

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

IF all three are true, THEN put need under NEW SKILLS (`## Output`). Do not write skill yourself. Builder does steps 4 + 5 (`.claude/skills/orchestration-builder/stages/1-skills/write-skill.md`), and gives you path + check result of each new skill.

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
- [ ] `Path` column: `path=` line of that skill in output of `rex skills list`. New skill: path of its `SKILL.md`, OR `pending` until builder gives it.
- [ ] Default: no row is `new`.
- [ ] Below table: `Left out: <skill> — <one-line reason>`, one line for each skill that assignment table gives and job does not get. OR `Left out: none`.
- [ ] Builder records skill list with plan.

NEW SKILLS: one table, OR `none`. One row for each need that passes test of step 3. Columns, in this sequence: `Name / Need / Reason / Gap language`.

- [ ] Name: short lower-case words with hyphens. Name = folder name. No skill in list of step 1 has that name.
- [ ] Need: what skill must make agents do. Reason: why this job needs it, and why later jobs need it also.
- [ ] Gap language: language of gap skill, OR `—`.
- [ ] Gap skill name: `rex-gap-<language>`. `<language>` = `--only` name of that language in `rex codebase`. Examples: `rex-gap-go`, `rex-gap-python`, `rex-gap-cpp`. IF `rex codebase` has no `--only` name for it, THEN `<language>` = its classification value.
- [ ] Default: `none`.
