Status: DRAFT

# C9. Experience and views: `rex recall`, `rex feedback`, `rex jobs`, `rex watch`

This contract fixes what four `rex` commands print and accept.
Skills and other lanes can call them, or parse their output, from this file alone.
Lane 7 builds the commands. C9 is frozen only after the review of the contract drafts.

## 1. Callers

| Caller | Calls | Why |
|---|---|---|
| `orchestration-builder` (Lane 9) | `rex recall <job>` | Gets lessons and orchestrator candidates for a new job (step 4 of the build flow). |
| `rex-feedback` (Lane 10) | `rex jobs --json`, `rex job show` (C2), `rex feedback <job>` | Finds the job, reads what happened, records the result. |
| People | `rex watch`, `rex jobs` | Browse past and running jobs. |
| Lane 2 | reads the "needs" table (section 7) | Builds the columns and the transition function C9 relies on. |
| Lane 3 | reads the "needs" table (section 7) | Provides the intake, run counts, tokens and the run tree. |

## 2. Common rules

- **Exit codes.** C9 needs three: `0` ok, `1` store or state error, `2` usage error (as `rex oms`). C11 sets the final rule. If C11 differs, C11 wins and this section changes.
- **Error text.** Errors go to standard error as one line that starts with `error: `. C11 sets the final style.
- **Job argument.** `<job>` is an exact job ID. No prefix match.
- **Unknown job.** The command prints `error: no job with ID <job>` and exits 1.
- **No store.** If no store is found, the command exits 1 with the error that C1's open call gives.
- **Time.** All timestamps are ISO 8601 UTC text, such as `2026-11-10T09:14:03Z`. They are printed as stored.
- **Output style.** Plain output is `key=value` lines in a fixed order. The first line is the answer (Task A 5 style). Where a record repeats (`rex recall`), each repeated line starts with a word that names its kind. If C11 defines a list style, C9 adopts it.
- **Values.** In a `key=value` pair the value has no spaces. The last key of a line (`text=`, `description=`) takes the rest of the line.
- **Intake.** `rex jobs` and `rex watch` first take in the hook event log (C5 intake). Taking the same line twice changes nothing. `rex recall` and `rex feedback` need no intake: they read job records and feedback, not runs.
- **What each command writes.** `rex recall` writes one job field (section 3.6). `rex feedback` writes one feedback row and the job status (section 4). `rex jobs` and `rex watch` write nothing except the intake.

## 3. `rex recall <job>`

Source: Task B 4. Gives the builder the lessons of similar past jobs and the orchestrators those jobs used.

```
rex recall <job> [--json]
```

### 3.1 The score

The score of another job against the asked job is the number of these four fields that are equal: `work_type`, `size`, `consequence`, `languages`. It runs from 0 to 4. There are no weights.

- Two fields are equal only when both are non-empty and have the same text. An empty field never matches. (Task B 4 uses SQL `IS`, which makes empty equal empty. C9 does not, so a job from an older scheme version with a missing field never scores on it. This follows Task B 7, item 3.)
- `languages` is stored as sorted names joined by commas, such as `go,rust`, or `none`. It is equal only when the two lists are the same. `none` equals `none`. `none` is a value; it is not empty.
- The asked job is never compared with itself.

### 3.2 Lessons

- Candidates: feedback rows whose `lesson` is not empty, on jobs other than the asked job, whose job has score 2 or more.
- Order: score high to low, then job `created_at` new to old, then feedback `id` high to low.
- At most 5 lines (`MAX_LESSONS=5`). The score floor is `MIN_SCORE=2`.

### 3.3 Orchestrators

- Candidates: jobs other than the asked job, with score 2 or more, that have an orchestrator. Group them by orchestrator name.
- For each group:

| Key | Meaning |
|---|---|
| `best_match` | The highest score in the group. |
| `runs` | Number of jobs in the group. |
| `accepted` | Jobs whose status is `accepted`. |
| `missed` | Jobs whose status is `rejected`, `analyzed` or `failed`. |
| `other` | The rest: `created`, `planned`, `delivered`, `blocked`, `cancelled`. So `runs = accepted + missed + other`. |
| `version` | The orchestrator version on the newest job of the group (by `created_at`). |

- Order: `best_match` high to low, then `runs` high to low. At most 3 groups (`MAX_ORCHESTRATORS=3`).
- Status is the stored status. (`running` is never stored, so it never appears here.)

