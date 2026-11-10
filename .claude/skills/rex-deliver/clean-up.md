# End 3 — clean-up

Needs `.claude/skills/rex-deliver/SKILL.md`: read it first. `## Result` is in `.claude/skills/rex-deliver/end.md`.

Only after a confirmed merge of this repo, or for an empty job branch (`PR_HEAD` = `BASE`).

CAUTION: Clean-up deletes branches and worktrees, and you cannot undo a delete. Delete only what has the job ID in its name and passes the guard. Keep everything else and report it.

Do steps in this order:
1. IF mode ≠ `local`, THEN `git -C "$REPO_DIR" fetch --prune origin` FIRST. Then each `origin/…` ref is current, and main holds squash commit.
2. Remove each worktree of the job without `--force`. IF Git refuses (uncommitted or untracked files), THEN keep that worktree and report it.
```sh
git -C "$REPO_DIR" worktree list --porcelain | sed -n "s|^worktree \(.*/\.rex/worktrees/$JOB/.*\)|\1|p"
git -C "$REPO_DIR" worktree remove "$DIR"   # once for each path above
```
   - Session that made a worktree can remove it. Do End 3 in that session.
   - Sandbox does not let a session remove a worktree that an earlier session made. Git then prints `failed to delete '.git/worktrees/…': Operation not permitted`. A new session does not help.
   - IF a worktree is from an earlier session, OR Git prints that error, THEN keep that worktree + its branch. Do not try again.
   - CAUTION: Do not use `--force`, `rm`, or `git worktree prune` on that worktree. Do not rename or delete a file below `.git/worktrees/`. Do not run `git branch -D` on its branch.
   - CAUTION: Do not start an agent or a command outside the sandbox for this removal. Do not propose a change of settings.
   - Go on with the other worktrees and branches. Outcome stays `merged`. A kept worktree is not a reason for `blocked`.
   - Put in `note`, in one line, with real values: "Worktree <path> and branch <branch> are kept. A session cannot remove a worktree that an earlier session made. A person runs these commands in their own shell: <command 1>, then <command 2>."
   - Commands for that person, outside each session. For `<PR_HEAD>` write full hash of pull request head (`headRefOid`). Never write `merge_commit` there. A squash commit has one parent, so branch tip is not its ancestor.
```sh
git -C <REPO_DIR> worktree remove <path>   # IF Git says that path is gone: git -C <REPO_DIR> worktree prune
git -C <REPO_DIR> merge-base --is-ancestor <branch> <PR_HEAD: headRefOid, never merge_commit> && git -C <REPO_DIR> branch -D <branch>
```
3. IF job worked in main checkout, THEN `git -C "$REPO_DIR" switch "$MAIN"`. IF mode ≠ `local`, THEN also `git -C "$REPO_DIR" merge --ff-only "origin/$MAIN"`.
4. Delete a local branch of the job only IF its tip is an ancestor of `PR_HEAD`. A branch with a later commit holds work that is not on main. Keep it and report it. Leave out each branch of a kept worktree. Git sees that branch as checked out, and `git branch -D` fails.
```sh
for B in $(git -C "$REPO_DIR" branch --list --format='%(refname:short)' "*-$JOB" "*-$JOB--*"); do
  if git -C "$REPO_DIR" merge-base --is-ancestor "$B" "$PR_HEAD"; then git -C "$REPO_DIR" branch -D "$B"; else echo "KEEP $B"; fi
done
```
   - IF `git branch -D` prints `could not lock config file`, THEN make sure: IF `git -C "$REPO_DIR" rev-parse --verify --quiet "refs/heads/$B"` fails, THEN branch is deleted.
   - IF Git refuses because branch is checked out in a kept worktree, THEN keep branch + report it.
5. Delete a remote branch of the job that is still in `origin/` only IF it passes the same guard. ELSE keep it and report it.
```sh
for B in $(git -C "$REPO_DIR" branch -r --list --format='%(refname:lstrip=3)' "origin/*-$JOB" "origin/*-$JOB--*"); do
  if git -C "$REPO_DIR" merge-base --is-ancestor "origin/$B" "$PR_HEAD"; then git -C "$REPO_DIR" push origin --delete "$B"; else echo "KEEP origin/$B"; fi
done
```
   - IF push prints `could not lock config file`, THEN `git -C "$REPO_DIR" ls-remote origin "refs/heads/$B"` prints nothing = deleted.
6. `rmdir "$REPO_DIR/.rex/worktrees/$JOB"`. IF not empty, THEN leave it. Never `rm -r`.
- [ ] Never delete: main. Each branch with no job ID in its name. Each branch or worktree that fails guard or holds uncommitted work. Each branch of a repo with no merge.
- [ ] Never delete: changes, stashes, tags of user. `.rex/`, `.claude/`. The pull request.
