# Plan jobs

Session: read at step 1 of `.claude/skills/orchestration-builder/SKILL.md`, for plan job or plan execution. Uses words and rules of that file.
Decision agent: read before stage 1, for same jobs. Lines on skill list, orchestrator skill, and tasks are for you. Other lines are for session.

For person, planning has two stages. Run 1 = plan job: it writes plan file. Person reads plan. Run 2 = plan execution: person runs `/orchestration-builder Execute plan <path>`, and builder does work of plan. Each run = one job.

## Run 1 — plan job

- [ ] Work type `research`. `MERGE` = `no`. No job branch, no delivery.
- [ ] Plan file: `$HOME_DIR/.rex/plans/<JOB>/plan.md`. Roles write only plan file + scratch files.
- [ ] Skill list: row `Plan job` of assignment table (`.claude/skills/orchestration-builder/stages/1-skills/assign.md`), + skills that subject of plan needs.
- [ ] Orchestrator skill is built for planning: `.claude/skills/orchestration-builder/stages/3-orchestration/plan-jobs.md`.
- [ ] Before plan is final: adversarial review. Agent that wrote no part of plan reads `.rex/skills/rex-codex-adversarial-review/SKILL.md` IN FULL and obeys it.
- [ ] Challenger = Codex. IF Codex is not available, THEN grid agent `model-high-effort-high` or above, by complexity. That skill gives choice.
- [ ] Author changes plan for each `upheld` finding. Each `contested` finding goes into `Open questions` of plan, with both positions.
- [ ] One review for each plan. IF revision changes approach of plan, THEN review again. Max 2 reviews for each job.
- [ ] Plan is final only after review. Plan writer writes first line of plan file in revision task: `Status: final. Reviewed by <challenger>.`
- [ ] Brief of plan writer gives absolute path of plan file, plan file sections (below), rule of `Status` line, and `SKILL LIST` of decision block (names + `Path`).
- [ ] Close job: `--status delivered`, no deliveries file. `--outcome` names plan file.
- [ ] Tell requester: path of plan file, open questions, and next step: "To execute this plan, run `/orchestration-builder Execute plan <absolute path of plan file>`."

Plan file sections, in this order:

1. `Status` line.
2. Goal: request, and outcome that execution must give.
3. Decisions: each decision, its reason, and alternative that plan rejected.
4. Scope: what is in, what is out.
5. Tasks: table of stage 2 form (`.claude/skills/orchestration-builder/stages/2-work-breakdown/stage.md`): `Task / Outcome / Evidence / Depends on / Owns (files, state) / Group / Skills`. Skill names come from skill list of plan job.
6. Risks: each risk and its answer.
7. Acceptance: evidence that shows whole work complete.
8. Open questions: each with options + recommendation.
9. Review: verdict table of adversarial review.

- [ ] Plan is for agent with no context. Write Goal, Decisions, Risks in full sentences.

## Run 2 — plan execution

- [ ] Request names plan file, or job ID of plan job. IF only job ID, THEN plan file = `.rex/plans/<that job ID>/plan.md`.
- [ ] IF first line of plan file does not start with `Status: final`, THEN tell requester. Stop.
- [ ] Settle merge agreement as for each job (step 2 of `.claude/skills/orchestration-builder/SKILL.md`).
- [ ] Request text in `rex job create` stays unchanged. It names plan, so record links both jobs.
- [ ] Stage 2 starts from task table of plan. Change task only IF repo changed since plan. Name each change in report.
- [ ] Each brief gives absolute path of plan file + task row of that agent.
- [ ] Open question that blocks task: ask requester (`## Questions to user` of `.claude/skills/orchestration-builder/SKILL.md`). ELSE use recommendation of plan.
- [ ] Do not review plan again.
