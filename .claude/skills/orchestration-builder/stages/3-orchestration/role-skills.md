# Skills of each role

Step 3 of `.claude/skills/orchestration-builder/stages/3-orchestration/stage.md`. Uses its words.

Each skill that agent reads enters its context. Cache gives it again on each later call of that agent. So agent reads few skills first, and opens each other skill only when its work reaches that case.

- Read first: `Skills` column. Skill or sibling file that agent uses in each step of its task.
- Open when: `Open when` column. Skill or sibling file, with case that sends agent there. Form: `<skill or skill/file.md> (<case>)`. Comma between items. OR `—`.

## Rules

- [ ] Skills of role that does tasks: union of `Skills` of its tasks.
- [ ] Open when of role that does tasks: union of `Open when` of its tasks. Leave out each skill that its `Skills` give.
- [ ] Skills of lead, and of role with `—`: skills it reads for its own job, from skill list: rows of assignment table that fit its job + other skills for its job. Gap skill of each language that role writes. Divide them in `Skills` + `Open when`, same way as stage 2.
- [ ] Skills that orchestrator skill gives a role: add them to `Skills` OR `Open when` of each task of that role, and to skill list with their `Path` (`path=` line of `rex skills list`).
- [ ] Reuse: roles come from reused file. Give tasks + skills same way.
- [ ] Lead, and role with `—`: max 6 skills + sibling files in `Skills`. Code role: 4 skills of `## Each task that writes code` of assignment table + max 2.
- [ ] `skills` of role in plan file = names in `Skills` of role only. `rex job close` names each one that no run of role opened, and each skill that a run opened beyond them, with its words.

## How agent reads

Skills are files. Agent first reads, IN FULL, `SKILL.md` of each skill in `Skills` of its task, and each sibling file there (lead, or role with `—`: `Skills` of its role). It opens a skill or file of `Open when` only IF its work reaches that case. It also reads a sibling file when a `SKILL.md` sends it there.

- [ ] Skill name in a `SKILL.md` (example: list of sister skills) = map, not read order. Agent opens that skill only IF its brief lists it.
- [ ] Brief gives absolute paths. Grid agent has no Skill tool: only skills that its brief names reach it.
