# Stage 5 — Lesson

Words + rules: `.claude/skills/rex-feedback/SKILL.md`. Causes come from Stage 4 (`.claude/skills/rex-feedback/cause.md`).

`rex recall` gives lesson to builder on later similar jobs: at least 2 of same work type, size, consequence, languages. That builder has no context of this job.

- [ ] Each `rejected` analysis gives at least one cause AND one lesson. A cause needs a lesson. A lesson needs a cause.
- [ ] 1 or 2 full sentences. Max 600 characters. For a reader with no context.
- [ ] General: no name of this job, file, or person. Says kind of task + what to do.
- [ ] IF cause = `missing-context`, THEN lesson tells agents to ask for that context. Form: "When you get <kind of task>, ask the user for <context> before you start."

Example (`missing-context`): "When you get a request to change a data import, ask the user for a real input file. Ask before you start."
