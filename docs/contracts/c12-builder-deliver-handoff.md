Status: DRAFT

Reviewed at gate G1 and settled by Lane 1, 10 November 2026. Not frozen yet. It waits for: C2 (2.4 the job ID form; 2.7 and C1 3.2 where `rex` finds the store; 5.1 and 5.2 the input of `rex job close`; 6.2 what `rex job show` prints; OP4 `existing=yes`; OP10 and section 7 the job status from deliveries), C1 (5.7 the delivery columns) and C4 (the tag block at the start of a brief). Every other part is stable: OP11, the branch and worktree names, the nine steps and the guarded clean-up.

# C12. Hand-off between `orchestration-builder` and `rex-deliver`

| | |
| --- | --- |
| Contract | C12 |
| Written by | Lane 10 (it also writes `rex-deliver`) |
| Read by | Lane 9 (`orchestration-builder`), Lane 2 (`rex job close`, C2), the G1 reviewer |
| Rests on | Build request §4 (rows "The one rule in every orchestrator" to "Job ID"), §5.1, §5.7 (limits), §5.8, §5.9, §5.16. Task C §1.3, §1.4, §1.6, §1.9, §1.10. Build plan section 4 (row C12) and 4.1 (points 10 and 11). Challenges 3 and 4. Checks 14 and 15. |

This contract fixes when the session reads `rex-deliver`, which values it holds for it, who names branches and worktrees, and what comes back for `rex job close`.
The builder knows when to read `rex-deliver` and which values to hold. `rex-deliver` holds every Git and GitHub step. Neither repeats the other.
Where a `rex` command is named here, its form is fixed by C2, not by this file.

## 1. Terms

| Term | Meaning |
| --- | --- |
| Session | The agent that ran `rex job create` for the job. It is the delivery owner, at any level: §5.1 lets a sub-agent be the orchestrator. |
| Home repo | The repo that holds the job's store. |
| Main | The default branch of a repo, whatever its name. Commands write `$MAIN`. |
| Job branch | The one branch of a job in one repo. It carries the finished work into the pull request. |
| Job dir | `JOB_DIR`: the absolute path where the job branch is checked out (the main checkout or the job worktree). |
| Unit | One piece of a job given to one parallel writer. |
| Unit branch | The branch one parallel writer works on, in its own worktree. |
| Mode | How a repo is delivered, fixed by `rex-deliver` at START: `github` (the real host of `origin` is `github.com` and `gh` can see the repo), `push-only` (`origin` exists, but the `github` test fails), `local` (no remote). |
| `PR_HEAD` | The one commit that is merged: the head of the pull request, or the job branch tip in a mode with no pull request. For an empty job branch it is `BASE`. |
| `BASE` | The commit hash of `origin/$MAIN` (or local main) that the job branch starts from. |

## 2. Who delivers

- One job has one delivery owner: the session.
- An agent that loads the builder in a session that already has an open job opens no second job (§5.6). It is a writer of the open job. It runs no part of `rex-deliver`, except the section "Parallel writers" when it starts parallel writers itself.
- A nested builder knows that a job is open because `rex job create` prints `existing=yes` (C2 OP4).
- IF an agent other than the session must change a repo that has no job branch, THEN it writes nothing there. It reports that repo to the agent that started it. The session runs START for that repo first.
- Agents other than the session never run START, END, the merge or the clean-up.

## 3. When the session reads `rex-deliver`

| Point | When | Who reads | What happens there |
| --- | --- | --- | --- |
| START | After `rex job plan`, before the first agent writes a file in a Git repo. Once for each repo the job will change, as soon as the session knows that repo will change. | The session | The mode is fixed. The job branch is created, and the job worktree if one is needed. |
| FAN-OUT | Each time an agent starts parallel writers in a repo. | That agent (the session or a lead). It reads the section "Parallel writers" only. | Unit branches and worktrees are named and created, each writer gets its path, the units are merged back, and the combined branch is tested again. |
| END | When the work is done: all units are merged into the job branch, the job's own checks pass on the combined branch, and `git -C "$JOB_DIR" status --porcelain` prints nothing. Once for each repo that has a job branch. Before `rex job close`. | The session | Push, pull request, merge and clean-up only if agreed, then the delivery result. |

The clean-tree test at END matters: uncommitted work is otherwise left out of the pull request, and it blocks worktree removal.

A lead that fans out gets these values in its brief from the agent that started it: `JOB`, `REPO_DIR`, its own branch, the absolute path where that branch is checked out, and `CHECKS`.

