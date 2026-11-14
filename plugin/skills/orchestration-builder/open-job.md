# Open job exists

Uses words and rules of `orchestration-builder/SKILL.md`.

Sign: `rex_job_open` gives `existing=yes`. You own an open job already.

IF a master started you with `rex_build`, THEN that job is your job. Its request is in your brief. Do not close it. Go on at step 4 of `orchestration-builder/SKILL.md`.

ELSE:

1. Call `rex_status` with `job` = `job` of `rex_job_open`. It gives `status`, `phase`, spend, and each open question. IF storage is on, THEN also run `REX_STORE="$HOME_DIR/.rex" rex job show <job-id>`.
2. IF request continues that job, THEN continue at first step not done. Take from output: `duration`, `budget`, `autonomy` (`none` = not stated), `merge_agreed`, `experience_recalled`. `experience_recalled` = output of `rex recall` as one JSON object: same facts as lines of `## Recall output` in `orchestration-builder/decide.md`. Its `orchestrators` and `unused` entries = `orchestrator` lines and `unused` lines.
3. IF that job is from earlier request that will not continue, THEN close it: `rex_job_close` with `status` = `cancelled` and `outcome` = why. Then do step 3 of `orchestration-builder/SKILL.md` again.

- [ ] IF work of that job failed, THEN use `status` = `failed` in place of `cancelled`.
- [ ] `outcome` also names each job branch of that job that stays (step 10 in `orchestration-builder/run.md`).
- [ ] `blocked` job is not open job.
