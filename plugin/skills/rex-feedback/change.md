# Stage 6 — Change

Words + rules: `rex-feedback/SKILL.md`. Lesson comes from Stage 5 (`rex-feedback/lesson.md`).

GENERAL test: remove names of this job from lesson (files, repo, feature, people). IF lesson still tells an agent what to do on a different request of same kind, THEN lesson is general. ELSE not general.

- [ ] IF lesson is not general, THEN lesson stays in store only. Change no file.
- [ ] Prefer a general lesson to a patch for one case.
- [ ] `missing-context` and `human-error` have no home. Lesson only.

| Cause | Home |
| --- | --- |
| `agent-skill` | That skill: file that holds the rule, `SKILL.md` or a sibling file. Repo skill or fork: in `.rex/skills/<name>/`. Shipped skill: make its fork first (`## Fork`). |
| `agent-skill`, task did not get skill or sibling file that holds rule | `orchestration-builder/stages/1-skills/assign.md` (assignment table: skills + sibling files of each kind of work) |
| `agent-skill`, table gave it, brief did not name it | `orchestration-builder/stages/2-work-breakdown/stage.md` (it gives skills of each task) |
| `agent-orchestration` | Orchestrator file, or its lead skill, in `.rex/orchestrators/<name>/` |
| `agent-model-effort`, check 3 | `orchestration-builder/stages/3-orchestration/stage.md`, only IF lesson is general |
| `agent-model-effort`, checks 1, 2, 4 | No file |

CAUTION: You cannot undo an edit to a skill or an orchestrator that Git does not track. Before you change a file, read the whole file. Copy the old text of each line that you change into the report.

1. Read whole home file. IF home is a sibling file, THEN read `SKILL.md` of that skill also. Copy old text into report.
2. IF a rule caused miss, THEN remove or correct that rule. Add no counter-rule.
3. IF lesson shows a rule is not needed now, THEN remove that rule.
4. IF no rule caused miss, THEN add one rule. Shortest text. Obey `rex-skill-language`.
5. IF new rule is for one step or one case, THEN write it in file of that step or case, next to `SKILL.md`. IF no such file exists, THEN create it and add one row for it to the table in that skill's `SKILL.md` that says when to read each file. Changed `SKILL.md` stays within limits: body max 1,500 words, `description` + `when_to_use` together max 60 words.
6. IF same rule is in a different file, THEN change it there. To find it, search `.rex/skills/` and `.rex/orchestrators/` for key words of rule, and read shipped skills that name its topic with `rex_skill`. One rule, one home. Do not copy it.
7. Change file with Edit tool or Write tool. Never use a shell command: one method for each file in `.rex/skills/` and `.rex/orchestrators/`.
8. Run check on folder of each changed skill or orchestrator. Give that folder, also when only a sibling file changed. Exit 0 = pass. Exit 1 = findings on standard output, one line each.

```sh
rex skills check .rex/skills/<name>           # repo skill or fork
rex skills check .rex/orchestrators/<name>    # orchestrator or its lead skill
```

- DO correct file UNTIL check passes OR 3 rounds done. IF it still fails, THEN restore old text with Edit tool + tell user. A restored file is not a changed file.
- Orchestrator version: `rex` gives a new version by itself at next index. No agent changes a version. Write that in report.

## Fork

Feedback never edits a shipped skill: plugin files are replaced on each update. First edit of a shipped skill makes its fork.

1. Call `rex_skill` with `name` = skill. IF `source` = `plugin`, THEN skill is shipped: do steps 2 to 4. ELSE skill is repo skill or fork already: edit it in `.rex/skills/<name>/`.
2. Read `SKILL.md` and each sibling file that it names, each with `rex_skill`. Keep `hash` of each file.
3. Write each file, unchanged, to `.rex/skills/<name>/<file>` with Write tool. Then make edit in the fork only.
4. Report gives origin of fork: skill name and `hash` of each file of step 2.

- [ ] IF Git tracks `.rex/skills/<name>/` (`git check-ignore -q .rex/skills/<name>/SKILL.md` fails), THEN commit changed skill files only, on the branch that is checked out. IF that branch is main, THEN do not commit: tell user which files to commit.
