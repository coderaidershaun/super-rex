# Deliver — END

Session reads this when work is done. Each END run reads End 1 and `## Result` here.
Needs `rex-deliver/SKILL.md`: read it first. Pushes obey `## Work — commit + push` in `rex-deliver/start.md`.
IF job changes more than one repo, THEN also read `rex-deliver/more-than-one-repo.md`.

## End 1 — done

Done condition. IF one is false, THEN it is not END yet.
- [ ] All units merged into job branch. `CHECKS` pass on job branch in `JOB_DIR`.
- [ ] `git -C "$JOB_DIR" status --porcelain` prints nothing. Uncommitted work stays out of delivery and blocks worktree removal.

1. IF `git -C "$JOB_DIR" rev-list --count "$START_COMMIT..$BRANCH"` prints `0`, THEN repo has no delivery. `WORK_TIP="$START_COMMIT"`. Go to End 3: read `rex-deliver/clean-up.md`. Give no delivery object.
2. Merge main in: obey `## Merge main in`.
3. IF `MERGE` = `no`, OR repo is not job repo, THEN go to `## No merge`.
4. IF mode = `local`, THEN `rex_merge` cannot merge: it pushes to `origin` only. Go to `## No merge`. Outcome = `kept-local`. Note: "Repo has no remote, so rex_merge cannot merge it. Branch <BRANCH> holds the work. A person merges it."
5. ELSE go to `## End 2 — squash + merge`.

## Merge main in

End 1 step 2 does this step. Each round of End 2 does it too, before squash.

Mode ≠ `local`: `git -C "$JOB_DIR" fetch origin "$MAIN"`. IF `origin/$MAIN` is not ancestor of job branch, THEN `git -C "$JOB_DIR" merge --no-edit "origin/$MAIN"`. Mode `local`: merge `"$MAIN"`. Never a rebase of a pushed commit, never a force-push. Resolve conflicts on job branch, run `CHECKS` again, push. Nobody reviews, so tree on main must be tested tree.

## No merge

`MERGE` = `no`, or no merge is possible:
- [ ] IF End 2 made a squash commit, THEN `$BRANCH--work` is branch of this delivery: push it, use it as `--head`, and give it in `branch`. Squash commit stays local: a push of it to `$BRANCH` needs a force-push.
1. Mode `github`: push job branch. IF `gh pr list --repo "$REPO" --head "$BRANCH" --state all` shows a pull request, THEN use it. ELSE open one. Body comes on standard input. A body file in `JOB_DIR` is untracked: an agent can commit it, and it blocks `git worktree remove`. Quote heredoc delimiter (`'EOF'`), so shell runs nothing in text. Write job ID as literal text.
```sh
gh pr create --repo "$REPO" --base "$MAIN" --head "$BRANCH" --title "$TITLE" --body-file - <<'EOF'
- Request: <the request, in one or two sentences>
- What changed: <the changes, in full sentences>
- How it was checked: <each command and its result, or "no build or test command in this repo">
- What was left out: <what this pull request does not do, or "Nothing.">

Rex-Job: <JOB>
EOF
```
2. `gh pr create` prints pull request URL. Outcome = `pr-open`, `pr` = that URL.
3. Mode `push-only`: push job branch. Outcome = `kept-local`. Mode `local`: outcome = `kept-local`.
- [ ] Do not wait for checks. Person who merges sees them on pull request.
- [ ] Remove nothing.
- [ ] Note examples: "Pull request <url> waits for a person to merge it." "Branch <b> is kept and is not merged."

## End 2 — squash + merge

Only IF `MERGE` = `yes`, repo = job repo, AND mode ≠ `local`.

CAUTION: A merge changes main, and it can start a deployment. Call `rex_merge` only when `MERGE` is `yes`.
```
DO round UNTIL rex_merge gives merged=yes OR 3 rounds done.
IF not merged after 3 rounds, THEN outcome = `blocked`.
```

1. Merge main in (`## Merge main in`). `CHECKS` pass in `JOB_DIR`.
2. Keep work: `WORK_TIP=$(git -C "$JOB_DIR" rev-parse "$BRANCH")`, then `git -C "$JOB_DIR" branch -f "$BRANCH--work" "$WORK_TIP"`.
3. Make squash commit. `git reset --soft` moves job branch to `origin/$MAIN` and keeps each file and the index as they are.

