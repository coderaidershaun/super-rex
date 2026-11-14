# Stopped or blocked job

LATER. Needs `rex-deliver/SKILL.md`: read it first.

Job stops without END: work failed, user cancelled, OR requester declined to raise spend limit. Spend limit refusal itself does not close job: step 8, item 16 of `orchestration-builder/run.md`.
- [ ] END does not run. Merge nothing to main. Remove nothing.
- [ ] Work merged in job branch is not lost: it stays on job branch. Each unit branch that is not merged stays also.
- [ ] IF repo has remote, THEN push each job branch: `git -C "$JOB_DIR" push origin "$BRANCH"`.
- [ ] Give no delivery object. `rex_job_close` gets `deliveries` = `[]`.
- [ ] Status: declined spend limit = `blocked`. Work failed = `failed`. User stopped = `cancelled`.
- [ ] Builder names each kept job branch in its `outcome` line. Declined spend limit: `outcome` says so also.
- [ ] New agent work after close = new job. It starts from kept job branch.

Kept branch that a person merged later (pull request of `pr-open`, or `$BRANCH--work` of `kept-local` or `blocked`). A later session does this for that repo. It also reads End 3 in `rex-deliver/clean-up.md`:
1. Read values again from `JOB` (`## Inputs`). Mode `github`: `gh pr view "$PR" --repo "$REPO" --json state,headRefOid,mergeCommit --jq '.state, .headRefOid, .mergeCommit.oid'`.
2. IF state ≠ `MERGED`, AND head of that branch is not ancestor of `origin/$MAIN`, THEN stop. Change nothing.
3. ELSE `WORK_TIP` = `headRefOid`, or tip of kept branch. Run End 3 for that repo, same guard. Each worktree of the job is from an earlier session: do not try to remove it. Obey End 3 step 2 for a kept worktree.
4. Record stays as it is. Mod records a merge only through `rex_merge`, and only builder of an open job calls `rex_job_close`. Tell person: which branches and worktrees clean-up removed, and which it kept.
