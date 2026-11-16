# Stopped or blocked job

LATER. Needs `rex-deliver/SKILL.md`: read it first.

Job stops without END: work failed, user cancelled, OR requester declined to raise spend limit. Spend limit refusal itself does not close job: step 8, item 15 of `orchestration-builder/run.md`.
- [ ] END does not run. Merge nothing to main. Remove nothing yourself.
- [ ] Work merged in job branch is not lost: it stays on job branch. Each unit branch that is not merged stays also. `rex_job_close` keeps each branch with a commit that is not on main, and each worktree with uncommitted work. It removes only a clean worktree and a branch with no commit of its own.
- [ ] Tell requester each `kept.` line of `rex_job_close` in one line. Master started you: put each one in report.
- [ ] IF mode ≠ `local`, THEN push each job branch: `git -C "$JOB_DIR" push origin "$BRANCH"`.
- [ ] Give no delivery object. `rex_job_close` gets `deliveries` = `[]`.
- [ ] Status: declined spend limit = `blocked`. Work failed = `failed`. User stopped = `cancelled`.
- [ ] Builder names each kept job branch in its `outcome` line. Declined spend limit: `outcome` says so also.
- [ ] New agent work after close = new job. It starts from kept job branch. Its own worktree comes from its own `rex_job_open`.

Kept branch that a person merged later (pull request of `pr-open`, or `$BRANCH--work` of `kept-local` or `blocked`). A later session outside any job does this for that repo. It also reads `## LATER: session outside a job` of `rex-deliver/clean-up.md`:
1. Read values again from `JOB` (`## Inputs`). Mode `github`: `gh pr view "$PR" --repo "$REPO" --json state,headRefOid,mergeCommit --jq '.state, .headRefOid, .mergeCommit.oid'`.
2. IF state ≠ `MERGED`, AND head of that branch is not ancestor of `$BASE_REF`, THEN stop. Change nothing.
3. ELSE `WORK_TIP` = `headRefOid`, or tip of kept branch. Run `## LATER: session outside a job` for that repo, same guard. Each worktree of the job is from an earlier session: do not try to remove it.
4. Record stays as it is. Mod records a merge only through `rex_merge`, and only builder of an open job calls `rex_job_close`. Tell person: which branches and worktrees clean-up removed, and which it kept.
