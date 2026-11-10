Status: DRAFT

# C2. Job commands: `rex job create`, `rex job plan`, `rex job close`, `rex job show`

This contract fixes the flags, input files, output, errors and status changes of the four job commands, and the one transition table behind them.
Read by: Lanes 3, 7, 9, 10.

Lane 2 builds the commands. C2 is frozen only after the review of the contract drafts. A skill or a lane can call the commands, or parse their output, from this file alone. The store they write is C1.

## 1. Callers

| Caller | Calls | Why |
|---|---|---|
| `orchestration-builder` (Lane 9) | `rex job create`, `rex job plan`, `rex job show` | Opens the job (step 3 of the flow, build request 5.1), records the plan (step 7). |
| The session, following `rex-deliver` (Lane 10) | `rex job close`, `rex job show` | Closes the job with one delivery per repository (step 11; 5.16). |
| `rex-feedback` (Lane 10) | `rex job show` | Reads what happened before the analysis (5.18). |
| Lane 7 (`rex feedback`, `rex jobs`, `rex watch`) | `status::apply`, `status::shown` (C1 7.3); this table (section 7) | Feedback moves the status through the same function. |
| Lane 3 | `rex job show` prints its run read model (C5) | Runs and tokens per job. |

## 2. Common rules

### 2.1 Output

- Plain output is `key=value` lines in a fixed order. The first line is the answer, as `rex oms` (Task A 5) and C9 2.
- In a `key=value` pair the value has no spaces. The last key of a line takes the rest of the line, spaces included.
- A value that is not set prints `none`, except where a section says otherwise.
- Times print as stored: `2026-11-10T09:14:03.512Z` (C1 5.1).
- Output goes to standard output. Notes and errors go to standard error.

### 2.2 `--json`

Each command takes `--json`. It prints one JSON object on one line, with `"schema":1` first (as C9). Lists are JSON arrays, flags are `true` or `false`, a value that is not set is `null`. Errors are the same plain `error: ` line on standard error with `--json` too.

### 2.3 Exit codes and errors

| Code | When |
|---|---|
| 0 | Success. Also `rex job create` in a session that already has an open job (OP4). |
| 1 | An error of the store or the job: store not found, job not found, a transition refused, an input file that cannot be read or is not valid. |
| 2 | A usage error: a missing or unknown flag, a value not allowed. The parser lists the allowed values. |

An error is one line on standard error that starts with `error: `. It says what failed, then the fix. The texts are given below for each command; the store's own errors are in C1 8. A missing required flag, or a value not in a flag's list, gets the parser's own message and exit 2.

These rules follow C11 (Lane 1). If C11 differs at its freeze, C11 wins and C2 changes.

### 2.4 Job ID

A job ID is 10 characters of `[0-9a-z]`, with no `-`, for example `0g4cbf7k2q`.

- The first 6 characters are the seconds since `2026-01-01T00:00:00Z` in base 36, zero-padded. So IDs sort by time as text, and the form lasts until the year 2095.
- The last 4 characters are random, base 36.
- It holds no `-`, so `rex-feedback` can read it from a branch `<type>/<slug>-<job-id>` as the part after the last `-` (C9 4.6). It is safe in a branch name.
- A `<job>` argument is an exact ID; there is no prefix match. Text that is not of this form is a usage error, exit 2: `error: <text> is not a job ID; a job ID is 10 lower-case letters and digits, such as 0g4cbf7k2q`.

### 2.5 Input files

- Input files are JSON, UTF-8. `-` in place of a path means standard input.
- A key that the file form does not name is an error. So is a missing required key, or a value of the wrong type.
- A path is relative to the folder the command runs in.
- The request file of `rex job create` is plain text, not JSON.

### 2.6 Intake first

`rex job plan`, `rex job close` and `rex job show` first take in new lines of the hook event log (C5 intake), so that runs are tied to their job before the command reads or moves it. Taking in the same line twice changes nothing. Until Lane 3 merges, intake does nothing. `rex job create` does not take in the log.

### 2.7 The caller rule: which store

Run `rex job ...` from the session's project folder, a sub-folder of it, or a Git worktree of it. From a sibling repository, set `REX_STORE=<project root>/.rex` for the command, for example `REX_STORE=/home/ana/code/shop/.rex rex job show 0g4cbf7k2q`. The reason and the full search rule are in C1 3.2. Every "job not found" and "no open job" error prints the store path it used, so a wrong store is visible at once.

### 2.8 What depends on contracts not yet frozen

| Part | Depends on | Until then |
|---|---|---|
| `running` as the shown status; the `run.` lines and `runs` in `rex job show` | C5 (Lane 3): open-run count, run read model | The open-run count is 0, so `running` never shows, and there are no run lines. |
| Intake (section 2.6) | C5 (Lane 3): intake entry point | Does nothing. |
| Exit codes, the `error: ` form, the JSON layout rule | C11 (Lane 1) | As in sections 2.1 to 2.3. |
| OMS in `rex job plan` | Lane 6: `oms::select`, `Inputs`, `Selection`, `VERSION`, `Rule::id()`, `GridAgent` as a library API (Task A 6) | `rex job plan` cannot be built. |
| The orchestrator `description` | C7 (Lane 6): the `SKILL.md` header form | Read as the `description:` key of a YAML header. |
| What the builder passes to `rex job close` for each repository | C12 (Lane 10) | Section 5 follows the C12 draft's section 6 field names and values. If C12 changes them before its freeze, section 5 follows. |

## 3. `rex job create`

Opens a job with its request, classification and session ID. Gives `created`. Source: build request 5.1 step 3, 5.6, 5.9; Task B 5.

```
rex job create --request-file <path|-> --asked-by human|agent --merge yes|no
               --type <type> --size <size> --consequence <level> --lang <name>...
               [--session <id>] [--duration <level>] [--budget <level>] [--autonomy <level>]
               [--builder-model <id>] [--builder-effort <level>] [--json]
```

### 3.1 Flags

