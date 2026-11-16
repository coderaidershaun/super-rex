# Deliver — START

Session reads this before first write in a repo. Needs `rex-deliver/SKILL.md`: read it first.
IF job changes more than one repo, THEN also read `rex-deliver/more-than-one-repo.md`.

## Start — mode + job worktree

Once per repo, before first write in it. Each Git command names `REPO_DIR` with `-C`. Job repo: `BRANCH`, `MAIN`, `JOB_DIR` come from `rex_job_open`. You name none of them.
- [ ] Job repo: mod made job worktree `JOB_DIR` on `BRANCH` from `BASE_REF` when `rex_job_open` opened job. START only reads places there. Run no `rex worktree` or `git worktree` command for job repo.
1. Job repo: IF `remote` of `rex_job_open` = `none`, THEN mode = `local`. Go to step 4. Other repo: IF `git -C "$REPO_DIR" remote get-url origin` fails, THEN mode = `local`. Go to step 4.
2. Read host + `REPO` from `origin` URL. Works with SSH host alias such as `github.com-<name>`, where `gh` cannot find repo by itself.
```sh
URL=$(git -C "$REPO_DIR" remote get-url origin)
HOST=$(printf '%s\n' "$URL" | sed -E 's#^[a-z+]+://##; s#^[^@/]*@##; s#[:/].*##')
REPO=$(printf '%s\n' "$URL" | sed -E 's#^[a-z+]+://##; s#^[^@/]*@##; s#^[^:/]*(:[0-9]+)?[:/]##; s#\.git$##')
```
3. Real host: HTTPS URL = `HOST`. SSH URL = `hostname` line of `ssh -G "$HOST"`, so alias resolves. Mode:
```sh
case "$URL" in http*) REAL_HOST=$HOST ;; *) REAL_HOST=$(ssh -G "$HOST" | awk '$1 == "hostname" {print $2}') ;; esac
```
   - `github`: `REAL_HOST` = `github.com` AND `gh repo view "$REPO" --json nameWithOwner` exits 0. Remote not on GitHub never gets pull request in unrelated GitHub repo of same name.
   - `push-only`: `origin` exists, but `github` test fails.
   - `local`: `remote=none`, or no `origin`. Mod already fixed `MAIN` + first commit. Never ask requester how to set up git, a branch, or a remote. Never rename a branch or add a remote.
4. Find `START_COMMIT`. Job repo: `START_COMMIT=$(git -C "$JOB_DIR" rev-parse "$BRANCH")` before first commit on it. Go to step 7. Other repo:
```sh
git -C "$REPO_DIR" fetch origin "$MAIN"                      # mode local: skip
START_COMMIT=$(git -C "$REPO_DIR" rev-parse "$BASE_REF")
```
5. Other repo, before its first worktree: IF `git -C "$REPO_DIR" check-ignore -q .rex/rex.db` fails, THEN append `/.rex/*`: `printf '/.rex/*\n' >> "$REPO_DIR/.git/info/exclude"` (local file, not committed). `/.rex/*`, not `.rex/`: a repo can then re-include `.rex/skills/` in `.gitignore`.
6. Other repo: make its job worktree. Job always works there. Do not touch main checkout at all.
```sh
rex worktree add <REPO_DIR> <JOB_DIR> <START_COMMIT> --branch <BRANCH>
```
   - [ ] Run `rex worktree add` as one plain command, with literal paths + hash written out. No `;`, `&&`, pipe, `$(…)`, variable assignment, or `REX_STORE=` in front. Only a plain `rex worktree` command leaves sandbox.
   `rex worktree add` takes only a full commit hash and a path in `${REPO_DIR}.worktrees/`. It lists the worktree, so sandboxed git commits there. It prints `worktree=<path>`: use that path as `JOB_DIR` from here on.
7. IF mode ≠ `local`, THEN `git -C "$JOB_DIR" push origin "$BRANCH"`.
8. Put `JOB_DIR` in brief of each writer. Writer writes files only under `JOB_DIR`, and names it in each shell command.
9. Before first write, state these values: mode, `REPO`, `MAIN`, `START_COMMIT`, `BRANCH`, `JOB_DIR`. IF a value did not come from steps 1 to 6 or from `rex_job_open`, THEN you skipped a step: do it now. Mode decides whether job gets a pull request.

## Work — commit + push

- [ ] Agents commit freely. No message format on job branch.
- [ ] Writer commits only on its own branch: job branch, or its unit branch.
- [ ] Sequential writers get no branch. They work on job branch, one after another.
- [ ] Push names its branch, with no upstream: `git -C "$JOB_DIR" push origin "$BRANCH"`.
- [ ] IF push prints `could not lock config file .git/config`, THEN obey its rule in `## Rules` of `rex-deliver/SKILL.md`.
- [ ] Push when a loss of work can hurt. Always push before End 1. Mode `local`: no push.
- [ ] Never push squash commit of End 2 to job branch on `origin`. Only `rex_merge` pushes it, to main.
