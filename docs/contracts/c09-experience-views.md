Status: FROZEN (10 November 2026, by Lane 1 at gate G1, on top of commit 136720b). A change to this file goes through Lane 1.

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

- **Exit codes.** As C11 part 4: 0 done; 1 the command failed (the store, a job's state, a file); 2 the command line is wrong (clap's own check, or an error whose `is_usage()` is true).
- **Error text.** As C11 part 5. `main` writes `error: ` and appends the error's sources joined by `: `. The texts in this contract show the whole printed line. The error's own text is the part after `error: `, and an underlying I/O or SQLite error is a `#[source]`, not part of the text.
- **Job argument.** `<job>` is an exact job ID (C2 2.4), checked by a clap value parser over `JobId`. No prefix match. Other text is a usage error, exit 2; the message says that the text is not a job ID and gives the form: 10 lower-case letters and digits, such as `0g4cbf7k2q`.
- **Unknown job.** The command fails with C1's `JobNotFound` (C1 8). Exit 1.
- **No store.** If no store is found, the command exits 1 with the error that C1's open call gives.
- **Time.** Every timestamp is printed as stored: ISO 8601 UTC with milliseconds, `YYYY-MM-DDTHH:MM:SS.sssZ`, such as `2026-11-10T09:14:03.512Z` (C1 5.1).
- **Output style.** Plain output is `key=value` lines in a fixed order. The first line is the answer (Task A 5 style). Where a record repeats (`rex recall`), each repeated line starts with a word that names its kind. If C11 defines a list style, C9 adopts it.
- **Values.** In a `key=value` pair the value has no spaces. The last key of a line (`text=`, `description=`) takes the rest of the line.
- **Intake.** `rex jobs` and `rex watch` call `capture::job_runs` for each job they show; it takes in the log itself (C5 1). `rex recall` and `rex feedback` call nothing of capture.
- **What each command writes.** `rex recall` writes the orchestrator index (section 3.4), one job field and `updated_at` (section 3.6). `rex feedback` writes one feedback row and the job status (section 4). `rex jobs` and `rex watch` write nothing except what `job_runs` stores (C5).

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
| `missed` | Jobs whose status is `rejected`, `analyzed` or `failed`. The rest of `runs` are jobs in any other stored status. |
| `version` | The orchestrator version on the newest job of the group (by `created_at`). |
| `path` | The absolute path of that orchestrator's `SKILL.md`: `<store>/orchestrators/<name>/SKILL.md`, from `StorePaths.orchestrators` (C1 7.1). |

- Order: `best_match` high to low, then `runs` high to low. At most 3 groups (`MAX_ORCHESTRATORS=3`).
- Status is the stored status. (`running` is never stored, so it never appears here.)

### 3.4 Unused orchestrators

`rex recall` calls `orchestrator::index_all` (C1 7.4) inside its write (section 3.6), before it reads the index, with Lane 6's header reader (C7 2) as the `describe` function. So an orchestrator that a person added, or that `rex init` seeded, shows at the first recall.

Unused: each orchestrator name in the index (C1 5.8) whose newest index version no job names. One line per name, with the version, the `path` (as in section 3.3) and the description of its newest index row. An `unused` line already marks an orchestrator whose newest version never ran, so it needs no other mark. An empty description prints `description=` empty (JSON `""`). Order: name, A to Z. No limit.

### 3.5 Output

Plain output, in this order. All count lines print even when the count is 0.

| Line | Form |
|---|---|
| 1 | `job=<id>` |
| 2 | `recorded=yes` or `recorded=no` (section 3.6) |
| 3 | `lessons=<n>` |
| next n | `lesson feedback=<id> job=<id> score=<n> status=<status of that job> causes=<a,b or none> text=<rest of line>` |
| next | `orchestrators=<n>` |
| next n | `orchestrator name=<name> version=<v> best_match=<n> runs=<n> accepted=<n> missed=<n> path=<rest of line>` |
| next | `unused=<n>` |
| next n | `unused name=<name> version=<v> path=<path> description=<rest of line>` (`path` ends at the text ` description=`, so a path may hold a space) |

