# No remote or no gh

Needs `.claude/skills/rex-deliver/SKILL.md` and `.claude/skills/rex-deliver/end.md`: read them first.

Modes `push-only` + `local`. Same steps, no pull request. `PR_HEAD` = job branch tip.
1. IF mode = `push-only`, THEN push job branch (End 1 step 3).
2. IF `MERGE` = `no`, THEN go to `## No merge agreed`. Outcome = `kept-local`.
3. `DO round UNTIL CHECKS pass OR 3 rounds done`. Round: merge main in, as in `## Merge main in`. Mode `push-only`: fetch, then merge `origin/$MAIN`. Mode `local`: merge `"$MAIN"`. Run `CHECKS` in `JOB_DIR`. IF fail, THEN correct with new commit. IF not passed after 3 rounds, THEN outcome = `blocked`.
4. `PR_HEAD=$(git -C "$JOB_DIR" rev-parse "$BRANCH")`.
5. Local squash merge needs main checkout. Never stash, reset, or clean it. IF one test below fails, THEN do not touch main checkout. Outcome = `blocked`.
   - IF main checkout is on job branch, THEN `git -C "$REPO_DIR" switch "$MAIN"`. IF Git refuses, THEN `blocked`.
   - IF `git -C "$REPO_DIR" branch --show-current` does not print `$MAIN`, THEN `blocked`.
   - IF `git -C "$REPO_DIR" diff --cached --quiet` fails (something is staged), THEN `blocked`.
   - Note: "A local squash merge needs main checked out, nothing staged, and no local change in the way."
6. IF mode = `push-only`, THEN `git -C "$REPO_DIR" fetch origin` + `git -C "$REPO_DIR" merge --ff-only "origin/$MAIN"`. IF Git refuses, THEN `blocked`. Stale local main gives a push that remote refuses.
7. IF `git -C "$REPO_DIR" merge-base --is-ancestor "$MAIN" "$PR_HEAD"` fails, THEN main moved: do step 3 again, same round count.
8. Run next commands one at a time. IF `git merge --squash` refuses because local changes are in the way, THEN `blocked`, note of step 5. Unstaged user changes in files the merge does not touch stay: `git commit` takes only staged squash result.

CAUTION: The next commands write a commit on main. Run them only when `MERGE` is `yes` for this repo.
```sh
git -C "$REPO_DIR" merge --squash "$BRANCH"
git -C "$REPO_DIR" commit -F - <<'EOF'
<TITLE>

<Two or three full sentences on what changed and why.>

Rex-Job: <JOB>
EOF
git -C "$REPO_DIR" push origin "$MAIN"   # mode push-only only
```
9. IF Git or remote refuses the commit or push, THEN outcome = `blocked`. Keep everything. Never reset main.
   - Commit refused (for example a hook fails). Note: "The squash of <branch> is staged in the main checkout and is not committed."
   - Push refused. Note: "Local main holds a commit that is not on the remote."
10. `merge_commit=$(git -C "$REPO_DIR" rev-parse "$MAIN")`. Go to End 3, same guard: read `.claude/skills/rex-deliver/clean-up.md`.
