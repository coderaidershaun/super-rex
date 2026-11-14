# Parallel writers

Read this file when you start parallel writers in a repo. You = agent that starts them: session or lead. Whoever fans out, fans in.
This file uses `## Words`, `## Rules` and `## Inputs` of `rex-deliver/SKILL.md`. Read that file in full first. Two of its lines apply to each step here: `$NAME` in `## Words`, and the CAUTION about the main checkout in `## Rules`.
- `JOB`: job ID. `REPO_DIR`: absolute path of repo root. `CHECKS`: commands that prove work (build, test). IF empty, THEN run no check.
- `PARENT`: your own branch. `PARENT_DIR`: absolute path where `PARENT` is checked out. A lead gets all five + `HOME_DIR` in its brief.
- Unit: one piece of work for one parallel writer. Name: 1 to 3 lower-case words joined by single hyphens, `a-z 0-9 -`. Unique in job. `work` + `review` are reserved: `$BRANCH--work` and `$JOB--review` have other owners.
- Unit branch: `$PARENT--<unit>` (two hyphens). Example: `rex/0g4cbf7k2q--parser`. Never a slash after job ID: Git cannot hold `a/b` and `a/b/c` together.
- Unit worktree: `${REPO_DIR}.worktrees/$JOB--<all text of unit branch after "$JOB--">`. Example: for repo `/code/app`, `rex/0g4cbf7k2q--parser--lexer` is in `/code/app.worktrees/0g4cbf7k2q--parser--lexer`.
- [ ] Never use Claude Code `isolation: "worktree"` for writers. Its names carry no job ID, so clean-up cannot find them. Its guard also refuses writes to main checkout.
- [ ] Never rebase. Merge only.
- [ ] Never remove a unit worktree or delete a unit branch here. Session removes them in clean-up, after merge.

1. Commit your own work first. Each unit starts from last commit of `PARENT`.
2. Before first worktree in repo: IF `git -C "$REPO_DIR" check-ignore -q .rex/rex.db` fails, THEN append `/.rex/*`: `printf '/.rex/*\n' >> "$REPO_DIR/.git/info/exclude"`.
3. For each unit, create branch + worktree from a commit hash, with no tracking:
```sh
TIP=$(git -C "$REPO_DIR" rev-parse "refs/heads/$PARENT")
UNIT_BRANCH="$PARENT--$UNIT"
UNIT_DIR="${REPO_DIR}.worktrees/$JOB--${UNIT_BRANCH#*"$JOB--"}"
rex worktree add "$REPO_DIR" "$UNIT_DIR" "$TIP" --branch "$UNIT_BRANCH"
```
   It prints `worktree=<path>`: use that path as `UNIT_DIR` from here on.
4. Write brief of each writer by template of `orchestration-builder/stages/3-orchestration/briefs.md`, with real values. First line = `Task: <task ID of that writer>`. Mod adds tags, skills of that task, tier text, and output file. Put these lines in its `Scope:` and `Limits:`:
```text
Scope: You work in the Git worktree at <UNIT_DIR>, on the branch <UNIT_BRANCH>. Your shell does not start there. Use this absolute path in every command, for example `git -C <UNIT_DIR> status` or `cd <UNIT_DIR> && <command>`. Write files only under <UNIT_DIR>.
Limits: Commit only on <UNIT_BRANCH>. Do not switch branches, rebase, amend a pushed commit, or force-push. If the repo has a remote, push with `git -C <UNIT_DIR> push origin <UNIT_BRANCH>`. Never use `git push -u`. If a push prints "could not lock config file", the push is still complete. Run git in the sandbox only, never with dangerouslyDisableSandbox. If a commit prints "index.lock': Operation not permitted", run it once more; if it fails again, report blocked with that output. Do not remove the worktree or delete the branch. Commit all of your work before you report.
```
5. After writer reports: IF `git -C "$UNIT_DIR" status --porcelain` prints text, THEN writer left uncommitted work. DO resume writer to commit it UNTIL status prints nothing OR 2 tries done. IF status still prints text, THEN report `blocked` to agent that started you, with that status output.
6. Merge each unit branch: `git -C "$PARENT_DIR" merge --no-ff --no-edit "$UNIT_BRANCH"`.
7. IF merge has conflicts, THEN resolve them in `PARENT_DIR` + commit. IF resolution needs context of writer, THEN resume that writer.
8. After last merge, run `CHECKS` in `PARENT_DIR`. Units that pass alone do not prove combined branch. IF `CHECKS` fail, THEN combined branch is not done. DO correct cause with new commit in `PARENT_DIR` + run `CHECKS` again UNTIL `CHECKS` pass OR 3 rounds done. IF `CHECKS` still fail, THEN report `blocked` to agent that started you, with failed `CHECKS` output.
9. IF repo has remote, THEN `git -C "$PARENT_DIR" push origin "$PARENT"`.