### 3.4 Unused orchestrators

Every orchestrator in the store's index that no job names. Each has its name, its index version and its description. Order: name, A to Z. No limit. Recall reads the store only. It never reads skill or orchestrator files. If the index description is empty, `description=` is printed empty.

### 3.5 Output

Plain output, in this order. All count lines print even when the count is 0.

| Line | Form |
|---|---|
| 1 | `job=<id>` |
| 2 | `recorded=yes` or `recorded=no` (section 3.6) |
| 3 | `lessons=<n>` |
| next n | `lesson feedback=<id> job=<id> score=<n> status=<status of that job> causes=<a,b or none> text=<rest of line>` |
| next | `orchestrators=<n>` |
| next n | `orchestrator name=<name> version=<v> best_match=<n> runs=<n> accepted=<n> missed=<n> other=<n>` |
| next | `unused=<n>` |
| next n | `unused name=<name> version=<v> description=<rest of line>` |

- `causes` are the feedback row's causes, sorted, comma-joined. `none` if the row has no cause.
- Newlines inside a lesson or description become one space each.

Worked example. Job `m4q7t2c9` is a `fix`, `medium`, `high`, `go,rust` job in status `planned`.

```
$ rex recall m4q7t2c9
job=m4q7t2c9
recorded=yes
lessons=2
lesson feedback=3 job=h2n6v0b4 score=4 status=analyzed causes=missing-context text=Ask which config formats must stay readable before changing a config file.
lesson feedback=2 job=d7s3j9x5 score=2 status=analyzed causes=agent-model-effort,agent-skill text=Give the verifier a model one tier higher when no test can check the result.
orchestrators=1
orchestrator name=lead-and-verifier version=1 best_match=4 runs=3 accepted=2 missed=1 other=0
unused=1
unused name=research-panel version=1 description=Three readers compare options and one writer recommends.
```

With no similar jobs and an empty index:

```
$ rex recall m4q7t2c9
job=m4q7t2c9
recorded=yes
lessons=0
orchestrators=0
unused=0
```

`--json` prints one line of JSON. The same example:

```
$ rex recall m4q7t2c9 --json
{"schema":1,"job":"m4q7t2c9","recorded":true,"lessons":[{"feedback":3,"job":"h2n6v0b4","score":4,"status":"analyzed","causes":["missing-context"],"text":"Ask which config formats must stay readable before changing a config file."},{"feedback":2,"job":"d7s3j9x5","score":2,"status":"analyzed","causes":["agent-model-effort","agent-skill"],"text":"Give the verifier a model one tier higher when no test can check the result."}],"orchestrators":[{"name":"lead-and-verifier","version":1,"best_match":4,"runs":3,"accepted":2,"missed":1,"other":0}],"unused":[{"name":"research-panel","version":1,"description":"Three readers compare options and one writer recommends."}]}
```

| Field | Type | Meaning | When empty |
|---|---|---|---|
| `schema` | number | Shape version. Always `1`. | never |
| `job` | string | The asked job ID. | never |
| `recorded` | bool | The experience-recalled field was written (section 3.6). | never |
| `lessons` | array | One object per lesson line. | `[]` |
| `lessons[].feedback` | number | Feedback row ID. | never |
| `lessons[].job` | string | Job the lesson came from. | never |
| `lessons[].score` | number | Score 2 to 4. | never |
| `lessons[].status` | string | Stored status of that job. | never |
| `lessons[].causes` | array of string | The row's causes. | `[]` |
| `lessons[].text` | string | The lesson, as stored (newlines kept in JSON). | never |
| `orchestrators` | array | One object per group. Keys as in section 3.3. | `[]` |
| `unused` | array | One object per unused orchestrator: `name`, `version`, `description`. | `[]` |

In JSON, `causes` for a row with no cause is `[]` (plain output prints `none`). A description that is empty is `""`. Exit code 0 in every case above.

### 3.6 The one write: experience recalled

`rex recall` writes the job field `experience_recalled` (the "experience recalled" of the job record). Rex records facts itself, so the builder does not pass recalled text on.

- It writes only while the job's stored status is `created` or `planned`. In any other status it writes nothing and prints `recorded=no`.
- A second call replaces the first. The last call wins.
- There is no flag to turn the write off. A repeat call does no harm.
- The field holds JSON text with this shape (example from the section 3.5 call):