| Flag | Required | Values | Default | Meaning |
|---|---|---|---|---|
| `--request-file` | yes | a path, or `-` | — | File with the request text. Must not be empty. |
| `--asked-by` | yes | `human`, `agent` | — | Who made the request. |
| `--merge` | yes | `yes`, `no` | — | Whether a merge to main was agreed (5.16 step 1). |
| `--type` | yes | `feature`, `fix`, `refactor`, `chore`, `research`, `other` | — | Work type (Task B 1.1). |
| `--size` | yes | `small`, `medium`, `large` | — | Size (Task B 1.2). |
| `--consequence` | yes | `low`, `medium`, `high` | — | Consequence (Task B 1.3). |
| `--lang` | yes, repeatable | a name of `[a-z0-9]+`, or `none` alone | — | A language of the source files the job changes (Task B 1.4). Lowercased, sorted, duplicates removed. |
| `--session` | no | text | `$CLAUDE_CODE_SESSION_ID` | The Claude Code session ID. Error if neither is given. |
| `--duration` | no | `low`, `medium`, `high`; also `fast` (= `low`), `long` (= `high`) | not stated | The user's stated duration preference. |
| `--budget` | no | `low`, `medium`, `high` | not stated | The user's stated budget preference. |
| `--autonomy` | no | `low`, `medium`, `high` | not stated | The user's stated autonomy preference. |
| `--builder-model` | no | text | unknown | Model ID running the builder (OP15). |
| `--builder-effort` | no | text | `$CLAUDE_EFFORT`, else unknown | Effort level running the builder (OP15). |
| `--json` | no | — | off | JSON output. |

The required flags are required: a forgotten flag is an error, not a silent default (Task B 5). Pass a preference only when the user stated it (build request 5.4). `rex` writes `class_version = 1` itself.

#### Classification values in code

The values of the three classification flags are public `clap::ValueEnum` types in the library, in `src/store/class.rs` (C1 7.5):

| Flag | Type | Value names, exactly |
|---|---|---|
| `--type` | `super_rex::store::class::WorkType` | `feature` `fix` `refactor` `chore` `research` `other` |
| `--size` | `super_rex::store::class::Size` | `small` `medium` `large` |
| `--consequence` | `super_rex::store::class::Consequence` | `low` `medium` `high` |

The parser of `rex job create` uses these types, so they are the one list. A test in `tests/contract/` (Lane 6, C7 5) reads them with `ValueEnum::value_variants()` and `to_possible_value()`, and compares the names with the value lines of the `orchestration-builder` skill (Task B 5).

### 3.2 Input file

The request file is plain text: the request as given, stored as read.

```
Make the order API reject a quantity of zero with a typed error,
and make the web client show that error to the user.
```

### 3.3 What it does

1. Checks the flags and reads the request file.
2. In one `BEGIN IMMEDIATE` write: looks for the session's open job (`created` or `planned`). If there is one, writes nothing (OP4). Otherwise inserts the job with status `created` and its first `job_transition` row (`from_status` NULL, event `job create`).

Because the check and the insert are one write, two creates at the same moment in one session give one job and one `existing=yes`.

### 3.4 Output

| Key | Value |
|---|---|
| `job` | The job ID. |
| `status` | The job's stored status. `created` for a new job. |
| `existing` | `no` if this call opened the job; `yes` if the session already had an open job and that job is given (OP4). |

```
$ rex job create --session 4f9c2e1a-7b3d-4e8f-9a6c-1d2e3f4a5b6c --request-file request.txt \
    --asked-by human --merge yes --type fix --size medium --consequence high \
    --lang rust --lang typescript --budget low --autonomy high \
    --builder-model claude-opus-5-5 --builder-effort high
job=0g4cbf7k2q
status=created
existing=no
```

```
$ rex job create ... --json
{"schema":1,"job":"0g4cbf7k2q","status":"created","existing":false}
```

The session already has open job `0g4cbf7k2q`, and a sub-agent's builder calls create again. Exit 0:

```
$ rex job create --request-file sub-request.txt --asked-by agent --merge no --type chore --size small --consequence low --lang none
job=0g4cbf7k2q
status=planned
existing=yes
```

and on standard error:

```
note: session 4f9c2e1a-7b3d-4e8f-9a6c-1d2e3f4a5b6c already has open job 0g4cbf7k2q; no job was opened; this work is recorded under it
```

### 3.5 Errors

| Exit | Text | When |
|---|---|---|
| 2 | `error: no session ID; pass --session <id>, or run inside Claude Code, which sets CLAUDE_CODE_SESSION_ID` | Neither `--session` nor the variable. |
| 2 | `error: --lang none cannot be combined with other languages; give none alone, or only language names` | `none` with another name. |
| 2 | `error: --lang <name> is not a language name; use lower-case letters and digits only, such as cpp or csharp` | A name not of `[a-z0-9]+` after lowercasing. |
| 1 | `error: cannot read request file <path>: <system message>` | The file cannot be read. |
| 1 | `error: request file <path> is empty; write the request text into it` | Empty, or only spaces and newlines. |
| 1 | C1 8 | Store not found, not set up, and so on. |

### 3.6 Status given

`created` for a new job. With `existing=yes`, none: nothing is written.

## 4. `rex job plan`

Records the work breakdown, the skills, the orchestrator and its version, and the role table with each role's OMS record. Gives `planned`. Source: 5.1 step 7, 5.3, 5.9; Task A 5, 8.

```
rex job plan <job> --plan-file <path|-> [--json]
```

### 4.1 Flags

| Flag | Required | Values | Default | Meaning |
|---|---|---|---|---|
| `<job>` | yes | a job ID | — | The job to plan. Must be `created`. |
| `--plan-file` | yes | a path, or `-` | — | The plan, JSON (section 4.2). |
| `--json` | no | — | off | JSON output. |

### 4.2 Input file

