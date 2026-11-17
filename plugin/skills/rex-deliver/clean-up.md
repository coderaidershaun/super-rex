# End 3 — clean-up

Needs `rex-deliver/SKILL.md`: read it first. `## Result` is in `rex-deliver/end.md`.

Only after `rex_merge` gave `merged=yes` for this repo, or for an empty job branch (`WORK_TIP` = `START_COMMIT`).

## In a job: mod cleans up at close

Session of an open job runs no worktree or branch command for job repo. `rex_job_close` does clean-up after it records close.

1. Mode `local`: skip, `rex_merge` already moved local main. Else bring local main up to `origin/$MAIN`. IF `git -C "$REPO_DIR" branch --show-current` prints `$MAIN`, THEN `git -C "$REPO_DIR" merge --ff-only "origin/$MAIN"`. ELSE `git -C "$REPO_DIR" fetch origin "$MAIN:$MAIN"`. IF Git refuses, THEN keep local main as it is and report it.
2. Go to `## Result` of `rex-deliver/end.md`. Builder calls `rex_job_close`.
3. Mod removes each worktree of job that holds no uncommitted or untracked file. Mod deletes each branch `rex/$JOB`, `rex/$JOB--*` that passes guard or holds no commit of its own: local, and on `origin` in mode ≠ `local`. Mod keeps everything else.
4. Read each clean-up line of `rex_job_close`:

| Line | Meaning |
| --- | --- |
| `removed.worktree=<path>`, `removed.branch=<name>`, `removed.remote_branch=<name>` | Gone. |
| `kept.worktree=<path>: <why>`, `kept.branch=<name>: <why>`, `kept.remote_branch=<name>: <why>` | Stays, for that reason. |

- [ ] Tell requester each `kept.` line in one line: what stays, and why. Master started you: put each one in report.
- [ ] A kept item is not a reason for `blocked`. Outcome stays as it is.
- [ ] CAUTION: Do not remove a kept item yourself. Do not use `rm`, `--force`, `git worktree prune`, or `git branch -D` on it. It can hold work that is on no other branch.
- [ ] Master started you: master checks after close that no branch and no worktree holds job ID.

## LATER: session outside a job

`rex-deliver/stopped-or-blocked-job.md` sends a later session here for a kept branch that a person merged. Mod runs no clean-up there. Do these steps in this order.

CAUTION: Clean-up deletes branches and worktrees, and you cannot undo a delete. Delete only what has the job ID in its name and passes the guard. Keep everything else and report it.

1. IF mode ≠ `local`, THEN `git -C "$REPO_DIR" fetch --prune origin` FIRST. Then each `origin/…` ref is current.
2. Bring local main up: step 1 of `## In a job: mod cleans up at close`.
3. Delete a local branch of the job only IF it passes guard. Keep each other one and report it. IF Git refuses because branch is checked out in a worktree, THEN keep branch + report it.
```sh
for B in $(git -C "$REPO_DIR" branch --list --format='%(refname:short)' "rex/$JOB" "rex/$JOB--*"); do
  if git -C "$REPO_DIR" merge-base --is-ancestor "$B" "$WORK_TIP" || git -C "$REPO_DIR" merge-base --is-ancestor "$B" "$BASE_REF"; then git -C "$REPO_DIR" branch -D "$B"; else echo "KEEP $B"; fi
done
```
   - IF `git branch -D` prints `could not lock config file`, THEN make sure: IF `git -C "$REPO_DIR" rev-parse --verify --quiet "refs/heads/$B"` fails, THEN branch is deleted.
4. Mode `local`: skip. Delete a remote branch of the job that is still in `origin/` only IF it passes the same guard. ELSE keep it and report it. Push one delete for each `DELETE` line, with the branch name written out. The mod refuses a push whose target the shell fills in, such as `"$B"`.
```sh
for B in $(git -C "$REPO_DIR" branch -r --list --format='%(refname:lstrip=3)' "origin/rex/$JOB" "origin/rex/$JOB--*"); do
  if git -C "$REPO_DIR" merge-base --is-ancestor "origin/$B" "$WORK_TIP" || git -C "$REPO_DIR" merge-base --is-ancestor "origin/$B" "origin/$MAIN"; then echo "DELETE $B"; else echo "KEEP origin/$B"; fi
done
git -C "$REPO_DIR" push origin --delete rex/<job ID>   # once for each DELETE line, its branch name written out
```
   - IF push prints `could not lock config file`, THEN `git -C "$REPO_DIR" ls-remote origin "refs/heads/$B"` prints nothing = deleted.
- [ ] Worktree of the job from an earlier session: do not remove it. Tell person: it stays, and a person removes it in their own shell with `rex worktree remove <REPO_DIR> <path>`.
- [ ] Never delete: main. Each branch with no job ID in its name. Each branch or worktree that fails guard or holds uncommitted work. Each branch of a repo with no merge.
- [ ] Never delete: changes, stashes, tags of user. `.rex/`, `.claude/`. The pull request.
