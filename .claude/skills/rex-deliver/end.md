# Deliver — END

Session reads this when work is done. Each END run reads End 1 and `## Result` here. End 1 sends you to file of your case.
Needs `.claude/skills/rex-deliver/SKILL.md`: read it first. Pushes obey `## Work — commit + push` in `.claude/skills/rex-deliver/start.md`.
IF job changes more than one repo, THEN also read `.claude/skills/rex-deliver/more-than-one-repo.md`.

## End 1 — pull request

Done condition. IF one is false, THEN it is not END yet.
- [ ] All units merged into job branch. `CHECKS` pass on job branch in `JOB_DIR`.
- [ ] `git -C "$JOB_DIR" status --porcelain` prints nothing. Uncommitted work stays out of pull request and blocks worktree removal.

1. IF `git -C "$JOB_DIR" rev-list --count "$BASE..$BRANCH"` prints `0`, THEN repo has no delivery. `PR_HEAD="$BASE"`. Go to End 3: read `.claude/skills/rex-deliver/clean-up.md`. Give no delivery object.
2. IF mode ≠ `github`, THEN read `.claude/skills/rex-deliver/no-remote-or-no-gh.md` and go on there.
3. Push: `git -C "$JOB_DIR" push origin "$BRANCH"`.
4. IF `gh pr list --repo "$REPO" --head "$BRANCH" --state all` shows a pull request, THEN use it. Open no second one.
5. ELSE open one pull request. Body comes on standard input. A body file in `JOB_DIR` is untracked: an agent can commit it, and it blocks `git worktree remove`. Quote heredoc delimiter (`'EOF'`), so shell runs nothing in text. Write job ID as literal text.
```sh
gh pr create --repo "$REPO" --base "$MAIN" --head "$BRANCH" --title "$TITLE" --body-file - <<'EOF'
- Request: <the request, in one or two sentences>
- What changed: <the changes, in full sentences>
- How it was checked: <each command and its result, or "no build or test command in this repo">
- What was left out: <what this pull request does not do, or "Nothing.">

Rex-Job: <JOB>
EOF
```
6. `gh pr create` prints pull request URL. `PR` = number at end of URL.
7. IF `MERGE` = `no`, THEN go to `## No merge agreed`. ELSE go to End 2: read `.claude/skills/rex-deliver/github-merge.md`.

## No merge agreed

`MERGE` = `no`:
- [ ] Stop after End 1 (mode `github`), after push (mode `push-only`), or after last commit (mode `local`).
- [ ] Do not wait for checks. Person who merges sees them on pull request.
- [ ] Remove nothing. Outcome = `pr-open` (mode `github`), ELSE `kept-local`.
- [ ] IF job worked in main checkout, THEN main checkout stays on job branch. Note says so.
- [ ] Outcome `blocked` obeys same rules: remove nothing, main checkout stays on job branch.

## Merge main in

End 2 step 1 does this step. Each round of `.claude/skills/rex-deliver/no-remote-or-no-gh.md` step 3 does it too.

`git -C "$JOB_DIR" fetch origin`. IF `origin/$MAIN` is not ancestor of job branch, THEN `git -C "$JOB_DIR" merge --no-edit "origin/$MAIN"`. Never a rebase, never a force-push. Resolve conflicts on job branch, run `CHECKS` again, push. Nobody reviews, so tree on main must be tested tree. Then `PR_HEAD=$(git -C "$JOB_DIR" rev-parse "$BRANCH")`.

## Result

At end of END, give one delivery object per repo that has a delivery. Exactly these seven keys:
```text
{"repo": "<REPO_DIR>", "branch": "<job branch>", "pr": "<url>" or null, "merge_agreed": true or false, "merge_commit": "<full hash>" or null, "outcome": "merged" | "pr-open" | "kept-local" | "blocked", "note": "<one line>" or null}
```

| `outcome` | Meaning | Removed |
| --- | --- | --- |
| `merged` | One squash commit on main: through pull request, or local squash merge. | Each branch + worktree of job that passed guard |
| `pr-open` | No merge agreed. Pull request is open for a person. | Nothing |
| `kept-local` | No merge agreed. No pull request possible. Job branch kept. IF repo has remote, THEN branch is pushed. | Nothing |
| `blocked` | Merge agreed, but not complete. Job branch + pull request kept. | Nothing |

- [ ] A value that is not there = `null`. Never the word `none`: `rex job close` refuses the text `"none"`.
- [ ] `merge_agreed` = `true` or `false`, from `MERGE` of that repo.
- [ ] `merged`: `merge_agreed` = `true` + `merge_commit` = full hash of squash commit on main.
- [ ] `pr-open` + `kept-local`: `merge_agreed` = `false`. `pr-open` needs `pr`. `blocked`: `merge_agreed` = `true`.
- [ ] `note`: one line, what remains to do. Required for each outcome except `merged`. For `merged`: `null`, unless clean-up kept something (then: what, and why).
- [ ] Note examples: "Pull request <url> waits for a person to merge it." "Branch <b> is kept and is not merged."
- [ ] Never give `failed` or `cancelled`. Those are builder's call about work.

Give each object to builder. Builder puts objects of all repos in one JSON array. It picks `--status` + `--outcome` and runs `rex job close` with array on standard input:
- [ ] IF any delivery is `blocked`, THEN `--status blocked`. ELSE `--status delivered`.
- [ ] With no delivery: no `--deliveries-file`.

Example, two repos, one `merged`, one `blocked`:
```sh
REX_STORE="$HOME_DIR/.rex" rex job close 0g4cbf7k2q --status blocked --outcome "API merged, web client waits for approval" --deliveries-file - <<'EOF'
[{"repo": "/home/dev/code/shop", "branch": "fix/zero-quantity-0g4cbf7k2q", "pr": "https://github.com/acme/shop/pull/41", "merge_agreed": true, "merge_commit": "3f2a9c1e8b7d6a5f4e3d2c1b0a9f8e7d6c5b4a39", "outcome": "merged", "note": null},
 {"repo": "/home/dev/code/web-client", "branch": "fix/zero-quantity-0g4cbf7k2q", "pr": "https://github.com/acme/web-client/pull/17", "merge_agreed": true, "merge_commit": null, "outcome": "blocked", "note": "Main needs the approval of a person: approve and merge pull request https://github.com/acme/web-client/pull/17, or remove the approval rule."}]
EOF
```
