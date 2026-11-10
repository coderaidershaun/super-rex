# More than one repo

Needs `.claude/skills/rex-deliver/SKILL.md`. Start is in `.claude/skills/rex-deliver/start.md`. End 1 and `## Result` are in `.claude/skills/rex-deliver/end.md`.

- [ ] Run Start to Result once for each repo the job changes. Own job branch, pull request, outcome, delivery object.
- [ ] Same `JOB` + same `SLUG` in every repo. `MERGE` per repo: job answer, unless request gave a different answer for that repo.
- [ ] Sibling repo can have no ignore line. Ignore step of `## Start — mode + job branch` step 6 applies there.
- [ ] One repo `blocked` does not stop delivery of other repos. Repo with empty job branch gives no delivery object.