```
{"recalled_at":"2026-11-10T11:31:40Z","lessons":[{"feedback_id":3,"job":"h2n6v0b4","score":4},{"feedback_id":2,"job":"d7s3j9x5","score":2}],"orchestrators":[{"name":"lead-and-verifier","version":1,"best_match":4}],"unused":["research-panel"]}
```

`rex job show` (C2) prints this field as stored.

## 4. `rex feedback <job>`

Sources: 5.8, 5.9, 5.18. Records what the user said about a finished job, and moves the job's status. Called by `rex-feedback`.

```
rex feedback <job> --verdict accepted|rejected
               (--said <text> | --said-file <path>)
               [--by <who>]
               [--cause <value>]...
               [--lesson <text> | --lesson-file <path>]
               [--changed-skill <name>]...
               [--changed-orchestrator <name>]...
```

### 4.1 Flags

| Flag | Required | Meaning |
|---|---|---|
| `--verdict` | yes | `accepted` or `rejected`. |
| `--said` / `--said-file` | exactly one | What the user said. The file is read as text. |
| `--by` | no | Who gave the feedback. Default `user`. |
| `--cause` | no, repeatable | A root cause. Values below. |
| `--lesson` / `--lesson-file` | no, at most one | The lesson, at most 600 characters. |
| `--changed-skill` | no, repeatable | A skill the analysis changed. |
| `--changed-orchestrator` | no, repeatable | An orchestrator the analysis changed. |

Causes. More than one can apply. The parser rejects any other value and lists the allowed ones.

| Value | Meaning |
|---|---|
| `human-error` | The requester was wrong. |
| `missing-context` | The agents needed context the requester did not give. Counts nearly as human error. |
| `agent-skill` | Agent error: wrong or missing skills for the orchestration. |
| `agent-model-effort` | Agent error: the model or effort was wrong. |
| `agent-orchestration` | Agent error: the shape of the orchestration was wrong. |

"Agent error" means any `agent-*` cause. Causes, changed skills and changed orchestrators are stored sorted, without duplicates, joined by commas. The database has no constraint on cause values; the Rust enum is the only check, so a cause can be added later without a migration.

### 4.2 Usage rules (exit 2)

Checked in this order. Each prints the line shown.

| # | Rule | Error text |
|---|---|---|
| 1 | `--verdict accepted` with `--cause`, `--lesson` or `--changed-*` | `error: --cause, --lesson, --changed-skill and --changed-orchestrator need --verdict rejected` |
| 2 | Not exactly one of `--said`, `--said-file` | `error: give exactly one of --said and --said-file` |
| 3 | Said text empty | `error: what the user said must not be empty` |
| 4 | Both `--lesson` and `--lesson-file` | `error: give at most one of --lesson and --lesson-file` |
| 5 | A lesson with no `--cause` | `error: a lesson needs a cause; add --cause` |
| 6 | `--changed-*` with no `--cause` | `error: --changed-skill and --changed-orchestrator need a cause; add --cause` |
| 7 | `--cause` with no lesson | `error: a cause needs a lesson; add --lesson or --lesson-file` |
| 8 | Lesson empty | `error: the lesson must not be empty` |
| 9 | Lesson longer than 600 characters | `error: the lesson is <n> characters; the limit is 600` |

A missing `--verdict` or a value that is not allowed gets the parser's own message, and exit 2. An unreadable `--said-file` or `--lesson-file` is not a usage error: it exits 1 with `error: cannot read <path>: <reason>`.

Why a lesson comes only with `rejected`: a lesson comes from a root cause analysis (5.18, step 4). Lessons from accepted work are not in the sources.

Content rules for the lesson (the skill obeys them; `rex` does not check them):

- The lesson is general. It still helps when every name in it is removed (5.18, step 6).
- Where a cause is `missing-context`, the lesson says that agents should ask for that context when given such a task (5.18).
- The limit is 600 characters because `rex recall` puts lessons into the builder's context.

When the cause is `agent-model-effort`, the skill judges the model choice in this order and only the last step blames the mapping:

1. Did the chosen agent run? If the model or effort that ran differs from the role's `agent`, the environment overrode it. The mapping is not at fault.
2. Did a cap fire? If rule C1, C2 or C3 is in the role's `rules`, the user's budget or duration limited the choice.
3. Were the inputs right? If the builder rated a hard role Low on intelligence, the lesson belongs to the builder's skill.
4. Otherwise the mapping is suspect. Look for the same rule set failing across several jobs, not one case.

