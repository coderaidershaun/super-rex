# Open job exists

Uses words and rules of `.claude/skills/orchestration-builder/SKILL.md`.

Sign: `rex job create` prints `existing=yes`. Session has an open job already.

1. Run `REX_STORE="$HOME_DIR/.rex" rex job show <job-id>`. Job ID = `job=` of `rex job create`.
2. IF request continues that job, THEN continue at first step not done. Take from output: `duration`, `budget`, `autonomy` (`none` = not stated), `merge_agreed`, `experience_recalled`. `experience_recalled` = output of `rex recall` as one JSON object: same facts as lines of `## Recall output` in `.claude/skills/orchestration-builder/decide.md`. Its `orchestrators` and `unused` entries = `orchestrator` lines and `unused` lines.
3. IF that job is from earlier request that will not continue, THEN close it. Then do step 3 again.

```sh
REX_STORE="$HOME_DIR/.rex" rex job close <job-id> --status cancelled --outcome "<why>"
```

- [ ] IF work of that job failed, THEN use `--status failed` in place of `cancelled`.
- [ ] `--outcome` also names each job branch of that job that stays (step 9 in `.claude/skills/orchestration-builder/run.md`).
- [ ] `blocked` job is not open job.