- `causes` are the feedback row's causes, sorted, comma-joined. `none` if the row has no cause.
- Newlines inside a lesson or description become one space each.

Worked example. Job `0g4imj3x8d` is a `fix`, `medium`, `high`, `go,rust` job in status `created`.

```
$ rex recall 0g4imj3x8d
job=0g4imj3x8d
recorded=yes
lessons=2
lesson feedback=3 job=0g30kh5n1w score=4 status=analyzed causes=missing-context text=Ask which config formats must stay readable before changing a config file.
lesson feedback=2 job=0g1d7s3j9x score=2 status=analyzed causes=agent-model-effort,agent-skill text=Give the verifier a model one tier higher when no test can check the result.
orchestrators=1
orchestrator name=lead-and-verifier version=1 best_match=4 runs=3 accepted=2 missed=1 path=/home/ana/code/widgets/.rex/orchestrators/lead-and-verifier/SKILL.md
unused=1
unused name=research-panel version=1 path=/home/ana/code/widgets/.rex/orchestrators/research-panel/SKILL.md description=Three readers compare options and one writer recommends.
```

With no similar jobs and an empty index:

```
$ rex recall 0g4imj3x8d
job=0g4imj3x8d
recorded=yes
lessons=0
orchestrators=0
unused=0
```

`--json` prints one line of JSON. The same example:

```
$ rex recall 0g4imj3x8d --json
{"schema":1,"job":"0g4imj3x8d","recorded":true,"lessons":[{"feedback":3,"job":"0g30kh5n1w","score":4,"status":"analyzed","causes":["missing-context"],"text":"Ask which config formats must stay readable before changing a config file."},{"feedback":2,"job":"0g1d7s3j9x","score":2,"status":"analyzed","causes":["agent-model-effort","agent-skill"],"text":"Give the verifier a model one tier higher when no test can check the result."}],"orchestrators":[{"name":"lead-and-verifier","version":1,"best_match":4,"runs":3,"accepted":2,"missed":1,"path":"/home/ana/code/widgets/.rex/orchestrators/lead-and-verifier/SKILL.md"}],"unused":[{"name":"research-panel","version":1,"path":"/home/ana/code/widgets/.rex/orchestrators/research-panel/SKILL.md","description":"Three readers compare options and one writer recommends."}]}
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
| `orchestrators` | array | One object per group. Keys as in section 3.3, `path` included. | `[]` |
| `unused` | array | One object per unused orchestrator: `name`, `version`, `path`, `description`. | `[]` |

In JSON, `causes` for a row with no cause is `[]` (plain output prints `none`). A description that is empty is `""`. Exit code 0 in every case above.

### 3.6 The one write: experience recalled

`rex recall` writes the job field `experience_recalled` (the "experience recalled" of the job record). Rex records facts itself, so the builder does not pass recalled text on.

- It writes only while the job's stored status is `created`. In any other status it writes nothing and prints `recorded=no`. The guard is `status = 'created'` in the one UPDATE of C1 5.12.
- A second call replaces the first. The last call wins.
- There is no flag to turn the write off. A repeat call does no harm.
- The field holds the exact JSON document that `rex recall <job> --json` prints for that call (section 3.5), one line. It holds no time: no caller reads one, and rex writes no time from Rust (C1 5.1). The same UPDATE sets `updated_at`.

`rex job show` (C2) prints this field as stored.

## 4. `rex feedback <job>`

Sources: 5.8, 5.9, 5.18. Records what the user said about a finished job, and moves the job's status. Called by `rex-feedback`.

```
rex feedback <job> --verdict accepted|rejected
               (--said <text> | --said-file <path>)
               [--by <who>]
               [--cause <value>]...
               [--lesson <text> | --lesson-file <path>]
               [--evidence <text>]
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
| `--evidence` | no | Free text: what in the record shows each cause. |
| `--changed-skill` | no, repeatable | A skill the analysis changed. |
| `--changed-orchestrator` | no, repeatable | An orchestrator the analysis changed. |