### 4.3 Status table

This is the status change `rex feedback` needs from Lane 2's transition function. That function is the single place for the rules; this table is what C9 requires of it. "Cause and lesson" means at least one cause and a lesson.

| Status before | Verdict | Cause and lesson | Status after | Result |
|---|---|---|---|---|
| `delivered` | `accepted` | none | `accepted` | row added |
| `delivered` | `rejected` | no | `rejected` | row added |
| `delivered` | `rejected` | yes | `analyzed` | row added; goes through `rejected` in one call |
| `accepted` | `accepted` | none | `accepted` | row added; status stays |
| `accepted` | `rejected` | no | `rejected` | row added; as from `delivered` (the user found a fault after confirming) |
| `accepted` | `rejected` | yes | `analyzed` | row added |
| `rejected` | `rejected` | yes | `analyzed` | row added |
| `rejected` | `rejected` | no | unchanged | error "analysis due" |
| `rejected` | `accepted` | none | unchanged | error "analysis due" |
| `analyzed` | any | any | `analyzed` | row added; status stays |
| `failed`, `cancelled`, `blocked` | any | any | unchanged | row added; status stays |
| `created`, `planned` | any | any | unchanged | refused: the job has no outcome yet |

Notes:

- Rejecting with a known cause is one call. Two calls also work: first `rejected` with no cause (the job stays `rejected`), then `rejected` with cause and lesson (the job becomes `analyzed`).
- For `failed`, `cancelled` and `blocked` the status already says the work did not finish, so feedback adds a row and leaves the status. A `blocked` job can still go on by `rex job close` (C2). This is a need on Lane 2 (section 7).
- A job whose shown status is `running` has the stored status `planned`. It is refused.
- State errors exit 1. They print one line that names the job, its stored status and the fix.
- The status and the new row are written in ONE transaction. If one fails, neither is written.

### 4.4 Output

Four lines, in this order:

| Key | Value |
|---|---|
| `job` | The job ID. |
| `feedback` | ID of the row this call added. |
| `previous` | Stored status before the call. |
| `status` | Stored status after the call. |

Each call adds one row. The command is not idempotent: the same call twice adds two rows.

### 4.5 Worked examples

Accepted:

```
$ rex feedback p6y2r8e4 --verdict accepted --said "Works as asked."
job=p6y2r8e4
feedback=1
previous=delivered
status=accepted
```

Rejected, with cause and lesson. The job goes to `analyzed` in one call:

```
$ rex feedback h2n6v0b4 --verdict rejected --said "It changed the config format without asking." --cause missing-context --lesson "Ask which config formats must stay readable before changing a config file."
job=h2n6v0b4
feedback=3
previous=delivered
status=analyzed
```

Rejected again, with no cause, on a job that is already `rejected`. Standard error, exit 1:

```
$ rex feedback c3v8n1t6 --verdict rejected --said "Still wrong."
error: job c3v8n1t6 is rejected and its analysis is due; give --cause and --lesson with --verdict rejected
```

A job with no outcome yet. Standard error, exit 1:

```
$ rex feedback m4q7t2c9 --verdict accepted --said "Looks good."
error: job m4q7t2c9 is planned and has no outcome yet; close it with rex job close first
```

For `created` the text is the same with `created` in place of `planned`.

### 4.6 Finding the job (5.18, step 2)

The skill gets the job ID in one of three ways:

1. From a commit's `Rex-Job:` line.
2. From a branch name `<type>/<slug>-<job-id>`. The ID is the part after the last `-`. Job IDs hold no `-` (Lane 2 owns this rule).
3. From `rex jobs --json` with `--search`, `--since` and `--until` (section 5).

## 5. `rex jobs`

Sources: 5.13, 5.18 step 2. Lists jobs for scripts, agents and people. This is also how `rex-feedback` searches the store by request text and date.

```
rex jobs [--json] [--search <text>]... [--since <date>] [--until <date>]
```

### 5.1 Filters and order