```json
{
  "orchestrator": "lead-and-verifier",
  "skills": ["rex-code-tdd", "rex-code-philosophy", "rex-code-ergonomics"],
  "breakdown": "1. API: reject zero in the validator; typed error; test.\n2. Web client: show the error; test. Needs 1.\n3. Review both changes.",
  "roles": [
    {"role": "engineer", "intelligence": "high", "reliability": "high", "consequence": "high", "verification": "low"},
    {"role": "reviewer", "intelligence": "medium", "reliability": "medium", "consequence": "medium", "verification": "high"},
    {"role": "skill-writer", "session": true}
  ]
}
```

| Key | Required | Type | Meaning |
|---|---|---|---|
| `orchestrator` | yes | string, `[a-z0-9-]+` | The orchestrator chosen. The folder `.rex/orchestrators/<name>/` must exist. |
| `skills` | yes | array of strings | The skills selected for the job (5.3 stage 1). May be empty. Each name is non-empty with no spaces or commas. Stored sorted, without duplicates. |
| `breakdown` | yes | string | The work breakdown, Markdown (5.3 stage 2). Not empty. |
| `roles` | yes | array, at least one | The role table. |
| `roles[].role` | yes | string, `[a-z0-9-]+` | Role name, as the orchestrator skill names it. Each role once. |
| `roles[].session` | no | bool | `true`: the role runs at the session's model and effort; OMS is not asked (Task A 4.2). Then no input key may be given. `false` is the same as leaving it out. |
| `roles[].intelligence` | unless `session` | `low`, `medium`, `high`, `none` | OMS input. |
| `roles[].reliability` | unless `session` | same | OMS input (required reliability). |
| `roles[].consequence` | unless `session` | same | OMS input (failure consequence of this role). |
| `roles[].verification` | unless `session` | same | OMS input (ease of verification). |

A role is either `"session": true` with no inputs, or all four inputs. Duration, budget and autonomy are the job's, from `rex job create`; they are never in this file (Task A 3).

### 4.3 What it does

1. Takes in the event log (section 2.6). Checks the file.
2. In one write: indexes the library with `orchestrator::index_all` (C1 7.4, OP3); for each role not run as the session, calls `oms::select` with the job's duration, budget and autonomy and the role's four inputs (OP2); writes the role rows with inputs, agent, demand, rules, conflict and OMS version; writes `breakdown`, `skills`, `orchestrator_name` and `orchestrator_version` on the job; moves the status with `status::apply(Event::Plan)`.

### 4.4 Output

| Line | Form |
|---|---|
| 1 | `job=<id>` |
| 2 | `status=planned` |
| 3 | `orchestrator=<name>` |
| 4 | `orchestrator_version=<n>` |
| one per role, in file order | `role.<name>=<grid agent>`, or `role.<name>=session` |
| right after a role line, only when its conflict is not `none` | `conflict.<name>=<budget, duration or budget,duration>` |

The job was created with `--budget low --autonomy high`:

```
$ rex job plan 0g4cbf7k2q --plan-file plan.json
job=0g4cbf7k2q
status=planned
orchestrator=lead-and-verifier
orchestrator_version=1
role.engineer=model-medium-effort-high
conflict.engineer=budget
role.reviewer=model-medium-effort-medium
role.skill-writer=session
```

The engineer needed `model-xhigh-effort-max`; the low budget capped it, and the High-stakes floor kept its effort at high (Task A 7.1, case 23). `rex job show` prints the full record.

```
$ rex job plan 0g4cbf7k2q --plan-file plan.json --json
{"schema":1,"job":"0g4cbf7k2q","status":"planned","orchestrator":{"name":"lead-and-verifier","version":1},"roles":[{"role":"engineer","session":false,"agent":"model-medium-effort-high","demand":"model-xhigh-effort-max","rules":["T1","T2","E1","E2","E4","C1","C2","F1"],"conflict":["budget"],"oms_version":1},{"role":"reviewer","session":false,"agent":"model-medium-effort-medium","demand":"model-medium-effort-medium","rules":["T1","E1","E3","E4"],"conflict":[],"oms_version":1},{"role":"skill-writer","session":true,"agent":null,"demand":null,"rules":null,"conflict":null,"oms_version":null}]}
```

In JSON, a conflict of `none` is `[]`.

### 4.5 Errors

| Exit | Text | When |
|---|---|---|
| 1 | `error: cannot read plan file <path>: <system message>` | The file cannot be read. |
| 1 | `error: plan file <path> is not valid: <parser message>` | Not JSON, an unknown key, a missing key, a wrong type or input value. The parser message names the key, for example ``unknown field `model`, expected one of `role`, `session`, `intelligence`, `reliability`, `consequence`, `verification` at line 6 column 30``. |
| 1 | `error: plan file <path> has no roles; give at least one role` | `roles` is empty. |
| 1 | `error: plan file <path> names role <role> twice; give each role once` | A repeated role. |
| 1 | `error: role name <role> in plan file <path> is not valid; use lower-case letters, digits and -` | A role name not of `[a-z0-9-]+`. |
| 1 | `error: orchestrator name <name> in plan file <path> is not valid; use lower-case letters, digits and -` | An orchestrator name not of `[a-z0-9-]+`. |
| 1 | `error: skill name <name> in plan file <path> is not valid; a skill name is not empty and has no spaces or commas` | A bad skill name. |
| 1 | `error: plan file <path> has an empty breakdown; write the work breakdown into "breakdown"` | `breakdown` is empty. |
| 1 | `error: role <role> has "session": true and OMS inputs; give either "session": true or all four of intelligence, reliability, consequence and verification` | Both forms. |
| 1 | `error: role <role> lacks <key>; give all four of intelligence, reliability, consequence and verification (each low, medium, high or none), or "session": true` | Some inputs missing. |
| 1 | `error: orchestrator <name> not found at <store>/orchestrators/<name>; write the orchestrator into that folder first` | No such folder. |
| 1 | `error: job <job> is <status>, and rex job plan accepts only a created job; a job is planned once, and a builder that got existing=yes records no plan of its own` | The job is not `created` (no re-plan). |
| 1 | C1 8 | Job not found, store not found. |

