# Deliver — START

Session reads this before first write in a repo. Needs `.claude/skills/rex-deliver/SKILL.md`: read it first.
IF job changes more than one repo, THEN also read `.claude/skills/rex-deliver/more-than-one-repo.md`.

## Start — mode + job branch

Once per repo, before first write in it. Each Git command names `REPO_DIR` with `-C`.
1. IF `git -C "$REPO_DIR" remote get-url origin` fails, THEN mode = `local`. Go to step 4.
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
   - `local`: no `origin`.
4. Find `MAIN`, `BASE`, `BRANCH`. Mode `local`: IF `git -C "$REPO_DIR" rev-parse --verify --quiet refs/heads/main` succeeds, THEN `MAIN=main`. ELSE `MAIN=master`.
```sh
MAIN=$(git -C "$REPO_DIR" ls-remote --symref origin HEAD | sed -n 's|^ref: refs/heads/\(.*\)\tHEAD$|\1|p')
git -C "$REPO_DIR" fetch origin
BASE=$(git -C "$REPO_DIR" rev-parse "origin/$MAIN")   # mode local: "$MAIN" for "origin/$MAIN"
BRANCH="<type>/$SLUG-$JOB"                            # for example feature/csv-export-0g4cbf7k2q
```
5. IF main checkout is clean AND `git -C "$REPO_DIR" branch --show-current` prints `$MAIN`, THEN job works in main checkout. `JOB_DIR="$REPO_DIR"`. After `rex init`, main checkout is clean only when user committed `AGENTS.md`, `.gitignore`, `.pre-commit-config.yaml`.
6. ELSE job works in job worktree. Do not touch main checkout at all. Before first worktree in repo: IF `git -C "$REPO_DIR" check-ignore -q .rex` fails, THEN append `.rex/`: `printf '.rex/\n' >> "$REPO_DIR/.git/info/exclude"` (local file, not committed).
```sh
git -C "$REPO_DIR" switch -c "$BRANCH" --no-track "$BASE"         # step 5
JOB_DIR="$REPO_DIR/.rex/worktrees/$JOB/root"                      # step 6
git -C "$REPO_DIR" worktree add -b "$BRANCH" "$JOB_DIR" "$BASE"   # step 6
```
   Start each branch from a commit hash. A remote-tracking start point makes Git write `.git/config`, and sandbox blocks that.
7. IF mode ≠ `local`, THEN `git -C "$JOB_DIR" push origin "$BRANCH"`.
8. IF job works in job worktree, THEN put `JOB_DIR` in brief of each writer. Writer uses absolute paths under `JOB_DIR` in each shell command and each file tool.
9. Before first write, state these values: mode, `REPO`, `MAIN`, `BASE`, `BRANCH`, `JOB_DIR`. IF a value did not come from steps 1 to 6, THEN you skipped a step: do it now. Mode decides whether job gets a pull request.

## Work — commit + push

- [ ] Agents commit freely. No message format on job branch.
- [ ] Writer commits only on its own branch: job branch, or its unit branch.
- [ ] Sequential writers get no branch. They work on job branch, one after another.
- [ ] Push names its branch, with no upstream: `git -C "$JOB_DIR" push origin "$BRANCH"`.
- [ ] IF push prints `could not lock config file .git/config`, THEN obey its rule in `## Rules` of `.claude/skills/rex-deliver/SKILL.md`.
- [ ] Push when a loss of work can hurt. Always push before End 1. Mode `local`: no push.
