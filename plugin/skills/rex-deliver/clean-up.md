# End 3 — clean-up

Needs `rex-deliver/SKILL.md`: read it first. `## Result` is in `rex-deliver/end.md`.

Only after `rex_merge` gave `merged=yes` for this repo, or for an empty job branch (`WORK_TIP` = `START_COMMIT`).

CAUTION: Clean-up deletes branches and worktrees, and you cannot undo a delete. Delete only what has the job ID in its name and passes the guard. Keep everything else and report it.

Do steps in this order:
1. IF mode ≠ `local`, THEN `git -C "$REPO_DIR" fetch --prune origin` FIRST. Then each `origin/…` ref is current, and main holds squash commit.
2. Remove each worktree of the job: job worktree `${REPO_DIR}.worktrees/$JOB` and each unit worktree `${REPO_DIR}.worktrees/$JOB--<unit>`. `rex worktree remove` removes without `--force`, then unlists the worktree. IF Git refuses (uncommitted or untracked files), THEN keep that worktree and report it.
```sh
git -C "$REPO_DIR" worktree list --porcelain | sed -n 's|^worktree ||p' | grep -E "\.worktrees/$JOB(--[^/]*)?$"
rex worktree remove "$REPO_DIR" "$DIR"   # once for each path above
```
   - IF it prints `Operation not permitted` (after `rex init --strict`, `rex worktree` runs in the sandbox), THEN keep that worktree + its branch. Do not try again.
   - CAUTION: Do not use `--force`, `rm`, or `git worktree prune` on that worktree. Do not rename or delete a file below `.git/worktrees/`. Do not run `git branch -D` on its branch.
   - CAUTION: Do not start an agent or a command outside the sandbox for this removal. Do not propose a change of settings.
   - Go on with the other worktrees and branches. Outcome stays `merged`. A kept worktree is not a reason for `blocked`.
   - Put in `note`, in one line, with real values: "Worktree <path> and branch <branch> are kept. A person runs these commands in their own shell: <command 1>, then <command 2>."
   - Commands for that person, outside each session. For `<WORK_TIP>` write full hash of `WORK_TIP`.
```sh
rex worktree remove <REPO_DIR> <path>
git -C <REPO_DIR> merge-base --is-ancestor <branch> <WORK_TIP> && git -C <REPO_DIR> branch -D <branch>
```
3. Bring local main up to `origin/$MAIN` (mode ≠ `local`). IF `git -C "$REPO_DIR" branch --show-current` prints `$MAIN`, THEN `git -C "$REPO_DIR" merge --ff-only "origin/$MAIN"`. ELSE `git -C "$REPO_DIR" fetch origin "$MAIN:$MAIN"`. IF Git refuses, THEN keep local main as it is and report it.
4. Delete a local branch of the job only IF it passes guard. A branch with a later commit holds work that is not on main. Keep it and report it. Leave out each branch of a kept worktree. Git sees that branch as checked out, and `git branch -D` fails.
```sh
for B in $(git -C "$REPO_DIR" branch --list --format='%(refname:short)' "rex/$JOB" "rex/$JOB--*"); do
  if git -C "$REPO_DIR" merge-base --is-ancestor "$B" "$WORK_TIP" || git -C "$REPO_DIR" merge-base --is-ancestor "$B" "origin/$MAIN"; then git -C "$REPO_DIR" branch -D "$B"; else echo "KEEP $B"; fi
done
```
   - IF `git branch -D` prints `could not lock config file`, THEN make sure: IF `git -C "$REPO_DIR" rev-parse --verify --quiet "refs/heads/$B"` fails, THEN branch is deleted.
   - IF Git refuses because branch is checked out in a kept worktree, THEN keep branch + report it.
5. Delete a remote branch of the job that is still in `origin/` only IF it passes the same guard. ELSE keep it and report it.
```sh
for B in $(git -C "$REPO_DIR" branch -r --list --format='%(refname:lstrip=3)' "origin/rex/$JOB" "origin/rex/$JOB--*"); do
  if git -C "$REPO_DIR" merge-base --is-ancestor "origin/$B" "$WORK_TIP" || git -C "$REPO_DIR" merge-base --is-ancestor "origin/$B" "origin/$MAIN"; then git -C "$REPO_DIR" push origin --delete "$B"; else echo "KEEP origin/$B"; fi
done
```
   - IF push prints `could not lock config file`, THEN `git -C "$REPO_DIR" ls-remote origin "refs/heads/$B"` prints nothing = deleted.
- [ ] Never delete: main. Each branch with no job ID in its name. Each branch or worktree that fails guard or holds uncommitted work. Each branch of a repo with no merge.
- [ ] Never delete: changes, stashes, tags of user. `.rex/`, `.claude/`. The pull request.
- [ ] Master started you: master checks after close that no branch and no worktree holds job ID. Each kept item goes in `note`.
