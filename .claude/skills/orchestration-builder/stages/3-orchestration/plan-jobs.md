# Plan jobs

Stage 3 for plan job. Uses words of `.claude/skills/orchestration-builder/stages/3-orchestration/stage.md`.

Plan job: deliverable = plan file, not code. `.claude/skills/orchestration-builder/plan-jobs.md` defines it. Build its orchestrator skill for good planning, not for writing code.

Order of roles:

1. Research, IF plan depends on facts that nobody knows yet: readers in parallel, one question each. Evidence: sources, code that they read, or spike in scratch folder (`rex-code-spike`).
2. Plan writer: one role writes whole plan file. One author keeps plan coherent.
3. Plan reviewer: role `plan-reviewer`. Never plan writer. Obeys `rex-codex-adversarial-review`.
4. Revision: plan writer changes plan from verdict table. It reads table at path of output file of `plan-reviewer`. Same agent IF its context is still useful.

- [ ] Each orchestrator skill for plan job holds steps 2, 3, 4. Step 1 is optional.
- [ ] Rate plan writer: intelligence `high`, verification `low` (plan is hard to examine). Consequence = consequence of work that plan leads to.
- [ ] Rate `plan-reviewer`: intelligence `high`, verification `low`. Its challenger = Codex, OR fallback agent from `rex oms` with ratings of its skill. Brief of `plan-reviewer` gives consequence of job + job inputs (duration, budget, autonomy) for those ratings.
- [ ] Roles of plan job change no file in Git repo. They write plan file + scratch files only.
- [ ] Reuse orchestrator skill for planning IF recall output gives one.