### 4.6 Status given

`planned`.

## 5. `rex job close`

Records the outcome, with one delivery for each repository the job changed, and moves the job on. Gives `delivered`, `blocked`, `failed` or `cancelled`. Source: 5.1 step 11, 5.8, 5.9, 5.16; challenges 3 and 6; C12 6 and 9.

```
rex job close <job> --status delivered|blocked|failed|cancelled --outcome <one line>
              [--deliveries-file <path|->] [--json]
```

### 5.1 Flags

| Flag | Required | Values | Default | Meaning |
|---|---|---|---|---|
| `<job>` | yes | a job ID | — | The job to close. |
| `--status` | yes | `delivered`, `blocked`, `failed`, `cancelled` | — | The status to give (5.8). |
| `--outcome` | yes | one line of text | — | What happened, in one line. Replaces the job's outcome on each close. |
| `--deliveries-file` | no | a path, or `-` | no repository changed | One entry per repository the job changed (section 5.2). |
| `--json` | no | — | off | JSON output. |

With no `--deliveries-file`, no repository changed in this call: for example research with no changes to a repository (challenge 6).

### 5.2 Input file

A JSON array, one object per repository. Each object is one DELIVERY block of C12 6, with the same seven field names and outcome values. The job above, closed from `/home/ana/code/shop`, where the API merged and the sibling web client waits for a human's approval:

```json
[
  {"repo": "/home/ana/code/shop", "branch": "fix/zero-quantity-0g4cbf7k2q",
   "pr": "https://github.com/acme/shop/pull/41", "merge_agreed": true,
   "merge_commit": "3f2a9c1e8b7d6a5f4e3d2c1b0a9f8e7d6c5b4a39", "outcome": "merged", "note": null},
  {"repo": "/home/ana/code/web-client", "branch": "fix/zero-quantity-0g4cbf7k2q",
   "pr": "https://github.com/acme/web-client/pull/17", "merge_agreed": true, "outcome": "blocked",
   "note": "main of web-client is protected; pull request 17 waits for a human's approval. After it merges, record it with rex job close."}
]
```

| Key | Required | Type | Meaning |
|---|---|---|---|
| `repo` | yes | string | A path in the repository: C12 gives the absolute path of the repository root; a relative path is relative to the command's folder. Resolved with `StorePaths::locate` (C1 3.1) to that repository's main checkout and stored as an absolute path. Must be in a Git repository. Each repository once. |
| `branch` | no | string | The job branch there (5.16 step 2). |
| `pr` | for `pr-open` | string or null | The pull request URL. |
| `merge_agreed` | no | bool | Whether a merge was agreed for this repository. Default: the job's `--merge` answer (5.16: "applies to every repo unless the request says otherwise"). |
| `merge_commit` | for `merged` | string or null | The full hash of the squash commit on main. |
| `outcome` | yes | `merged`, `pr-open`, `kept-local`, `blocked` | How this repository's work ended (5.9, C12 6). |
| `note` | unless `merged` | string or null | One line: what remains to do in this repository. For `merged` it may say what clean-up kept, and why. |

C12's text form writes `none` for a value that is not there, and `yes`/`no` for `merge_agreed`. In this JSON file the skill writes `null`, or leaves the key out, and writes `merge_agreed` as `true` or `false`. The text `"none"` in `pr`, `merge_commit` or `note` is an error, so a missing value is never stored as the word.

Rules:

- `merged` needs `merge_agreed` true and a `merge_commit`.
- `pr-open` needs a `pr`.
- Any outcome but `merged` needs a `note`.
- `--status delivered` is refused while any delivery of the job is `blocked`, counting both the file and the rows kept from an earlier close.
- On a later close of a `blocked` job, each entry replaces the row of the same repository; repositories not named keep their rows.

Examples of challenge 6: a job with no remote and merge declined closes `delivered` with one `kept-local` entry and its `note`; research with no repository changes closes `delivered` with no file.

**C12 9, item by item.** `rex job close` accepts:

- 0 to n deliveries per job, each with the seven fields of C12 6 and their values: the file above, or no file.
- A `note` for every outcome except `merged`: the third rule above, and the `CHECK` on `delivery` (C1 6).
- The job statuses `delivered` and `blocked` as C12 6 sets them, and OP10 for mixed outcomes: any `blocked` delivery makes the job `blocked` (section 8, OP10), as C12 recommends, and every delivery is recorded.
- The job record keeps the merge answer, and `rex job show` prints it with the work type and the request text: `merge_agreed`, `type` and `request` in section 6.2.

### 5.3 What it does

1. Takes in the event log (section 2.6). Checks the flags and the file, and resolves each repository.
2. In one write: inserts or replaces each delivery by repository; checks the `delivered` rule over all the job's deliveries; writes `outcome`; moves the status with `status::apply(Event::Close(..))`. If any step fails, nothing is written.

### 5.4 Output

| Line | Form |
|---|---|
| 1 | `job=<id>` |
| 2 | `status=<the new stored status>` |
| 3 | `deliveries=<n>`: all the job's deliveries after this call, new and kept |
| next n | `delivery.<i>=<outcome> <repo>`, in the order the rows were first recorded, `i` from 1. The repository path takes the rest of the line. |

```
$ cd /home/ana/code/shop
$ rex job close 0g4cbf7k2q --status blocked --outcome "API merged; web client waits for approval of its pull request" --deliveries-file deliveries.json
job=0g4cbf7k2q
status=blocked
deliveries=2
delivery.1=merged /home/ana/code/shop
delivery.2=blocked /home/ana/code/web-client
```

Later, after a person approved and merged pull request 17, the session records the rest with a file of one entry, `[{"repo": "/home/ana/code/web-client", "branch": "fix/zero-quantity-0g4cbf7k2q", "pr": "https://github.com/acme/web-client/pull/17", "merge_agreed": true, "merge_commit": "9b1d4e7a2c3f5b6d8e9f0a1b2c3d4e5f6a7b8c90", "outcome": "merged"}]`:

```
$ rex job close 0g4cbf7k2q --status delivered --outcome "API and web client merged" --deliveries-file rest.json --json
{"schema":1,"job":"0g4cbf7k2q","status":"delivered","deliveries":[{"outcome":"merged","repo":"/home/ana/code/shop"},{"outcome":"merged","repo":"/home/ana/code/web-client"}]}
```

### 5.5 Errors

| Exit | Text | When |
|---|---|---|
| 2 | `error: --outcome must be one line; put the details in each delivery's "note"` | `--outcome` holds a newline. |
| 2 | `error: --outcome must not be empty; say in one line what happened` | Empty. |
| 1 | `error: cannot read deliveries file <path>: <system message>` | The file cannot be read. |
| 1 | `error: deliveries file <path> is not valid: <parser message>` | Not a JSON array of the form above. |
| 1 | `error: "<key>" of the delivery to <repo> in deliveries file <path> is the text none; write null or leave the key out` | `pr`, `merge_commit` or `note` is `"none"`. |
| 1 | `error: repo <given path> in deliveries file <path> is not in a Git repository; give a path inside the repository the job changed` | `locate` found no Git repository. |
| 1 | `error: deliveries file <path> names repo <root> twice; give one entry per repository` | Two entries resolve to one main checkout. |
| 1 | `error: delivery to <root> is merged but no merge was agreed for it; record it as pr-open or kept-local, or set "merge_agreed": true if the requester agreed` | `merged` with `merge_agreed` false. |
| 1 | `error: delivery to <root> is merged but has no merge_commit; add the full hash of the squash commit on main` | `merged` with no commit. |
| 1 | `error: delivery to <root> is pr-open but has no pr; add the pull request URL` | `pr-open` with no URL. |
| 1 | `error: delivery to <root> is <outcome> but has no note; add a "note" that says what remains to do` | No `note`. |
| 1 | `error: job <job> cannot be delivered while its delivery to <root> is blocked; close it as blocked, or record that repository's new outcome in the deliveries file` | `delivered` with a `blocked` delivery. |
| 1 | `error: job <job> is created, and rex job close --status delivered accepts only a planned or blocked job; record the plan with rex job plan first, or close it as blocked, failed or cancelled` | `delivered` on `created`. |
| 1 | `error: job <job> is <status>, and rex job close accepts only a created, planned or blocked job; a closed job is not closed again; open a new job for new work` | The job is `delivered`, `failed`, `cancelled`, `accepted`, `rejected` or `analyzed`. |
| 1 | C1 8 | Job not found, store not found. |

### 5.6 Status given

The `--status` value. Closing a `blocked` job as `blocked` again keeps the status, updates the deliveries and the outcome, and still writes one transition row (`blocked` to `blocked`, event `job close`), so every close is in the history.

## 6. `rex job show`

Prints one job's record. Source: 5.9, 5.10, 5.13; build plan 2.8.

```
rex job show [<job>] [--json]
```

### 6.1 Flags

| Flag | Required | Values | Default | Meaning |
|---|---|---|---|---|
| `<job>` | no | a job ID | the open job of `$CLAUDE_CODE_SESSION_ID` | The job to show. |
| `--json` | no | — | off | The full record as JSON. |

With no `<job>`: if the session has no open job, the command fails with `NoOpenJob` (C1 8). If `CLAUDE_CODE_SESSION_ID` is not set either, it is a usage error (section 6.4).

### 6.2 Plain output

In this order:

1. Scalar lines, one key each: `job`, `status` (the shown status, from `status::shown`), `stored_status`, `type`, `size`, `consequence`, `languages`, `duration`, `budget`, `autonomy`, `merge_agreed` (`yes` or `no`), `session`, `asked_by`, `builder_model`, `builder_effort`, `orchestrator`, `orchestrator_version`, `skills`, `created_at`, `updated_at`, `outcome`. A value not set prints `none`. `outcome` is last and takes the rest of the line; before the job is closed it prints empty (`outcome=`).
2. One line per role, in the order of the plan file: `role.<name>=<agent> inputs=<7 characters> demand=<pair> rules=<list> conflict=<list> oms_version=<n>`, or `role.<name>=session` for a role run as the session. `inputs` is the Task A 5 form: the seven OMS inputs in the order duration, budget, intelligence, autonomy, reliability, consequence, verification, each `L`, `M`, `H`, or `-` for None. `oms_version` is the OMS function version the role was chosen with. So `rex-feedback` reads the whole OMS record from plain output for its "were the inputs right" step (Task A 8).
3. One line per delivery, numbered as in `rex job close`: `delivery.<i>=<outcome> branch=<branch> pr=<url> commit=<hash> repo=<path>`. `repo` takes the rest of the line. The keys are C12 6's field names; `commit` is `merge_commit`.
4. One line per sub-agent run, by `first_seen_at` (depends on C5): `run.<agent_id>=<open|finished|unconfirmed> parent=<agent_id|main> role=<role> agent=<agent type> ran=<model>/<effort> tokens=<n> provisional=<yes|no>`. `tokens` is the sum of input, output, cache-read and cache-creation tokens. An unknown part prints `none`.
5. The fields that can hold several lines, last. Each is a `<key>:` line, then its text with every line indented by two spaces. The text ends at the first line that does not start with two spaces. The keys, in order: `request`, `breakdown`, `experience_recalled` (as stored, C9 3.6), then `delivery.<i>.note` for each delivery that has one. A field that is not set prints its `<key>:` line with no text after it.

`status=running` shows only when the stored status is `planned` and C5 reports at least one open run. It never shows with the hooks absent, because then there are no runs.

The job of sections 3 to 5, after its first close:

```
$ rex job show 0g4cbf7k2q
job=0g4cbf7k2q
status=blocked
stored_status=blocked
type=fix
size=medium
consequence=high
languages=rust,typescript
duration=none
budget=low
autonomy=high
merge_agreed=yes
session=4f9c2e1a-7b3d-4e8f-9a6c-1d2e3f4a5b6c
asked_by=human
builder_model=claude-opus-5-5
builder_effort=high
orchestrator=lead-and-verifier
orchestrator_version=1
skills=rex-code-ergonomics,rex-code-philosophy,rex-code-tdd
created_at=2026-11-10T09:14:03.512Z
updated_at=2026-11-10T10:41:27.880Z
outcome=API merged; web client waits for approval of its pull request
role.engineer=model-medium-effort-high inputs=-LHHHHL demand=model-xhigh-effort-max rules=T1,T2,E1,E2,E4,C1,C2,F1 conflict=budget oms_version=1
role.reviewer=model-medium-effort-medium inputs=-LMHMMH demand=model-medium-effort-medium rules=T1,E1,E3,E4 conflict=none oms_version=1
role.skill-writer=session
delivery.1=merged branch=fix/zero-quantity-0g4cbf7k2q pr=https://github.com/acme/shop/pull/41 commit=3f2a9c1e8b7d6a5f4e3d2c1b0a9f8e7d6c5b4a39 repo=/home/ana/code/shop
delivery.2=blocked branch=fix/zero-quantity-0g4cbf7k2q pr=https://github.com/acme/web-client/pull/17 commit=none repo=/home/ana/code/web-client
run.c4e8f1a2b3d5e6f7a=finished parent=main role=skill-writer agent=general-purpose ran=claude-opus-5-5/high tokens=88210 provisional=yes
run.a1b8ec258e0b550ba=finished parent=main role=engineer agent=model-medium-effort-high ran=claude-sonnet-5-5/high tokens=512340 provisional=yes
run.b7c2d9e04f1a6c3d8=finished parent=main role=reviewer agent=model-medium-effort-medium ran=claude-sonnet-5-5/medium tokens=143877 provisional=yes
request:
  Make the order API reject a quantity of zero with a typed error,
  and make the web client show that error to the user.
breakdown:
  1. API: reject zero in the validator; typed error; test.
  2. Web client: show the error; test. Needs 1.
  3. Review both changes.
experience_recalled:
  {"recalled_at":"2026-11-10T09:15:11.204Z","lessons":[{"feedback_id":3,"job":"0g30kh5n1w","score":3}],"orchestrators":[{"name":"lead-and-verifier","version":1,"best_match":3}],"unused":[]}
delivery.2.note:
  main of web-client is protected; pull request 17 waits for a human's approval. After it merges, record it with rex job close.
```

A planned job with one sub-agent open, from a worktree, with no job argument (first lines only):

```
$ cd /home/ana/code/shop/.claude/worktrees/agent-d3f0 && rex job show
job=0g4imj3x8d
status=running
stored_status=planned
...
```

### 6.3 `--json`

One line: the full record. The job above (shortened here only by `...` inside long texts):

```
$ rex job show 0g4cbf7k2q --json
{"schema":1,"job":{"id":"0g4cbf7k2q","status":"blocked","stored_status":"blocked","request":"Make the order API reject ...\n","asked_by":"human","class_version":1,"work_type":"fix","size":"medium","consequence":"high","languages":["rust","typescript"],"pref_duration":null,"pref_budget":"low","pref_autonomy":"high","merge_agreed":true,"session_id":"4f9c2e1a-7b3d-4e8f-9a6c-1d2e3f4a5b6c","builder_model":"claude-opus-5-5","builder_effort":"high","experience_recalled":"{\"recalled_at\":\"2026-11-10T09:15:11.204Z\", ...}","breakdown":"1. API: ...","orchestrator":{"name":"lead-and-verifier","version":1},"skills":["rex-code-ergonomics","rex-code-philosophy","rex-code-tdd"],"outcome":"API merged; web client waits for approval of its pull request","created_at":"2026-11-10T09:14:03.512Z","updated_at":"2026-11-10T10:41:27.880Z"},"transitions":[{"from":null,"to":"created","event":"job create","at":"2026-11-10T09:14:03.512Z"},{"from":"created","to":"planned","event":"job plan","at":"2026-11-10T09:21:40.017Z"},{"from":"planned","to":"blocked","event":"job close","at":"2026-11-10T10:41:27.880Z"}],"roles":[{"role":"engineer","session":false,"inputs":{"duration":null,"budget":"low","autonomy":"high","intelligence":"high","reliability":"high","consequence":"high","verification":"low"},"agent":"model-medium-effort-high","demand":"model-xhigh-effort-max","rules":["T1","T2","E1","E2","E4","C1","C2","F1"],"conflict":["budget"],"oms_version":1}, ...],"deliveries":[{"repo":"/home/ana/code/shop","branch":"fix/zero-quantity-0g4cbf7k2q","pr":"https://github.com/acme/shop/pull/41","merge_agreed":true,"merge_commit":"3f2a9c1e8b7d6a5f4e3d2c1b0a9f8e7d6c5b4a39","outcome":"merged","note":null,"recorded_at":"2026-11-10T10:41:27.880Z"}, ...],"runs":[{"agent_id":"a1b8ec258e0b550ba","state":"finished","parent_agent_id":null,"role":"engineer","agent_type":"model-medium-effort-high","model_ran":"claude-sonnet-5-5","effort_ran":"high","started_at":"2026-11-10T09:22:05.310Z","stopped_at":"2026-11-10T10:02:51.004Z","duration_ms":2445694,"tool_uses":87,"lines_added":214,"lines_removed":31,"tokens":{"input":20411,"output":30112,"cache_read":441207,"cache_creation":20610,"provisional":true}}, ...],"feedback":[]}
```

