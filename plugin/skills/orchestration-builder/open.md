# Open job, merge answer, recall

Steps 3 to 5 of `orchestration-builder/SKILL.md`. Uses its words and rules.

## Step 3 — Open job

Call `rex_job_open` before you start first sub-agent. Sub-agent that starts while you have no open job gets no record.

| Argument | Value |
| --- | --- |
| `request` | Request text, unchanged. |
| `type` | Work type. |
| `size` | Size. |
| `consequence` | Consequence. |
| `langs` | List of languages, for example `["rust", "ts"]`. `["none"]` IF job writes no code. |
| `prefs` | `{ duration, budget, autonomy }`: only each preference that user stated, as `low`, `medium`, or `high`. `fast` = `low`, `long` = `high`. No stated preference: leave `prefs` out. |

Result keys: `job`, `status`, `existing` (`yes` or `no`), `merge` (`yes`, `no`, or `unknown`), `storage` (`on` or `off`), `limit_usd`, `repo`, `branch`, `base`, `worktree`.

- [ ] Hold `job` as `JOB`. Each later step uses it.
- [ ] `repo` = `HOME_DIR`. `branch`, `base`, `worktree` = job places. `rex-deliver` uses them. You name no job branch and no job worktree.
- [ ] `storage=off`: mod keeps no store. `rex_recall` and `rex job show` give nothing. Job runs as normal.
- [ ] IF output gives `existing=yes`, THEN read `orchestration-builder/open-job.md`.
- [ ] IF mod refuses call, THEN give requester its reason text and stop. Main session opens job only right after a prompt `/super-rex:orchestration-builder`: typed by a person, or sent headless (origin `sdk`) while `.rex/config.json` has `jobs_from_sdk` = `true`.
- [ ] Spend limit: `limit_usd`. Request can set it: `Set the spend limit of this job to <usd> dollars`. Person raises it later with `/rex limit <job> <usd>`.

## Step 4 — Merge answer

Reason: A merge can need the approval of a person, or it can start a deployment.

1. Plan job: `MERGE` = `no`. Do not ask. Plan job writes no file in Git repo, so nothing merges.
2. IF `merge` of `rex_job_open` = `yes` or `no`, THEN `MERGE` = that value. Ask nothing.
3. IF a master started you, THEN ask nothing. `MERGE` = `yes`: End 2 of `rex-deliver/end.md` calls `rex_merge`. With no answer of policy, `rex_merge` parks a merge question for owner and gives `merged=no`, `question=<id>`. Owner answer `Keep the branch` gives `no`.
4. IF `origin` of `rex_status` (with `job`) = `sdk`, THEN job runs headless and no person can answer: `MERGE` = `no`. A headless request that wants a merge says `merge: yes` in its text: then step 2 applies.
5. ELSE ask requester with AskUserQuestion tool. One question, two options, exact text. Mod records only this form:
   - Question: `Merge this job to main when its checks pass?`
   - Options: `Yes, merge` and `No, keep the branch`
6. `Yes, merge` gives `MERGE` = `yes`. `No, keep the branch` gives `no`. IF person answers other text, THEN ask same question one more time. IF still other text, THEN `MERGE` = `no`.

- [ ] Never pass merge answer to a tool. Mod records it from person or from policy.
- [ ] Person can change answer later: prompt `merge: yes` or `merge: no`. Latest word holds.
- [ ] `MERGE` is for job repo only. `rex_merge` merges no other repo: `rex-deliver/more-than-one-repo.md`.

## Step 5 — Recall

Call `rex_recall` with `job` = `JOB`.

- [ ] Storage on: output = output of `rex recall`, unchanged. Mod records recalled experience on job, while job status = `created`.
- [ ] Storage off: `recall=off` + `reason=`. Recall output = `none`.
- [ ] Keep output unchanged. Step 6 gives it to decision agent. `orchestration-builder/decide.md` gives its form.