- `--search <text>`: repeatable. A job matches if its request text contains the text, ignoring case. All terms must match.
- `--since <date>`, `--until <date>`: dates as `YYYY-MM-DD`, UTC. They apply to the date part of the job's `created_at`. Both ends are inclusive. Dates are compared as text. (`--since 2026-11-10 --until 2026-11-10` gives every job created on 10 November 2026 UTC.) `--since` later than `--until` matches nothing and is not an error.
- A date that is not `YYYY-MM-DD`, or whose month is not 01 to 12 or whose day is not 01 to 31, is a usage error: `error: --since needs a date as YYYY-MM-DD` (or `--until`). Exit 2.
- Order: newest `created_at` first. There is no limit. A limit would cut a list that scripts expect to be complete; filter by status with `jq`.
- Zero matches is not an error. Exit 0.

### 5.2 JSON output

`--json` prints one line. Two jobs, the second one running and with a request longer than 200 characters:

```
$ rex jobs --json
{"schema":1,"jobs":[{"id":"m4q7t2c9","created_at":"2026-11-10T11:30:19Z","updated_at":"2026-11-10T11:41:52Z","status":"running","stored_status":"planned","request":"Make the order validator reject a quantity of zero with a typed error, and add a test for it. Also check every other caller of the validator in the order router and the batch importer, and make sure n","request_truncated":true,"asked_by":"user","class_version":1,"work_type":"fix","size":"medium","consequence":"high","languages":["go","rust"],"orchestrator":{"name":"lead-and-verifier","version":1},"merge_agreed":false,"builder":null,"deliveries":[],"runs":{"open":2,"finished":1,"unconfirmed":0},"tokens":{"input":9412,"output":2877,"cache_read":151630,"cache_creation":12044,"provisional":true},"has_feedback":false},{"id":"k8d2p5w1","created_at":"2026-11-10T09:14:03Z","updated_at":"2026-11-10T10:02:47Z","status":"delivered","stored_status":"delivered","request":"Add a --json flag to rex status so scripts can read its output.","request_truncated":false,"asked_by":"user","class_version":1,"work_type":"feature","size":"small","consequence":"low","languages":["rust"],"orchestrator":{"name":"solo-engineer","version":2},"merge_agreed":true,"builder":{"model":"opus","effort":"high"},"deliveries":[{"repo":"widgets","branch":"feature/rex-status-json-k8d2p5w1","pr":"https://github.com/acme/widgets/pull/12","outcome":"merged"}],"runs":{"open":0,"finished":2,"unconfirmed":0},"tokens":{"input":18234,"output":6120,"cache_read":402113,"cache_creation":21580,"provisional":false},"has_feedback":false}]}
```

With no match:

```
$ rex jobs --json --search nothing-like-this
{"schema":1,"jobs":[]}
```

Fields of one job object, in this order:

| Name | Type | Meaning | When null |
|---|---|---|---|
| `id` | string | Job ID. | never |
| `created_at` | string | When the job was opened. | never |
| `updated_at` | string | When the job record last changed. | never |
| `status` | string | The shown status. May be `running` (a planned job with sub-agents open). | never |
| `stored_status` | string | The stored status. Never `running`. | never |
| `request` | string | The first 200 characters of the request text (characters, not bytes). | never |
| `request_truncated` | bool | `true` if the request is longer than 200 characters. | never |
| `asked_by` | string | Who asked, as recorded by `rex job create` (C2). | never |
| `class_version` | number | Version of the classification scheme. | never |
| `work_type` | string | `feature`, `fix`, `refactor`, `chore`, `research` or `other`. | never |
| `size` | string | `small`, `medium` or `large`. | never |
| `consequence` | string | `low`, `medium` or `high`. | never |
| `languages` | array of string | Sorted language names, or `["none"]`. | never |
| `orchestrator` | object | `{"name": string, "version": number}`. | before the job is planned |
| `merge_agreed` | bool | Whether a merge to main was agreed. | if the record has no answer |
| `builder` | object | `{"model": string, "effort": string}` that ran the builder. | when unknown |
| `deliveries` | array | One object per repository: `repo`, `branch`, `pr`, `outcome` (all strings; `pr` is `null` when there is no pull request). Texts come from C2 unchanged. | `[]` when none |
| `runs` | object | `{"open": n, "finished": n, "unconfirmed": n}`: counts of sub-agent runs (C5). | never (zeros) |
| `tokens` | object | `{"input": n, "output": n, "cache_read": n, "cache_creation": n, "provisional": bool}` summed over the job's runs (C5). | never (zeros) |
| `has_feedback` | bool | The job has at least one feedback row. | never |

The full record of one job is `rex job show` (C2). `rex jobs` shows only this summary.

### 5.3 Plain output

