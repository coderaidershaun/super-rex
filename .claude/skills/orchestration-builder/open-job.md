# Open job exists

Uses words and rules of `.claude/skills/orchestration-builder/SKILL.md`.

Signs:

1. Your brief starts with tag block that holds `Rex-Job:`. You are inside that job. Do not run `rex job create`.
2. `rex job create` prints `existing=yes`.

Case "inside open job": sign 1, OR sign 2 when you are sub-agent.

1. Record nothing: no plan, no START, no END, no close.
2. Run `REX_STORE="$HOME_DIR/.rex" rex job show <job-id>`. Job ID = `Rex-Job` of your brief, or `job=` of `rex job create`.
3. Take from output: `duration`, `budget`, `autonomy` (`none` = not stated), `merge_agreed`, `experience_recalled` (= recall output for decision agent).
4. Do step 1 for your part of work. Record nothing.
5. Do step 5 for your part: read `.claude/skills/orchestration-builder/decide.md`. Recall output = `experience_recalled`. Orchestrator level = your level. Do not do step 6. Then run your part with roles of role table, as step 7 of `.claude/skills/orchestration-builder/run.md` says, with no START.
6. IF you have no Agent tool, THEN start no decision agent. Do stages yourself: for each row of `### Stages` in `decide.md`, read its file IN FULL, do it, keep its output. Then do `.claude/skills/orchestration-decision-challenge/start.md`: it skips challenge.
7. Start each role with agent of `Agent` column of role table (`agent` line of `rex oms`): there is no plan.
8. Work on branch that your brief gives. Copy `Rex-Job` of your brief into tag block of each brief that you write.
9. IF you start parallel writers, THEN read `.claude/skills/rex-deliver/SKILL.md`, then `.claude/skills/rex-deliver/parallel-writers.md`.
10. IF you must change repo that has no job branch, THEN write nothing there. Report that repo to agent that started you.
11. You have fewer sub-agent levels below you. Give your level to decision agent in its brief.
12. Report result to requester.

Case "session": no `Rex-Job` in your brief, and `rex job create` prints `existing=yes`.

1. Run `REX_STORE="$HOME_DIR/.rex" rex job show <job-id>`.
2. IF request continues that job, THEN continue at first step not done.
3. IF that job is from earlier request that will not continue, THEN close it. Then do step 3 again.

```sh
REX_STORE="$HOME_DIR/.rex" rex job close <job-id> --status cancelled --outcome "<why>"
```

- [ ] IF work of that job failed, THEN use `--status failed` in place of `cancelled`.
- [ ] `--outcome` also names each job branch of that job that stays (step 9 in `.claude/skills/orchestration-builder/run.md`).
- [ ] `blocked` job is not open job.