`--changed-skill` takes the folder name of the skill under `.claude/skills/` (such as `orchestration-selection`). `--changed-orchestrator` takes the folder name under `.rex/orchestrators/`; a lead skill inside it counts as that orchestrator. Names, not paths. `--evidence` is stored in `feedback.evidence` (C1 5.10). `rex recall` prints it nowhere; `rex job show --json` prints it (C2 6.3). With `--verdict accepted` the call carries no cause, no lesson and no evidence; what the user said goes in `--said`.

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

Rules 2, 4, 5, 6 and 7 are clap rules (argument groups and `requires`), so they print clap's own message; the lines shown give the facts, not the words. The command checks rules 1, 3, 8 and 9, in that order, after it reads the files, with the texts shown; those errors have `is_usage()` true. The lesson length counts Unicode characters (Rust `char`s), not bytes.

| # | Rule | Error text |
|---|---|---|
| 1 | `--verdict accepted` with `--cause`, `--lesson`, `--evidence` or `--changed-*` | `error: --cause, --lesson, --evidence, --changed-skill and --changed-orchestrator need --verdict rejected` |
| 2 | Not exactly one of `--said`, `--said-file` | `error: give exactly one of --said and --said-file` |
| 3 | Said text empty | `error: what the user said must not be empty` |
| 4 | Both `--lesson` and `--lesson-file` | `error: give at most one of --lesson and --lesson-file` |
| 5 | A lesson with no `--cause` | `error: a lesson needs a cause; add --cause` |
| 6 | `--changed-*` with no `--cause` | `error: --changed-skill and --changed-orchestrator need a cause; add --cause` |
| 7 | `--cause` with no lesson | `error: a cause needs a lesson; add --lesson or --lesson-file` |
| 8 | Lesson empty | `error: the lesson must not be empty` |
| 9 | Lesson longer than 600 characters | `error: the lesson is <n> characters; the limit is 600` |

A missing `--verdict` or a value that is not allowed gets the parser's own message, and exit 2. An unreadable `--said-file` or `--lesson-file` exits 1: `error: cannot read <path>: <system message>`, where the system message is the `#[source]`.

Why a lesson comes only with `rejected`: a lesson comes from a root cause analysis (5.18, step 4). Lessons from accepted work are not in the sources.

The limit is 600 characters because `rex recall` puts lessons into the builder's context.

### 4.3 Status table

This is the status change `rex feedback` gets from Lane 2's transition function, `status::apply` (C1 7.3). That function is the single place for the rules; this table is the feedback rows of C2 7. "Cause and lesson" means at least one cause and a lesson.

| Status before | Verdict | Cause and lesson | Status after | Result |
|---|---|---|---|---|
| `delivered` | `accepted` | none | `accepted` | feedback row added |
| `delivered` | `rejected` | no | `rejected` | feedback row added |
| `delivered` | `rejected` | yes | `analyzed` | feedback row added; goes through `rejected` in one call |
| `accepted` | `accepted` | none | `accepted` | feedback row added; status stays |
| `accepted` | `rejected` | no | `rejected` | feedback row added; as from `delivered` (the user found a fault after confirming) |
| `accepted` | `rejected` | yes | `analyzed` | feedback row added |
| `rejected` | `rejected` | yes | `analyzed` | feedback row added |
| `rejected` | `rejected` | no | unchanged | error "analysis due" |
| `rejected` | `accepted` | none | unchanged | error "analysis due" |
| `blocked`, `failed`, `cancelled`, `analyzed` | any | any | unchanged | Status stays. The feedback row is added. No transition row. |
| `created`, `planned` | any | any | unchanged | Refused (`NoOutcomeYet`): `job <job> is <status> and has no final outcome yet; close it with rex job close first` (C1 8). No feedback row. |

Notes:

- Rejecting with a known cause is one call. Two calls also work: first `rejected` with no cause (the job stays `rejected`), then `rejected` with cause and lesson (the job becomes `analyzed`).
- For `failed` and `cancelled` the status already says the work did not finish, so feedback adds a row and leaves the status.
- A call that leaves the status writes no `job_transition` row (C1 5.5). Feedback on a `blocked` job leaves the status, so a later `rex job close` cannot overtake it, and a lesson about a blocked job is worth keeping.
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
$ rex feedback 0g3p6y2r8e --verdict accepted --said "Works as asked."
job=0g3p6y2r8e
feedback=1
previous=delivered
status=accepted
```

Rejected, with cause and lesson. The job goes to `analyzed` in one call:

```
$ rex feedback 0g30kh5n1w --verdict rejected --said "It changed the config format without asking." --cause missing-context --lesson "Ask which config formats must stay readable before changing a config file."
job=0g30kh5n1w
feedback=3
previous=delivered
status=analyzed
```

Rejected again, with no cause, on a job that is already `rejected`. Standard error, exit 1:

```
$ rex feedback 0g3c3v8n1t --verdict rejected --said "Still wrong."
error: job 0g3c3v8n1t is rejected and its analysis is due; give --cause and --lesson with --verdict rejected
```

A job with no outcome yet. Standard error, exit 1:

```
$ rex feedback 0g4imj3x8d --verdict accepted --said "Looks good."
error: job 0g4imj3x8d is planned and has no final outcome yet; close it with rex job close first
```

For `created` the text is the same with `created` in place of `planned`. A `blocked` job takes feedback (section 4.3).

### 4.6 Finding the job (5.18, step 2)

The skill gets the job ID in one of three ways:

1. From a commit's `Rex-Job:` line.
2. From a branch name. A job branch is `<type>/<slug>-<job-id>`; a unit branch adds `--<unit>` (C12 5). Cut the name at the first `--`, then take the text after the last `-` (`b=${BRANCH%%--*}; JOB=${b##*-}`). This works because a slug holds no `--` and a job ID holds no `-` (C2 2.4). Use the result only if it is a job ID (10 characters of `[0-9a-z]`); else try the next way.
3. From `rex jobs --json` with `--search`, `--since` and `--until` (section 5).

### 4.7 One call, after the changes

`rex-feedback` calls `rex feedback` once, after its file changes (5.18, step 6) and their checks, so the changed lists name only changes that were kept. A second call adds a second row; no call amends a row. The two-call path of 4.3 is for a verdict recorded before its analysis.

## 5. `rex jobs`

Sources: 5.13, 5.18 step 2. Lists jobs for scripts, agents and people. This is also how `rex-feedback` searches the store by request text and date.

```
rex jobs [--json] [--search <text>]... [--since <date>] [--until <date>]
```

### 5.1 Filters and order

- `--search <text>`: repeatable. A job matches if its full request text contains the text as a literal substring, ignoring ASCII case only (`instr(lower(request), lower(:text)) > 0`). All terms must match.
- `--since <date>`, `--until <date>`: dates as `YYYY-MM-DD`, UTC. They apply to the date part of the job's `created_at`. Both ends are inclusive. Dates are compared as text. (`--since 2026-11-10 --until 2026-11-10` gives every job created on 10 November 2026 UTC.) `--since` later than `--until` matches nothing and is not an error.
- A date that is not `YYYY-MM-DD`, or whose month is not 01 to 12 or whose day is not 01 to 31, is a usage error: `error: --since needs a date as YYYY-MM-DD, not <text>` (or `--until`). Exit 2; `is_usage()` is true.
- Order: newest `created_at` first. There is no limit. A limit would cut a list that scripts expect to be complete; filter by status with `jq`.
- Zero matches is not an error. Exit 0.

### 5.2 JSON output

`--json` prints one line. Three jobs: the first one running and with a request longer than 200 characters; the third one created, with no runs and no main-session figure, so its `tokens` is `null`:

```
$ rex jobs --json
{"schema":1,"jobs":[{"id":"0g4imj3x8d","created_at":"2026-11-10T11:30:19.208Z","updated_at":"2026-11-10T11:41:52.637Z","status":"running","stored_status":"planned","request":"Make the order validator reject a quantity of zero with a typed error, and add a test for it. Also check every other caller of the validator in the order router and the batch importer, and make sure n","request_truncated":true,"asked_by":"human","class_version":1,"work_type":"fix","size":"medium","consequence":"high","languages":["go","rust"],"orchestrator":{"name":"lead-and-verifier","version":1},"merge_agreed":false,"builder_model":null,"builder_effort":null,"deliveries":[],"runs":{"open":2,"finished":1,"unconfirmed":0},"tokens":{"input":9412,"output":2877,"cache_read":151630,"cache_creation":12044,"provisional":true},"incomplete":false,"has_feedback":false},{"id":"0g4cbf7k2q","created_at":"2026-11-10T09:14:03.512Z","updated_at":"2026-11-10T10:02:47.091Z","status":"delivered","stored_status":"delivered","request":"Add a --json flag to rex status so scripts can read its output.","request_truncated":false,"asked_by":"human","class_version":1,"work_type":"feature","size":"small","consequence":"low","languages":["rust"],"orchestrator":{"name":"solo-engineer","version":2},"merge_agreed":true,"builder_model":"claude-opus-5-5","builder_effort":"high","deliveries":[{"repo":"/home/ana/code/widgets","branch":"feature/rex-status-json-0g4cbf7k2q","pr":"https://github.com/acme/widgets/pull/12","outcome":"merged"}],"runs":{"open":0,"finished":2,"unconfirmed":0},"tokens":{"input":18234,"output":6120,"cache_read":402113,"cache_creation":21580,"provisional":false},"incomplete":false,"has_feedback":false},{"id":"0g30ph2w6c","created_at":"2026-11-09T16:05:41.330Z","updated_at":"2026-11-09T16:05:41.330Z","status":"created","stored_status":"created","request":"Compare three ways to cache the price feed and recommend one.","request_truncated":false,"asked_by":"human","class_version":1,"work_type":"research","size":"small","consequence":"low","languages":["none"],"orchestrator":null,"merge_agreed":false,"builder_model":"claude-opus-5-5","builder_effort":"low","deliveries":[],"runs":{"open":0,"finished":0,"unconfirmed":0},"tokens":null,"incomplete":false,"has_feedback":false}]}
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
| `asked_by` | string | `human` or `agent`, as recorded by `rex job create` (C1 5.4). | never |
| `class_version` | number | Version of the classification scheme. | never |
| `work_type` | string | `feature`, `fix`, `refactor`, `chore`, `research` or `other`. | never |
| `size` | string | `small`, `medium` or `large`. | never |
| `consequence` | string | `low`, `medium` or `high`. | never |
| `languages` | array of string | Sorted language names, or `["none"]`. | never |
| `orchestrator` | object | `{"name": string, "version": number}`. | before the job is planned |
| `merge_agreed` | bool | Whether a merge to main was agreed. | never |
| `builder_model` | string | Model ID that ran the builder. | when unknown |
| `builder_effort` | string | Effort that ran the builder. | when unknown |
| `deliveries` | array | One object per repository, in the order first recorded: `repo` (absolute path of that repository's main checkout, C1 5.7), `branch` (string or null), `pr` (string or null), `outcome` (`merged`, `pr-open`, `kept-local`, `blocked`). | `[]` when none |
| `runs` | object | `{"open": n, "finished": n, "unconfirmed": n}`: counts of sub-agent runs, `JobRuns.open`, `finished` and `unconfirmed` (C5 1). | never (zeros) |
| `tokens` | object or null | `JobRuns.tokens` (C5): every run plus the main session (OP12), `{"input", "output", "cache_read", "cache_creation", "provisional"}`, or `null` when unknown. | when unknown |
| `incomplete` | bool | `JobRuns.incomplete` (C5 7) | never |
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
0g4cbf7k2q delivered feature small low rust 2026-11-10T09:14:03.512Z Add a --json flag to rex status so scripts can read its output.
```

With no match the command prints nothing and exits 0.

## 6. `rex watch`

Source: 5.13. A live view for people. No skill calls it.

```
rex watch
```

- It draws a ratatui screen over the store. Every `REFRESH_SECONDS` (2) seconds it calls `capture::job_runs` for each job it shows, which takes in the event log (C5 1). It writes nothing except what `job_runs` stores.
- Data comes from the same summary builder as `rex jobs`, plus the run tree from C5. So the screen and `rex jobs --json` agree: the same set of jobs, and for each job the same status, run counts and token totals.
- Keys:

| Key | Does |
|---|---|
| Up, Down | Move the selection in the job list. |
| Enter | Switch between the job list and the tree of runs of the selected job. |
| `q` | Quit. |

- Job list: the columns of the plain output of `rex jobs` (section 5.3), and the run counts and token totals.
- Run tree: sub-agent runs, nested by parent. Each shows its role, model, effort, duration and tokens. Durations come from C5 timestamps. A run whose parent is not a run of the job is shown at the top level (C5 4). A provisional total has `~` after it. When `tag_job` is set, the tag job is shown beside the run.
- If standard output is not a terminal, nothing is drawn and the command exits 1:

```
$ rex watch > out.txt
error: rex watch needs a terminal; run it without redirecting its output
```

## 7. What C9 needs from other lanes

Store boundary: Lane 2 provides `Store::open`, `Store::write` and `status::apply(tx, job, Event::Feedback { verdict, has_analysis })` (C1 7.2, 7.3). `rex feedback` takes `previous` and `status` for its output from the returned `Change`. There is no `set_status`. The shown status is `Shown::of(stored, open_runs)`, with `open_runs` = `JobRuns.open`. `rex jobs` and `rex watch` call `capture::job_runs(store: &mut Store, job: &JobId, now: SystemTime) -> Result<JobRuns, CaptureError>` (C5 1) for each job they show. Lane 7 owns its SELECTs, the INSERT of the feedback row and the UPDATE of `experience_recalled`, which also sets `updated_at`. No other lane writes those.

| Need | Who | Why |
|---|---|---|
| Job columns | Lane 2 (C1 5.4) | `rex recall` scores on four of them; `rex jobs` and `rex watch` print them all. |
| Job column `experience_recalled` and its one guarded UPDATE | Lane 2 (C1 5.4, 5.12) | `rex recall` writes it (OP8). |
| Delivery rows per job | Lane 2 (C1 5.7) | `deliveries` in `rex jobs`. |
| Feedback table, `evidence` included | Lane 2 (C1 5.10) | `rex feedback` inserts a row; `rex recall` reads lessons. The lesson is a column of the feedback row. |
| Orchestrator index and `orchestrator::index_all` | Lane 2 (C1 5.8, 7.4); Lane 6, the header reader (C7 2) | Recall indexes the library, lists unused orchestrators and prints their description. |
| `status::apply` with `Event::Feedback`, and the errors `AnalysisDue` and `NoOutcomeYet` | Lane 2 (C1 7.3, 8; the table is C2 7) | `rex feedback` has no status rules of its own. |
| `Shown::of(stored, open_runs)` (C1 7.3), with `open_runs` = `JobRuns.open` (C5) | Lane 2 (C1 7.3) with Lane 3 (C5 1) | `rex job show`, `rex jobs` and `rex watch` agree on status. |
| Job IDs hold no `-`; slugs hold no `--` | Lane 2 (C2 2.4); Lane 10 (C12 5) | The skill reads the ID from a branch name (section 4.6). |
| The one time form | Lane 2 (C1 5.1) | Dates compare as text; there is no date library. |
| `StorePaths::discover`, `Store::open`, `Store::write` | Lane 2 (C1 3.2, 7.2) | Both writing commands run from any worktree. |
| `capture::job_runs` and `JobRuns`: run counts, `tokens`, `incomplete`, the runs | Lane 3 (C5 1, 7, 8) | `runs`, `tokens` and `incomplete` in `rex jobs`; the job list of `rex watch`. |
| The run tree | Lane 3 (C5 1, 4) | `rex watch`. It is the same source as `rex job show`. |
| Exit codes, error style, how to add an error, the `ratatui` dependency | Lane 1 (C11 4 to 7) | Section 2. |

## 8. Open points

### OP8 — settled

Question: where is "the experience recalled" written on the job (5.9), and how does `rex-feedback` search the store by request text and date (5.18, step 2)?

Answer, part 1 (where it is written): `rex recall` writes it itself, into the job column `experience_recalled` (section 3.6), in one UPDATE that also sets `updated_at`, and only while the job is `created` (the guard `status = 'created'`, C1 5.12). The value is the exact one-line JSON document that `rex recall <job> --json` prints for that call. It holds no time. Reason: rex records facts itself (5.9). If the builder passed the text to `rex job plan`, the agent could paraphrase it or leave it out. A separate lesson table was rejected: a lesson is one column of a feedback row. The column is C1 5.4.

Answer, part 2 (how the feedback skill searches): `rex jobs --json` with `--search`, `--since` and `--until` (section 5). Reason: `rex-feedback` already reads `rex jobs --json` and needs no new command. `--search <text>` is a literal substring of the full request text, ASCII case ignored, and every term must match. `--since` and `--until` are UTC dates, compared with the first ten characters of `created_at`. A skill that already has the ID uses the commit `Rex-Job:` line or the branch name (section 4.6).

### OP10 (feedback part) — settled

Question: which states accept feedback, is `blocked` still able to go on, and are `rejected` and `analyzed` one call or two?

Answer: the table in section 4.3. Feedback is refused (`NoOutcomeYet`) only on `created` and `planned`. On `blocked`, `failed`, `cancelled` and `analyzed` the feedback row is added and the status stays (no transition row). On `delivered`, `accepted` and `rejected` it moves the status as section 4.3 says. A rejection with a cause and a lesson is one call; a bare `rejected` followed by an analysis is two calls. A `rejected` job needs a cause and lesson before it moves ("analysis due"). Reason: the status already says what is owed. A `rejected` job owes an analysis, so a bare verdict cannot move it. For `failed` and `cancelled` the status already says the work did not finish, so a row is added and the status stays. A call that leaves the status cannot be overtaken by a later close, and a lesson about a blocked job is worth keeping. C2 section 7 holds the one transition table. The rest of OP10 (the other commands) is C2's.

## 9. Assumptions

Where the sources and the plan leave a detail open, C9 chose as follows. Each can change in review.

| # | Assumption |
|---|---|
| A1 | The lesson cap is 600 characters, because recall puts lessons into the builder's context. A constant in one place. |
| A2 | The request excerpt in `rex jobs` is 200 characters. A constant in one place. |
| A3 | `REFRESH_SECONDS` is 2 for `rex watch`. |
| A4 | Dates are UTC and compared as ISO 8601 text, with no date library. The date filters apply to the first ten characters of `created_at`. |
| A5 | `rex watch` refuses a non-terminal with exit 1 (C11 part 4). Tests use the data layer, not a terminal. |
| A6 | Recall constants: `MIN_SCORE=2`, `MAX_LESSONS=5`, `MAX_ORCHESTRATORS=3`, at the top of one file. Task B 8 (decision 6) says to adjust them after real use. |
| A7 | Error texts follow C11 part 5. The facts in each message (job, status, fix) stay. |
| A8 | `--json` prints one JSON document on one line, `snake_case` keys, `"schema":1` first (C11 part 3). |
| A9 | `version` in an orchestrator group is the version on the newest job of the group. |
| A10 | An empty `--lesson` or `--said` is a usage error. Duplicate causes and changed names are removed. |
| A11 | An unreadable `--said-file` or `--lesson-file` is a runtime error (exit 1), not a usage error. |
| A12 | `rex jobs` plain output with no match prints nothing. |
| A13 | In `rex watch`, Enter switches between the job list and the run tree; `q` quits from either. |
| A14 | The texts of `outcome`, `asked_by`, `builder_model` and `builder_effort` come from C2 unchanged. |
| A15 | Feedback with `accepted` cannot carry a cause, lesson, evidence or change list. |

## 10. Source map

| Source | Covered in |
|---|---|
| Task B 4 (matching, recall, reuse, the two queries) | Sections 3.1 to 3.5 |
| Task B 5 (columns on the job table: `class_version`, `work_type`, `size`, `consequence`, `languages` stored sorted, comma-joined) | Section 3.1, section 5.2 (fields), section 7 (columns needed) |
| Task A 5 (output style: `key=value` lines) | Section 2 |
| Task A 8 (order for judging a model choice) | Not in C9: the `rex-feedback` skill holds it (section 4.2) |
| 5.8 (statuses, transitions, root cause) | Sections 4.1, 4.3, 5.2 |
| 5.9 (records: feedback fields, experience recalled) | Sections 3.6, 4, 7 |
| 5.13 (terminal views) | Sections 5 and 6 |
| 5.18 (feedback flow) | Sections 4.2, 4.3, 4.6, 5 |