| Field | Type | Meaning | When null or empty |
|---|---|---|---|
| `schema` | number | Shape version. Always `1`. | never |
| `job` | object | Every `job` column (C1 5.4) under its column name, plus `status` (shown) and `stored_status`. `languages` and `skills` are arrays; `merge_agreed` is a bool; `orchestrator` is `{"name", "version"}`. `experience_recalled` is the stored text. | A column that is NULL is `null`; `orchestrator` is `null` before the plan. |
| `transitions` | array | One object per `job_transition` row, oldest first: `from`, `to`, `event`, `at`. | never empty (create writes one) |
| `roles` | array | One object per role: `role`, `session`, `inputs` (the seven OMS inputs, `null` for None), `agent`, `demand`, `rules` (array), `conflict` (array, `[]` for none), `oms_version`. For a session role all but `role`, `session` and `inputs` are `null`. | `[]` before the plan |
| `deliveries` | array | One object per delivery, in recorded order: every `delivery` column but `id` and `job_id`; `merge_agreed` is a bool. | `[]` when none |
| `runs` | array | One object per run, by `first_seen_at`: `agent_id`, `state` (`open`, `finished`, `unconfirmed`), `parent_agent_id` (`null` = main session), `role`, `agent_type`, `model_ran`, `effort_ran`, `started_at`, `stopped_at`, `duration_ms`, `tool_uses`, `lines_added`, `lines_removed`, `tokens` `{input, output, cache_read, cache_creation, provisional}`. Depends on C5. | `[]` when none, and until C5 is merged |
| `feedback` | array | One object per feedback row, oldest first: `id`, `given_by`, `said`, `verdict`, `causes` (array), `lesson`, `changed_skills` (array), `changed_orchestrators` (array), `created_at`. | `[]` when none |

### 6.4 Errors

| Exit | Text | When |
|---|---|---|
| 2 | `error: give a job ID; CLAUDE_CODE_SESSION_ID is not set, so there is no session to find the open job of` | No `<job>` and no variable. |
| 1 | `error: session <session> has no open job in <store>/rex.db; give the job ID, or run rex from the session's project folder` | No `<job>` and the session has no open job (C1 `NoOpenJob`). |
| 1 | `error: no job with ID <job> in <store>/rex.db; run rex from the session's project folder or a worktree of it, or set REX_STORE` | C1 `JobNotFound`. |
| 1 | C1 8 | Store not found. |

### 6.5 Status given

None. `rex job show` writes nothing except the intake.

## 7. The transition table

`status::apply` (C1 7.3) is the one place where these rules live. `rex job plan`, `rex job close` and `rex feedback` (Lane 7) call it; none of them has status rules of its own. Anything not in the table is refused: exit 1, the status stays, no row is written. The error names the job, its stored status, the event, what the event accepts, and the fix (texts in sections 4.5 and 5.5).

| Event | Stored status before | Result |
|---|---|---|
| `job create` | (no job) | `created` |
| `job plan` | `created` | `planned` |
| `job plan` | anything else | refused: a job is planned once |
| `job close` | `created` | `blocked`, `failed` or `cancelled`. `delivered` is refused: record the plan first. |
| `job close` | `planned` | `delivered`, `blocked`, `failed` or `cancelled` |
| `job close` | `blocked` (the rest recorded later) | `delivered`, `blocked`, `failed` or `cancelled` |
| `job close` | `delivered`, `failed`, `cancelled`, `accepted`, `rejected`, `analyzed` | refused: a closed job is not closed again |
| `feedback` | as C9 4.3, adopted unchanged | C9 4.3 gives every row, with `NoOutcomeYet` for `created` and `planned` and `AnalysisDue` for a `rejected` job without analysis. |

Further rules:

- A change writes one `job_transition` row per step. Every `rex job close` writes one row, even `blocked` to `blocked`. Only a feedback call that leaves the status as it was writes no row.
- A sub-agent started before the plan (for example one writing a skill) does not change the status (5.8; challenge 6).
- `running` is never stored. It is the shown status of a `planned` job with open runs (C1 7.3, `status::shown`).
- `rex job close --status delivered` is also refused while any delivery of the job is `blocked` (section 5.2). That check is in `rex job close`, because it reads deliveries, not only the status.

## 8. Open points

### OP2 — proposed

Question: does `rex job plan` take each role's inputs and work out the OMS result itself?

Answer: yes. The plan file gives each role's four inputs (or `"session": true`). `rex` calls Lane 6's `oms::select` for each role with the job's duration, budget and autonomy and the role's inputs, and stores the inputs, agent, demand, rules, conflict and version.

Reason: the function is pure (Task A 5), so the record cannot disagree with it, and no agent copies rule lists or versions by hand.

### OP3 — proposed (C2 part)

Question: who writes the orchestrator index, and how does an orchestrator get its version?

Answer: `rex job plan` writes the index, through `orchestrator::index_all`; no agent does. The version is a number rex gives when the folder's content changes, and `rex job plan` prints it as `orchestrator_version`. The description of a never-run orchestrator comes from its `SKILL.md` header, read at index time, so `rex recall` reads the store only. The full rule is C1 7.4 and C1 11.1.

Reason: rex records facts itself (5.9). A version an agent had to bump would drift. Task B 4 has `rex recall` list never-run orchestrators "by name and description".

### OP4 — proposed

Question: what does `rex job create` print in a session that already has an open job?

Answer: no job is opened. The output is `job=<the open job>`, `status=<its stored status>`, `existing=yes`, exit 0, and on standard error `note: session <s> already has open job <id>; no job was opened; this work is recorded under it`. A caller that gets `existing=yes` does NOT call `rex job plan` or `rex job close`: the job's opener owns the record. (Notice to Lane 9: the builder skill must branch on `existing`.)

Reason: 5.6 puts the work under the open job. Exit 0, because the caller can go on. The `existing` line tells the builder skill to skip its own plan and close. `rex job plan` accepts only `created`, so a nested builder that plans by mistake fails loudly instead of replacing the parent's plan.

### OP10 — proposed (the part outside feedback)

Question: which states each command accepts; how a `blocked` job goes on, and whether it is still the session's open job; the job status when one repository merged and another is blocked, and how the rest is recorded later; whether `rejected` and `analyzed` are one call or two.

Answer:

