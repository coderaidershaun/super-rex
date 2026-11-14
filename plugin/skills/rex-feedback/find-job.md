# Stage 2 — Find job

Words + rules: `rex-feedback/SKILL.md`.

Try each way in order. DO next way UNTIL one job fits OR all 5 ways done.

1. Job ID given by user. Use it only IF it is 10 characters of `[0-9a-z]`.
2. Commit: read its `Rex-Job:` line. IF it has none (a person merged it), THEN find its branch or pull request.
3. Branch name: cut at first `--`, then take text after last `/`, then text after last `-`. `rex/<job ID>` and old `<type>/<slug>-<job ID>` both give job ID. Use result only IF it is a job ID. ELSE next way.
4. Pull request body (`PR` = URL or number user gave): last line is `Rex-Job: <job-id>`.
5. Store search by request text + date.

```sh
git log -1 --format=%B "$COMMIT" | grep '^Rex-Job:'
git branch -a --contains "$COMMIT"
gh pr list --search "$COMMIT" --state merged --json number,body
b=${BRANCH%%--*}; b=${b##*/}; JOB=${b##*-}
gh pr view "$PR" --json body --jq .body | grep '^Rex-Job:'
rex jobs --json --search "<text>" --search "<text>" --since 2026-11-01 --until 2026-11-10
```

- `--search`: repeatable. Each term must be in request text. `rex` ignores ASCII case. Dates: `YYYY-MM-DD`, UTC, both inclusive.
- `request` in output = first 200 characters of request text. IF text is longer, THEN `request_truncated` = `true`.
- IF `gh` is not there, THEN skip `gh` commands and way 4.
- IF more than one job fits, THEN ask user. Give each job ID with its request text + date. Write the question in full sentences.
- IF no job fits, THEN tell user in full sentences what you tried. Stop.