Without `--json`, one line per job:

```
<id> <status> <type> <size> <consequence> <languages> <created_at> <request excerpt>
```

`languages` is comma-joined. The excerpt is the first 200 characters of the request, each newline replaced by one space, and it takes the rest of the line. Example:

```
$ rex jobs --search status --search JSON
k8d2p5w1 delivered feature small low rust 2026-11-10T09:14:03Z Add a --json flag to rex status so scripts can read its output.
```

With no match the command prints nothing and exits 0.

## 6. `rex watch`

Source: 5.13. A live view for people. No skill calls it.

```
rex watch
```

- It draws a ratatui screen over the store. It takes in the event log every `REFRESH_SECONDS` (2) seconds. It changes nothing else.
- Data comes from the same summary builder as `rex jobs`, plus the run tree from C5. So the screen and `rex jobs --json` agree: the same set of jobs, and for each job the same status, run counts and token totals.
- Keys:

| Key | Does |
|---|---|
| Up, Down | Move the selection in the job list. |
| Enter | Switch between the job list and the tree of runs of the selected job. |
| `q` | Quit. |

- Job list: the columns of the plain output of `rex jobs` (section 5.3), and the run counts and token totals.
- Run tree: sub-agent runs, nested by parent. Each shows its role, model, effort, duration and tokens. Durations come from C5 timestamps.
- If standard output is not a terminal, nothing is drawn. The command exits 2:

```
$ rex watch > out.txt
error: rex watch needs a terminal
```

## 7. What C9 needs from other lanes

Store boundary: Lane 2 provides the open call, the write transaction with writer wait, the transition function and `set_status`. Lane 7 owns its own SELECTs, the INSERT of the feedback row and the UPDATE of `experience_recalled`. No other lane writes those.

| Need | Who | Why |
|---|---|---|
| Job columns: `id`, `created_at`, `updated_at`, `status`, `request`, `asked_by`, `class_version`, `work_type`, `size`, `consequence`, `languages` (sorted, comma-joined text), orchestrator name and version, `merge_agreed`, builder model and effort | Lane 2 (C1) | `rex recall` scores on four of them; `rex jobs` and `rex watch` print them all. |
| Job column `experience_recalled TEXT NULL` | Lane 2 (C1) | `rex recall` writes it (OP8). |
| Delivery rows per job: repo, branch, pr, outcome | Lane 2 (C1) | `deliveries` in `rex jobs`. |
| Feedback table: `id` (integer), `job_id`, `given_by`, `said`, `verdict`, `causes` (text), `lesson`, `changed_skills`, `changed_orchestrators`, `created_at` | Lane 2 (C1) | `rex feedback` inserts a row; `rex recall` reads lessons. The lesson is a column of the feedback row. |
| Orchestrator index with `name`, `version`, `description` (empty allowed) | Lane 2 (C1) | Recall lists unused orchestrators and prints their description. |
| A transition function that gives the new status for the table in section 4.3 and returns the errors "analysis due" and "no outcome yet" | Lane 2 (C2) | `rex feedback` has no status rules of its own. |
| `blocked` stays able to continue by `rex job close` | Lane 2 (C2) | Feedback on a `blocked` job leaves the status alone, so it must not trap the job. |
| One function that gives the shown status (the `running` rule), used by `rex job show`, `rex jobs` and `rex watch` | Lane 2 (C2) with Lane 3 (C5) | The three agree on status. |
| Job IDs hold no `-` | Lane 2 (C2) | The skill reads the ID from a branch name after the last `-`. |
| Timestamps are ISO 8601 UTC text | Lane 2 (C1) | Dates compare as text; there is no date library. |
| Store API: open from any folder (including a worktree); a write transaction that waits for other writers | Lane 2 (C1) | Both writing commands run from any worktree. |
| Intake entry point | Lane 3 (C5) | The first step of `rex jobs` and `rex watch`. |
| Per job: run counts (open, finished, unconfirmed) and token totals with the provisional flag | Lane 3 (C5) | `runs` and `tokens` in `rex jobs`. |
| The run tree: parent links, role, model, effort, start and end times, tokens per run | Lane 3 (C5) | `rex watch`. It is the same source as `rex job show`. |
| Final exit codes, error style and a way to add an error without editing `src/error.rs`; the `ratatui` dependency (with serde_json and rusqlite shared) | Lane 1 (C11) | Section 2. |

## 8. Open points

