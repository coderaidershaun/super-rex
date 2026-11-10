# GitHub merge

Needs `.claude/skills/rex-deliver/SKILL.md` and `.claude/skills/rex-deliver/end.md`: read them first.

## End 2 — merge

Only IF `MERGE` = `yes` AND mode = `github`.

CAUTION: A merge changes main, and it can start a deployment. Merge only when `MERGE` is `yes` for this repo.
```
DO round UNTIL pull request is merged OR 3 rounds done.
IF not merged after 3 rounds, THEN outcome = `blocked`.
```

1. Merge main in: obey `## Merge main in` in `.claude/skills/rex-deliver/end.md`.
2. Wait for checks, max 30 minutes in this round (3 rounds: max 90 minutes in all). Count checks first: `gh pr view "$PR" --repo "$REPO" --json statusCheckRollup --jq '.statusCheckRollup | length'`.
   - IF count = 0, THEN run `CHECKS` in `JOB_DIR` + count again. IF still 0, THEN pull request has no checks: local `CHECKS` = proof. IF they fail, THEN do step 4. Do not call `gh pr checks`: with no checks it exits with error that is not a failed check.
   - ELSE `DO gh pr checks "$PR" --repo "$REPO" --watch --fail-fast UNTIL exit 0 (pass) OR exit 1 (a check failed) OR 3 calls done`.
   - Give each call tool time limit 600000 ms. Stop by time limit, or exit 8 = still pending: call again. No `sleep`, no `timeout` command.
   - IF 3 calls give no exit 0 or 1, THEN checks not complete after 30 minutes: outcome = `blocked`.
3. IF all checks pass, THEN `git -C "$JOB_DIR" fetch origin` again. `--match-head-commit` guards only branch head, not main.
   - IF `git -C "$JOB_DIR" merge-base --is-ancestor "origin/$MAIN" "$PR_HEAD"` fails, THEN main moved: next round.
   - ELSE, before merge command, read `gh pr view "$PR" --repo "$REPO" --json reviewDecision --jq .reviewDecision`.
   - IF it prints `REVIEW_REQUIRED` or `CHANGES_REQUESTED`, THEN do not merge. Outcome = `blocked`. Note says what person must do (`## Protected main`).
   - ELSE squash-merge. Subject: `<TITLE> (#<pr number>)`. Then two or three sentences on what changed and why. Last line: `Rex-Job: <JOB>`, as literal text. No `--delete-branch`: deletion belongs to End 3.
```sh
gh pr merge "$PR" --repo "$REPO" --squash --match-head-commit "$PR_HEAD" --subject "$TITLE (#$PR)" --body-file - <<'EOF'
<Two or three full sentences on what changed and why.>

Rex-Job: <JOB>
EOF
```
   - IF `gh pr merge` refuses because a review is required, THEN outcome = `blocked` (`## Protected main`).
   - IF `gh pr merge` refuses because repo does not let a squash merge, THEN outcome = `blocked`.
   - IF GitHub refuses for another reason (head moved, or a ruleset needs an up-to-date branch), THEN next round.
4. IF a check fails, THEN read failed log: `gh run view <run-id> --repo "$REPO" --log-failed`. Run ID is in link that `gh pr checks` prints. IF local `CHECKS` failed (step 2, no checks), THEN read output of `CHECKS`. Correct cause with new commit on job branch. Push. Next round. Never turn a check off, weaken a test, or skip a hook.

Confirm: `gh pr view "$PR" --repo "$REPO" --json state,mergeCommit --jq '.state, .mergeCommit.oid'`. IF state = `MERGED`, THEN `merge_commit` = that hash. Go to End 3: read `.claude/skills/rex-deliver/clean-up.md`. ELSE outcome = `blocked`.

Outcome = `blocked` (pull request left open, nothing removed, note says what remains) IF:
- [ ] 3 rounds done with no merge, OR checks not complete after 30 minutes in one round.
- [ ] Conflict with main cannot be resolved.
- [ ] Main needs approval of a person (`## Protected main`), OR repo refuses squash merge.
- [ ] Pull request is not `MERGED` after merge command, for example in a merge queue. Note says so.

## Protected main

In End 2 step 3, before merge command: `gh pr view "$PR" --repo "$REPO" --json reviewDecision --jq .reviewDecision`.
- IF it prints `REVIEW_REQUIRED` or `CHANGES_REQUESTED`, THEN main needs a decision of a person. Do not merge.
- IF `gh pr merge` refuses because a review is required, THEN main needs approval of a person.
- Never go around it. Never `--admin`, never `--auto`. This rule does not rest on a deny rule in settings.
- Outcome = `blocked`. Keep pull request, job branch, and worktrees.
- Approval note: "Main needs the approval of a person: approve and merge pull request <url>, or remove the approval rule."
- Changes note: "A person asked for changes on pull request <url>. Make them in a new job, or approve and merge."
