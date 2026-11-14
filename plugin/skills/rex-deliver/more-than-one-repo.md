# More than one repo

Needs `rex-deliver/SKILL.md`. Start is in `rex-deliver/start.md`. End 1 and `## Result` are in `rex-deliver/end.md`.

- [ ] Run Start to Result once for each repo the job changes. Own job branch, pull request, outcome, delivery object.
- [ ] Job repo: `BRANCH`, `MAIN`, `JOB_DIR` from `rex_job_open`. Each other repo: same form. `BRANCH` = `rex/$JOB`. `JOB_DIR` = `${REPO_DIR}.worktrees/$JOB`. `MAIN` = `git -C "$REPO_DIR" ls-remote --symref origin HEAD | sed -n 's|^ref: refs/heads/\(.*\)\tHEAD$|\1|p'`. Mode `local`: IF `git -C "$REPO_DIR" rev-parse --verify --quiet refs/heads/main` succeeds, THEN `MAIN=main`. ELSE `MAIN=master`.
- [ ] `rex_merge` merges only job repo. Each other repo goes to `## No merge` of `rex-deliver/end.md`: pull request (mode `github`) or kept branch. Note says that a person merges it.
- [ ] Sibling repo can have no ignore line. Ignore step of `## Start — mode + job worktree` step 6 applies there.
- [ ] One repo `blocked` does not stop delivery of other repos. Repo with empty job branch gives no delivery object.