CAUTION: Run `git reset --soft` only with `-C "$JOB_DIR"`, never in the main checkout. Branch `$BRANCH--work` keeps the work.
```sh
git -C "$JOB_DIR" reset --soft "origin/$MAIN"
git -C "$JOB_DIR" commit -F - <<'EOF'
<TITLE>

<Two or three full sentences on what changed and why.>

Rex-Job: <JOB>
EOF
git -C "$JOB_DIR" diff --quiet "$WORK_TIP" "$BRANCH"   # exit 0: squash holds all work
```
   - IF commit is refused (for example a hook fails), THEN correct cause, `git add`, commit again. IF not possible, THEN outcome = `blocked`. Note: "The squash of <branch> is staged in <JOB_DIR> and is not committed. Branch <BRANCH>--work holds the work."
   - IF `diff --quiet` fails, THEN squash lost work: outcome = `blocked`. Note gives `$BRANCH--work`.
4. Call `rex_merge` with no arguments. Mod checks job, merge answer, squash, and policy, then pushes squash commit to main.

| Result | Do |
| --- | --- |
| `merged=yes`, `merge_commit`, `base`, `branch` | Keep `merge_commit`. Go to End 3: read `rex-deliver/clean-up.md`. |
| `merged=no`, `question=<id>` | A person must answer first. See below. |
| refused: `must be one squash commit over origin/<base>` | Do step 3 again on current tip. Same round. |
| refused: `origin/<base> moved` | `git -C "$JOB_DIR" fetch origin "$MAIN"`, then `git -C "$JOB_DIR" rebase "origin/$MAIN"`: squash commit is not pushed. Resolve each conflict, run `CHECKS`. Next round. |
| refused: `a merge of <repo> runs now` | Next round. |
| refused: `push to origin/<base> failed` | IF error says main is protected or needs a review, THEN go to `## No merge`. Note: "Main of <repo> takes no direct push, so rex_merge cannot merge it. A person merges branch <BRANCH>--work." ELSE fetch, rebase as above, next round. |
| refused: `has no merge answer` | Ask merge question (step 4 of `orchestration-builder/SKILL.md`). Then call again. |
| other refusal | Outcome = `blocked`. Note = refusal text. |

`merged=no` with `question=<id>`: not a refusal. Mod parked a merge question for a person, with options `Merge` and `Keep the branch`.
1. Do not call `rex_merge` again before an answer.
2. Main session: tell requester that question `<id>` waits, and that they answer it in the pane or with `/rex answer <id> Merge`. End your turn. At next turn, call `rex_status` with `job`. IF it gives `question.<id>=answered`, THEN call `rex_merge` again.
3. Master started you: end your turn with state `awaiting decision` and the question ID. When master sends you go-on text, call `rex_merge` again.
4. Answer `Keep the branch`: merge answer = `no`. Go to `## No merge`.

## Result

At end of END, give one delivery object per repo that has a delivery:
```text
{"repo": "<REPO_DIR>", "branch": "<job branch>", "pr": "<url>", "outcome": "merged" | "pr-open" | "kept-local" | "blocked", "merge_commit": "<full hash>", "note": "<one line>"}
```

| `outcome` | Meaning | Removed |
| --- | --- | --- |
| `merged` | `rex_merge` gave `merged=yes`: one squash commit on main. | Each branch + worktree of job that passed guard |
| `pr-open` | No merge. Pull request is open for a person. | Nothing |
| `kept-local` | No merge, and no pull request. Job branch kept. IF repo has remote, THEN branch is pushed. | Nothing |
| `blocked` | Merge agreed, but not complete. Job branch, `$BRANCH--work`, and pull request kept. | Nothing |

- [ ] Leave out a key that has no value. Never the word `none`.
- [ ] `merged` needs `merge_commit` of `rex_merge`. Mod refuses `merged` for a merge that `rex_merge` did not do.
- [ ] `pr-open` needs `pr`.
- [ ] `note`: one line, what remains to do. Required for each outcome except `merged`. For `merged`: leave out, unless clean-up kept something (then: what, and why).
- [ ] Never give `failed` or `cancelled`. Those are builder's call about work.

Give each object to builder. Builder puts objects of all repos in one list, picks `status` + `outcome`, and calls `rex_job_close`:
- [ ] IF any delivery is `blocked`, THEN `status` = `blocked`. ELSE `status` = `delivered`.
- [ ] With no delivery: `deliveries` = `[]`.

Example, two repos, job repo `merged`, sibling repo `pr-open`:
```text
status: delivered
outcome: API merged, web client waits for a person to merge it
deliveries: [{"repo": "/home/dev/code/shop", "branch": "rex/0g4cbf7k2q", "outcome": "merged", "merge_commit": "3f2a9c1e8b7d6a5f4e3d2c1b0a9f8e7d6c5b4a39"},
 {"repo": "/home/dev/code/web-client", "branch": "rex/0g4cbf7k2q", "pr": "https://github.com/acme/web-client/pull/17", "outcome": "pr-open", "note": "Pull request https://github.com/acme/web-client/pull/17 waits for a person to merge it."}]
```
