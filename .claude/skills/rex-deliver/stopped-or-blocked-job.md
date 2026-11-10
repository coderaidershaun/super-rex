# Stopped or blocked job

LATER. Needs `.claude/skills/rex-deliver/SKILL.md`: read it first.

Job stops without END (`failed` or `cancelled`, builder decides):
- [ ] END does not run. Merge nothing. Remove nothing.
- [ ] Give no delivery object.
- [ ] Builder names each kept job branch in its `--outcome` line.

Blocked repo that a person merged later. A later session does this for that repo. It also reads End 3 in `.claude/skills/rex-deliver/clean-up.md` and `## Result` in `.claude/skills/rex-deliver/end.md`:
1. Read values again from `JOB` (`## Inputs`). Read: `gh pr view "$PR" --repo "$REPO" --json state,headRefOid,mergeCommit --jq '.state, .headRefOid, .mergeCommit.oid'`.
2. IF state ≠ `MERGED`, THEN stop. Delivery stays `blocked`. Change nothing.
3. ELSE `PR_HEAD` = `headRefOid`. Run End 3 for that repo, same guard. Each worktree of the job is from an earlier session: do not try to remove it. Obey End 3 step 2 for a kept worktree.
4. Give a new delivery object: outcome `merged`, `merge_commit` = `mergeCommit.oid`, `note` as `## Result` says.
5. Later session runs `rex job close` call of `## Result` on blocked job, with an array of that one object. Rows of other repos stay.
   - IF a row of another repo stays `blocked`, THEN `--status blocked`. ELSE `--status delivered`.