`rex-deliver` is not read when the job changes no file in a Git repo (for example a research answer). That job has no delivery, and `rex job close` gets zero deliveries. A skill that stage 1 writes before the plan lands in `.claude/skills/`. IF `git check-ignore -q <path of the new SKILL.md>` succeeds, THEN that write is not a repo change and needs no START. ELSE START for that repo comes before the write, even before the plan.

IF the job branch has no commit beyond `BASE` at END, THEN that repo has no delivery. `rex-deliver` runs its clean-up with `PR_HEAD` = `BASE`, with the same guard and the same order. This is the one removal without a merge.

**A job that stops without END** (`failed` or `cancelled`; the builder decides). END does not run. Nothing is merged or removed. The builder gives no deliveries file. Its `--outcome` line names each job branch that is kept.

**A blocked repo that a person merges later.** A later session confirms state `MERGED` with `gh pr view "$PR" --json state,headRefOid`. It sets `PR_HEAD` to `headRefOid`, runs the clean-up of `rex-deliver` for that repo (same guard), and records `merged` with `rex job close` on the blocked job (C2 OP10).

## 4. What the session passes

The session reads `rex-deliver` itself, so "passes" means that it holds these values under these exact names. Both skills use the same names.

| Name | Value | Made by | Read again from |
| --- | --- | --- | --- |
| `JOB` | The job ID: 10 characters of `[0-9a-z]`, with no `-` (C2 2.4). | `rex job create` | The session, branch names |
| `WORK_TYPE` | `feature`, `fix`, `refactor`, `chore`, `research` or `other` | Builder (classification) | `rex job show` |
| `SLUG` | 2 to 5 words of `a-z 0-9`, joined by single hyphens: it matches `^[a-z0-9]+(-[a-z0-9]+){1,4}$`. At most 40 characters. Taken from the request. One slug per job, the same in every repo. | Builder | The job branch name |
| `MERGE` | `yes` or `no`: was a merge to main agreed. One value per repo: the job's answer, unless the request gave a different answer for that repo. | Builder (the one rule) | `rex job show` (the job's answer only) |
| `HOME_DIR` | Absolute path of the main checkout root of the home repo (the repo that holds the job's store). | Builder | The session's project folder |
| `REPO_DIR` | The absolute path of the main checkout root of each repo the job changes, never a linked worktree: the parent of `git rev-parse --path-format=absolute --git-common-dir`. | Builder | Recovery search below |
| `CHECKS` | The commands that prove the job's work in that repo (build, test). Used at FAN-OUT and END to test again. | Builder or orchestrator | The plan or the repo |
| `TITLE` | `<type>: <what works now>`. END only. | Session | Pull request title |
| `BODY` | END only: the request, what changed, how it was checked (commands and results), what was left out. Its last line is `Rex-Job: <JOB>`. | Session | Pull request body |

`rex-deliver` maps `WORK_TYPE` to the branch type:

| `WORK_TYPE` | `feature` | `fix` | `refactor` | `chore` | `research` | `other` |
| --- | --- | --- | --- | --- | --- | --- |
| Branch type | `feature` | `fix` | `refactor` | `chore` | `chore` | `chore` |

**The `MERGE` guard.** `rex-deliver` never decides `MERGE`. IF `MERGE` is not known for a repo, THEN it stops, and the session gets the answer from the requester. It never assumes `yes`.

**Where `rex` runs.** Every `rex` command of this hand-off runs as `REX_STORE="$HOME_DIR/.rex" rex …`. A shell that is in a sibling repo or a worktree then still finds the job's store. That variable never falls through to another place (C1 3.2, C2 2.7).

**Recovery after lost context**, starting from `JOB`. `rex job show "$JOB"` gives the work type, the job's merge answer and the request text. For repos, look for a branch of the job in the home repo and in each repo in its parent folder (the edit scope of §4, so the search has a limit); the branch name gives the slug. `gh pr list` for that branch gives the pull request. For `MERGE` in a job with more than one repo, read the request text for a per-repo answer: IF it is not clear, THEN stop and ask the requester (the guard above). `CHECKS` is read again from the plan or the repo. Then run again the parts of START that only read (mode, `MAIN`, `REPO`). They change nothing. `JOB_DIR` = the path that `git -C "$REPO_DIR" worktree list --porcelain` gives for `refs/heads/$BRANCH`. `BASE` = `git -C "$REPO_DIR" merge-base "$BRANCH" "origin/$MAIN"` (mode `local`: `"$MAIN"`). `PR_HEAD` is set again by the next round of section 7.

```sh
git -C "$REPO_DIR" branch --list "*-$JOB"
gh pr list --repo "$REPO" --head "$BRANCH" --state all
```

## 5. Who names the worktrees and branches

| Thing | Name | Named and created by |
| --- | --- | --- |
| Job branch | `<type>/<slug>-<job-id>`, for example `feature/csv-export-0g4cbf7k2q` | The session, at START, through `rex-deliver` |
| Job worktree (only if needed) | `<repo>/.rex/worktrees/<job-id>/root` | The session, at START |
| Unit branch | `<parent branch>--<unit>` (two hyphens) | The agent that starts the parallel writers |
| Unit worktree | `<repo>/.rex/worktrees/<job-id>/<all text of the unit branch after "<job-id>--">` | The same agent |
| Review checkout | `<repo>/.rex/worktrees/<job-id>/review` (detached) | `rex-pr-review`, which makes and removes it |

The builder names no branch and no worktree. It makes `SLUG` only.

Examples: the session's unit `parser` is `feature/csv-export-0g4cbf7k2q--parser` in `.rex/worktrees/0g4cbf7k2q/parser`. That lead's unit `lexer` is `feature/csv-export-0g4cbf7k2q--parser--lexer` in `.rex/worktrees/0g4cbf7k2q/parser--lexer`.
A slash cannot be used in a unit branch name: Git cannot hold `a/b` and `a/b/c` together.

**The `<unit>` rule.** 1 to 3 lower-case words joined by single hyphens, characters `a-z 0-9 -`, unique in the job. `root` and `review` are reserved.

**Where the job works** (Task C §1.3, made simpler). IF the main checkout is clean (`git status --porcelain` prints nothing) AND it is on main, THEN the job branch is checked out in the main checkout. ELSE the job works in the job worktree, and the main checkout is not touched at all.
Reason: no "branch the main checkout was on" has to be recorded, and a second job or the user's own branch always sends the job to a worktree.

**Who fans out, fans in** (Task C §1.4). The agent that starts parallel writers creates each unit branch and worktree from the current head of its own branch. It puts the absolute worktree path and the unit branch name in each writer's brief. The brief starts with the tag block of C4 (`Rex-Job: <JOB>`, `Rex-Role: <role>`). It merges each unit branch back with `git merge --no-ff --no-edit "$UNIT_BRANCH"`, never with a rebase, and resolves conflicts. After the last merge, it tests the combined branch again with `CHECKS`. The session does the last integration, on the job branch. Sequential writers get no branch: they work on the job branch, one after another.

A writer in a unit worktree is a normal sub-agent. Its shell starts in the session's folder. Its brief tells it to use the absolute worktree path in every command (`git -C <path>`, `cd <path> && …`) and to commit only on its unit branch.

**No `isolation: "worktree"`.** Parallel writers do not use Claude Code's worktree isolation. Check 14 showed that it puts the worktree in `.claude/worktrees/agent-<id>` on the branch `worktree-agent-<id>`. Those names carry no job ID, so clean-up cannot find them. Its guard also refuses `git -C <main>` and file-tool writes to the main checkout.

**Hash and `--no-track`.** Every branch is created from a commit hash, with no tracking. A remote-tracking start point makes Git write to `.git/config`, which the sandbox blocks (§5.7, check 15).

```sh
git switch -c "$BRANCH" --no-track "$BASE"
git worktree add -b "$BRANCH" "$DIR" "$BASE"
```

**Clean-up finds things by name only.** It takes local and remote branches whose name ends with `-<job-id>` or contains `-<job-id>--`, and worktrees under `.rex/worktrees/<job-id>/`. There is no register.

**Reading the job ID from a branch name** (C9 4.6, `rex-feedback`). Cut the name at the first `--`. The job ID is the text after the last `-` of what is left. This works because a slug holds no `--` and does not end with `-`, a unit name holds no `--`, and a job ID holds no `-` (C2 2.4).

**Sibling repos.** A sibling repo can have no SuperRex ignore block. Before the first worktree there: IF `git -C "$REPO_DIR" check-ignore -q .rex` fails, THEN append `.rex/` to `$REPO_DIR/.git/info/exclude`. That is a local file and is not committed.

**Removal in one place only.** Branches and worktrees are removed only in the clean-up of `rex-deliver`. Clean-up runs only after a confirmed merge of that repo, or for an empty job branch (section 3). Unit worktrees stay until then.

## 6. What comes back for `rex job close`

At the end of END, the session holds one delivery result for each repo that has a delivery. The fields are the "Delivery" record of §5.9, plus one note (§5.16: "recorded with what remains to do").

| Field | Value |
| --- | --- |
| `repo` | The absolute path of the repo root |
| `branch` | The job branch |
| `pr` | The pull request URL, or `null` |
| `merge_agreed` | `true` or `false` |
| `merge_commit` | The full hash of the squash commit on main, or `null` |
| `outcome` | `merged`, `pr-open`, `kept-local` or `blocked` |
| `note` | One line: what remains to do. Required for every outcome except `merged`. For `merged` it is `null`, unless clean-up kept something (then: what, and why). |

| `outcome` | Meaning | Removed | Example `note` |
| --- | --- | --- | --- |
| `merged` | One squash commit is on main, through the pull request or a local squash merge. | Every branch and worktree of the job that passed the guard | `null` |
| `pr-open` | No merge was agreed. The pull request is open for a person. | Nothing | Pull request <url> waits for a person to merge it. |
| `kept-local` | No merge was agreed, and no pull request is possible (no remote, or no `gh`). The job branch is kept, and pushed if there is a remote. | Nothing | Branch <b> is kept and is not merged. |
| `blocked` | A merge was agreed, but it could not be completed. The job branch and the pull request are kept. | Nothing | What the user must do. |

`pr-open` and `kept-local` mean no merge was agreed (`merge_agreed` false). `blocked` means a merge was agreed (`merge_agreed` true) and could not be completed. `pr-open` needs `pr`.

The session holds one JSON object per repo, exactly the entry of C2 5.2: `{"repo": "<absolute path>", "branch": "<job branch>", "pr": "<url>" or null, "merge_agreed": true or false, "merge_commit": "<full hash>" or null, "outcome": "merged" | "pr-open" | "kept-local" | "blocked", "note": "<one line>" or null}`. A value that is not there is `null`, never the word `none`. At close, the builder puts the objects of all repos in one JSON array and runs `rex job close "$JOB" --status <status> --outcome "<one line>" --deliveries-file -` (C2 5). With no delivery, it gives no file.

**Job status from deliveries** (C2 OP10). IF any delivery is `blocked`, THEN the job closes `blocked`. ELSE (every delivery is `merged`, `pr-open` or `kept-local`, or there is no delivery) it closes `delivered`. `rex job close` refuses `delivered` while any delivery of the job is `blocked`. Every delivery is recorded. The rest of a blocked job is recorded later by another `rex job close` on that job. That call names only the repos that changed; rows it does not name are kept.

`rex-deliver` never returns `failed` or `cancelled`. Those are the builder's call about the work.

Nothing has to be stored between START and END. Every delivery fact can be read again from Git and GitHub by job ID (section 4, recovery). So the removed command `rex job deliver` is not needed.

## 7. OP11 — settled

**Question.** How is a job delivered when checks fail or main has moved?

**Answer.** The loop below applies only when `MERGE` = `yes`. With `MERGE` = `no`, the pull request is left open at once, and the session does not wait for checks: the person who merges sees them on the pull request. The outcome is `pr-open`.

The loop has at most 3 rounds. One round is:

1. `git fetch origin`. IF `origin/$MAIN` is not an ancestor of the job branch head, THEN merge it into the job branch (never a rebase, never a force-push), resolve conflicts on the job branch, run `CHECKS` again, and push. Nobody reviews, so the tree that lands on main must be the tree that was tested. `PR_HEAD` is the job branch head after this step.
2. Wait for the pull request's checks, at most 30 minutes in this round (3 rounds: at most 90 minutes in all), in this order:
   - Count the checks first. IF the count is 0, THEN run `CHECKS` locally (that takes the time a check needs to appear), then count once more. IF it is still 0, THEN the pull request has no checks, and the local `CHECKS` are the proof. Do not call `gh pr checks` then: with no checks it exits with an error that is not a failed check.
   - ELSE call `gh pr checks` with `--watch --fail-fast` UNTIL it exits 0 (all pass) or 1 (a check failed), OR 3 calls are done. One shell call runs 10 minutes at most (tool time limit 600000 ms). A call that the time limit stops, or exit 8, means "still pending": call again. 3 calls are 30 minutes. No `sleep`, no `timeout` command.
3. IF all checks pass, THEN `git fetch origin` again. IF `origin/$MAIN` is not an ancestor of `PR_HEAD` (main moved during the wait), THEN go to the next round. ELSE, before the merge command, read `gh pr view "$PR" --repo "$REPO" --json reviewDecision --jq .reviewDecision`. IF it prints `REVIEW_REQUIRED` or `CHANGES_REQUESTED`, THEN do not merge: the outcome is `blocked`, and the note says what the person must do. ELSE squash-merge with `--match-head-commit "$PR_HEAD"`. That flag guards only the branch head, not main, which is why the fetch comes first. IF `gh pr merge` refuses because a review is required, THEN the outcome is also `blocked`. IF GitHub refuses for another reason (the head moved, or a ruleset needs an up-to-date branch), THEN go to the next round.
   The squash commit has the subject `"<TITLE> (#<pr number>)"` (mode `github`) or `"<TITLE>"` (local squash merge), then two or three sentences on what changed and why, and as its last line `Rex-Job: <JOB>`.
4. IF a check fails, THEN read the failed log, correct the cause with a new commit on the job branch, push, and go to the next round. Never turn a check off, weaken a test, or skip a hook to pass.

```sh
git merge --no-edit "origin/$MAIN"
gh pr view "$PR" --repo "$REPO" --json statusCheckRollup --jq '.statusCheckRollup | length'
gh pr checks "$PR" --repo "$REPO" --watch --fail-fast
git merge-base --is-ancestor "origin/$MAIN" "$PR_HEAD"
```

The loop ends as `blocked` (pull request left open, nothing removed, the note says what remains) when 3 rounds are done with no merge, when the checks are not complete after 30 minutes in one round, when a conflict with main cannot be resolved, when main needs a person's approval (never `gh pr merge --admin`, never `--auto`), when the repo does not allow a squash merge, or when the pull request is not `MERGED` after the merge command (for example a merge queue; the note says so).
This rule is C12's own. It does not rest on a deny rule in the settings (C6).

**Modes with no pull request** (`push-only`, `local`). No remote checks exist. Step 1 still applies when there is a remote. `CHECKS` run locally on the job branch are the proof. IF they fail, THEN correct and run again, at most 3 rounds, then `blocked`.

**Reasons.** Build plan 4.1 point 11 gives this shape as the default: wait within a bound, fix on the job branch, merge main in without a rebase, `blocked` past the bound. A rebase of a pushed branch needs a force-push, which §5.7 denies. 30 minutes is the default of Task C §1.9. 3 rounds bound the fix loop and the "main moved again" loop with one number. The result is `blocked`, not `failed`, because the work exists and waits on a decision (§5.8).

## 8. What the builder must do (Lane 9)

- [ ] Hold the values of section 4 under those exact names. Make `SLUG`.
- [ ] Settle `MERGE` with the requester before work starts, per repo where the request differs.
- [ ] Read START before the first write in each repo. Never start work on main.
- [ ] Tell a lead that fans out to read "Parallel writers", and give it `JOB`, `REPO_DIR`, its branch, the absolute path where that branch is checked out, and `CHECKS`.
- [ ] IF the job works in a job worktree, THEN put `JOB_DIR` in each writer's brief. The writer uses absolute paths under it, in every shell command and every file tool.
- [ ] Use no `isolation: "worktree"` for writers.
- [ ] Read END for each repo before `rex job close`.
- [ ] Pass the JSON array of section 6 to `rex job close` with `--deliveries-file`, and pick the job status as section 6 says.
- [ ] Under an already-open job, deliver nothing (section 2).

## 9. What `rex job close` must accept (Lane 2)

- [ ] 0 to n deliveries per job, each with the seven fields of section 6 and their values.
- [ ] A `note` for every outcome except `merged`.
- [ ] The job statuses `delivered` and `blocked` as section 6 sets them (C2 OP10).
- [ ] The job record keeps the merge answer, and `rex job show` prints it with the work type and the request text (section 4, recovery).

## 10. Not tested

No live check ran for this draft. Not tested: (1) what `gh` reports for a pull request with no checks; (2) `git worktree add` under `.rex/worktrees/` inside the sandbox; (3) a write to `.git/info/exclude` of a sibling repo inside the sandbox; (4) `ssh -G` inside the sandbox (the `github` mode test for an SSH host alias). Gates P2.1, P2.9 and P2.11 cover the first three.

## 11. Decisions made here that the G1 review can change

| Decision | Source it differs from, or why it is a choice |
| --- | --- |
| The job uses the main checkout only IF it is clean AND on main. ELSE a job worktree, and the main checkout is not touched. | Task C §1.3 used the main checkout when it was clean, recorded its branch, and returned it there. |
| Unit worktrees are kept until clean-up. | Task C §1.4 step 4 removed each one at once to save disk. §5.16 step 7 and §4 "Merge and clean-up" are later: with no merge, nothing is removed. |
| A repo that does not allow a squash merge gives `blocked`. | Task C §1.9 used the method the repo allows. §4 "Merge method" decides a squash merge. |
| With no merge agreed, the session does not wait for checks. | A choice: the person who merges sees the checks on the pull request. |
| At most 3 rounds and at most 30 minutes of waiting per round. | A choice. 30 minutes is the Task C §1.9 default. |