### OP8 — proposed

Question: where is "the experience recalled" written on the job (5.9), and how does `rex-feedback` search the store by request text and date (5.18, step 2)?

Answer, part 1 (where it is written): `rex recall` writes it itself, into the job column `experience_recalled` (section 3.6). Reason: rex records facts itself (5.9). If the builder passed the text to `rex job plan`, the agent could paraphrase it or leave it out. A separate lesson table was rejected: a lesson is one column of a feedback row. This needs the column in C1.

Answer, part 2 (how the feedback skill searches): `rex jobs --json` with `--search`, `--since` and `--until` (section 5). Reason: `rex-feedback` already reads `rex jobs --json` and needs no new command. The search is a case-insensitive substring of the request text with all terms required, and a UTC date range on `created_at`. A skill that already has the ID uses the commit `Rex-Job:` line or the branch name (section 4.6).

### OP10 (feedback part) — proposed

Question: which states accept feedback, is `blocked` still able to go on, and are `rejected` and `analyzed` one call or two?

Answer: the table in section 4.3. Feedback works on a job with an outcome (`delivered`, `accepted`, `rejected`, `analyzed`, `failed`, `cancelled`, `blocked`) and is refused on `created` and `planned`. Rejecting with a cause and lesson is one call; two calls also work. A `rejected` job needs a cause and lesson before it moves ("analysis due"). Reason: the status already says what is owed. A `rejected` job owes an analysis, so a bare verdict cannot move it. For `failed`, `cancelled` and `blocked` the status already says the work did not finish, so a row is added and the status stays. Lane 2 keeps the transition table; the rest of OP10 (the other commands) is C2's.

## 9. Assumptions

Where the sources and the plan leave a detail open, C9 chose as follows. Each can change in review.

| # | Assumption |
|---|---|
| A1 | The lesson cap is 600 characters, because recall puts lessons into the builder's context. A constant in one place. |
| A2 | The request excerpt in `rex jobs` is 200 characters. A constant in one place. |
| A3 | `REFRESH_SECONDS` is 2 for `rex watch`. |
| A4 | Dates are UTC and compared as ISO 8601 text, with no date library. The date filters apply to the first ten characters of `created_at`. |
| A5 | `rex watch` refuses a non-terminal with exit 2. Tests use the data layer, not a terminal. |
| A6 | Recall constants: `MIN_SCORE=2`, `MAX_LESSONS=5`, `MAX_ORCHESTRATORS=3`, at the top of one file. Task B 8 (decision 6) says to adjust them after real use. |
| A7 | Error wording in sections 2, 4.2, 4.5 and 5.1 is C9's choice. C11 may change the style (for example the `error: ` prefix). The facts in each message (job, status, fix) stay. |
| A8 | `--json` prints one line and has `"schema":1`. C11 may change the JSON layout rule. |
| A9 | `version` in an orchestrator group is the version on the newest job of the group. |
| A10 | An empty `--lesson` or `--said` is a usage error. Duplicate causes and changed names are removed. |
| A11 | An unreadable `--said-file` or `--lesson-file` is a runtime error (exit 1), not a usage error. |
| A12 | `rex jobs` plain output with no match prints nothing. |
| A13 | In `rex watch`, Enter switches between the job list and the run tree; `q` quits from either. |
| A14 | `merge_agreed` is `null` if the job record holds no answer; otherwise a bool. The texts of `outcome`, `asked_by`, `model` and `effort` come from C2 unchanged. |
| A15 | Feedback with `accepted` cannot carry a cause, lesson or change list. |

## 10. Source map

| Source | Covered in |
|---|---|
| Task B 4 (matching, recall, reuse, the two queries) | Sections 3.1 to 3.5 |
| Task B 5 (columns on the job table: `class_version`, `work_type`, `size`, `consequence`, `languages` stored sorted, comma-joined) | Section 3.1, section 5.2 (fields), section 7 (columns needed) |
| Task A 5 (output style: `key=value` lines) | Section 2 |
| Task A 8 (order for judging a model choice) | Section 4.2 |
| 5.8 (statuses, transitions, root cause) | Sections 4.1, 4.3, 5.2 |
| 5.9 (records: feedback fields, experience recalled) | Sections 3.6, 4, 7 |
| 5.13 (terminal views) | Sections 5 and 6 |
| 5.18 (feedback flow) | Sections 4.2, 4.3, 4.6, 5 |