- Which states each command accepts: the table in section 7.
- A `blocked` job is not the session's open job (open means `created` or `planned`). It goes on by a later `rex job close` that records what happened (a person merged it, or it was dropped). New agent work after the block is a new job.
- A job that waits for the user's answer inside the same conversation stays open and is not closed as `blocked`. `blocked` is for a session that stops with the job waiting.
- One repository merged and another blocked: the job is `blocked`. Any `blocked` delivery makes the job `blocked`, as C12 6 recommends; `merged`, `pr-open` and `kept-local` all allow `delivered`. `rex job close` refuses `delivered` while any delivery is `blocked`. The rest is recorded later by `rex job close` on the blocked job; deliveries are replaced by `repo`, and those not named are kept, each with its `note`.
- No re-plan: `rex job plan` accepts `created` only.
- Feedback, which states accept it, and `rejected` against `analyzed`: C9 4.3 (Lane 7's part of OP10), adopted. C2 does not repeat it.

Reason: a blocked job may wait for days. The session must be free to open a new job, and new sub-agents must not be tied to a waiting job. Without re-plan, a nested builder cannot replace a parent's plan by mistake.

**FLAG for Lane 1 (1.7), to go to the owner:** this narrows 5.8's meaning of `blocked`, "waiting on a decision or on something only the user can give". Under this answer, a job waiting for the user's answer in the same conversation is not `blocked`; only a job the session stops on is. The owner should confirm the narrower meaning.

### OP15 — proposed

Question: how does `rex job create` learn the builder's model and effort?

Answer: `--builder-effort` defaults to `$CLAUDE_EFFORT` (seen in a Bash shell on Claude Code v2.1.322); the builder skill can also write `${CLAUDE_EFFORT}`, which check 7 confirmed. `--builder-model` is passed by the builder, since the model knows its own ID. When absent, the column is NULL, and Lane 3's intake may fill it from the main session transcript's `message.model` (notice to Lane 3).

Reason: the substitution and the variable are the only confirmed sources of the effort. No source gives the model in the environment.

## 9. Assumptions

Where the plan leaves a detail open, C2 chose as follows. Each can change in review.

| # | Assumption |
|---|---|
| A1 | The request file is stored exactly as read; an empty or whitespace-only file is an error. |
| A2 | `rex job create` checks all flags and reads the request file before it looks for an open job, so a call with `existing=yes` still fails on bad input. |
| A3 | `--session`, `--builder-model` and `--builder-effort` take any non-empty text; rex does not check them against a list. An empty variable counts as not set. |
| A4 | Language names that break the `[a-z0-9]+` form, `none` mixed with names, and a missing session ID are usage errors (exit 2). An `--outcome` with a newline or empty is a usage error too. |
| A5 | Preferences take no `none` value; leaving the flag out means not stated. |
| A6 | The plan file's `skills` key is required and may be empty. A skill name is any non-empty text with no spaces or commas, because plugin skills hold `:`. |
| A7 | `"session": false` is the same as leaving `session` out. |
| A8 | In `rex job plan` output, a `conflict.<name>` line comes right after its role line. |
| A9 | `delivery.<i>` numbers the job's deliveries in the order their rows were first recorded, in both `rex job close` and `rex job show`. |
| A10 | In `rex job show`, an `outcome` that is not set prints empty, and a several-line field that is not set prints only its `<key>:` line. |
| A11 | `tokens` in a `run.` line is the sum of all four token counts. |
| A12 | `experience_recalled` in `--json` is the stored text, not parsed JSON. |
| A13 | Errors are plain `error: ` lines with `--json` too. |
| A14 | The error for `delivered` with a blocked delivery is a `Refused` from `rex job close` itself. |
| A15 | A `<job>` argument that is not of the job ID form is a usage error (exit 2). |
| A16 | `rex job show` has no `--session` flag; with no `<job>` it uses `CLAUDE_CODE_SESSION_ID` only. |

## 10. Source map

| Source | Covered in |
|---|---|
| Build request 5.1 (the flow: create, plan, close; nested builder) | Sections 1, 3, 4, 5, OP4 |
| Build request 5.4 (OMS inputs; job and role) | Sections 3.1, 4.2, OP2 |
| Build request 5.6 (one open job per session; session ID) | Sections 3.3, 3.4, OP4 |
| Build request 5.8 (statuses, transitions, `running` shown only) | Sections 6.2, 7, OP10 |
| Build request 5.9 (records) | Sections 3, 4, 5, 6 |
| Build request 5.12 (store from a worktree) | Section 2.7 |
| Build request 5.16 (delivery per repository, merge agreed, blocked on a protected main) | Section 5, OP10 |
| Build plan 2.3 to 2.9 (the commands; one transition table) | Sections 3 to 7 |
| Build plan OP2, OP3, OP4, OP10, OP15 | Section 8 |
| Build plan OP13 (sibling repository) | Section 2.7; C1 11.2 |
| Challenge 2 (nested builder; one job per session) | Section 3.3, OP4 |
| Challenge 3 (two repositories, one merged, one blocked) | Sections 5.2, 5.4, OP10 |
| Challenge 6 (sub-agent before plan; hooks absent; research; no remote) | Sections 5.1, 5.2, 6.2, 7 |
| Task A 3, 4.2, 5, 7.1, 8 (OMS inputs, session roles, output, the OMS record) | Sections 4.2, 4.4, OP2 |
| Task B 1, 5 (the classification flags; required; `class_version`) | Section 3.1 |
| C9 draft 4.3, 4.6, 7 (feedback rows; IDs without `-`; shown status) | Sections 2.4, 7 |
| C12 draft 6 (the delivery result: `repo`, `branch`, `pr`, `merge_agreed`, `merge_commit`, `outcome`, `note`; job status from deliveries) | Sections 5.2, 5.4, 6.2, 6.3, OP10 |
| C12 draft 9 (what `rex job close` must accept) | Section 5.2 ("C12 9, item by item") |
| C7 draft 5 (the test that the builder lists the classification values) | Section 3.1 ("Classification values in code") |
