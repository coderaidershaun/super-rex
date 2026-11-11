# Stopped or blocked job

LATER. Needs `.claude/skills/rex-deliver/SKILL.md`: read it first.

Job stops without END: work failed, user cancelled, OR requester declined to raise spend limit. Spend limit refusal itself does not close job: step 7, item 13 of `.claude/skills/orchestration-builder/run.md`.
- [ ] END does not run. Merge nothing to main. Remove nothing.
- [ ] Work merged in job branch is not lost: it stays on job branch. Each unit branch that is not merged stays also.
- [ ] IF repo has remote, THEN push each job branch: `git -C "$JOB_DIR" push origin "$BRANCH"`.
- [ ] Give no delivery object. `rex job close` gets no deliveries file.
- [ ] Status: declined spend limit = `--status blocked`. Work failed = `failed`. User stopped = `cancelled`.
- [ ] Builder names each kept job branch in its `--outcome` line. Declined spend limit: `--outcome` says so also.
- [ ] New agent work after close = new job. It starts from kept job branch.

Blocked repo that a person merged later. A later session does this for that repo. It also reads End 3 in `.claude/skills/rex-deliver/clean-up.md` and `## Result` in `.claude/skills/rex-deliver/end.md`:
1. Read values again from `JOB` (`## Inputs`). Read: `gh pr view "$PR" --repo "$REPO" --json state,headRefOid,mergeCommit --jq '.state, .headRefOid, .mergeCommit.oid'`.
2. IF state ≠ `MERGED`, THEN stop. Delivery stays `blocked`. Change nothing.
3. ELSE `PR_HEAD` = `headRefOid`. Run End 3 for that repo, same guard. Each worktree of the job is from an earlier session: do not try to remove it. Obey End 3 step 2 for a kept worktree.
4. Give a new delivery object: outcome `merged`, `merge_commit` = `mergeCommit.oid`, `note` as `## Result` says.
5. Later session runs `rex job close` call of `## Result` on blocked job, with an array of that one object. Rows of other repos stay.
   - IF a row of another repo stays `blocked`, THEN `--status blocked`. ELSE `--status delivered`.
