Status: DRAFT

# C1. The store: layout, finding it, schema version 1, and its code interface

This contract fixes where the SuperRex store lives, how `rex` finds it from any folder, what its tables hold, and the Rust calls other lanes use to open and write it.
Read by: Lanes 3, 4, 7 (and 9, 10 for the layout).

Lane 2 builds the store. C1 is frozen only after the review of the contract drafts. A reader should need no other file to open, read or write the store. The job commands that change a job are in C2.

## 1. Callers

| Caller | Uses | Why |
|---|---|---|
| Lane 2 (`rex job ...`, C2) | everything here | Opens, plans, closes and shows jobs. |
| Lane 3 (`rex hook`, intake, `rex status`, C3, C5) | the layout, `StorePaths::discover`, `Store::open`, `Store::write`, the `sub_agent_run` and `meta` tables, the upsert form (section 6.1) | The hook appends to the event log; intake writes runs. |
| Lane 4 (`rex init`, C6) | the layout, `StorePaths::locate`, `Store::create`, optionally `orchestrator::index_all` | Makes the store; writes the ignore entry for `.rex/`. |
| Lane 7 (`rex recall`, `rex feedback`, `rex jobs`, `rex watch`, C9) | `discover`, `open`, `write`, `status::apply`, `status::shown`, the `job`, `delivery`, `feedback` and `orchestrator` tables | Reads jobs, writes feedback and `experience_recalled`. |
| Lane 6 (contract test, C7 5) | `store::class` (section 7.5) | Compares the classification values with the builder skill. |
| Lanes 9 and 10 (skills) | the layout (section 2) | The orchestrator library folder; where the store is. |

## 2. Layout

Everything lives in one folder, `.rex/`, at the root of the repository's main checkout. A Git worktree has no `.rex/` of its own; `rex` finds the main checkout's folder (section 3).

| Path | What | Written by |
|---|---|---|
| `.rex/rex.db` | The SQLite database, schema version 1 (section 5). | `Store::create` makes it; the lanes write it through `Store::write`. |
| `.rex/rex.db-wal`, `.rex/rex.db-shm` | SQLite's own files in WAL mode. Never edit or delete them while `rex` runs. | SQLite |
| `.rex/events.jsonl` | The hook event log. One line per event. The line format is C3 (Lane 3). | `rex hook` appends; intake reads. |
| `.rex/orchestrators/<name>/` | The orchestrator library: one folder per orchestrator, with its `SKILL.md` and any lead skills. | Skills (Lanes 9, 10); `rex init` may seed it (Lane 4). |

Other folders under `.rex/` are for other contracts to name. For example C12 (Lane 10) may name `.rex/worktrees/`. `rex init` puts `.rex/` in the ignore entries (build request 5.12), so nothing in it goes into Git.

## 3. Finding the store

### 3.1 `StorePaths::locate(start)`: the main checkout root

`locate` finds the root of the main checkout that holds `start`. It reads files only. It never runs `git`, so `rex hook` stays within a few milliseconds (build request 5.6). It does not check that `.rex/` exists.

1. Make `start` absolute and resolve symbolic links. Walk up from it to the first folder that holds an entry named `.git`.
2. If `.git` is a folder, that folder is the root.
3. If `.git` is a file, it holds one line `gitdir: <path>`. A relative `<path>` is relative to the folder that holds the `.git` file.
   - If `<path>/commondir` exists, this is a linked worktree. The common Git folder is `<path>/<contents of commondir, trimmed>`, made absolute and resolved. If the last part of that folder's path is `.git`, the root is its parent. Otherwise `locate` fails with `UnsupportedLayout`. That happens for a worktree of a bare repository, of a repository made with `--separate-git-dir`, or of a submodule.
   - If there is no `commondir`, this is a submodule or a checkout made with `--separate-git-dir`. The root is the folder that holds the `.git` file.
4. If no folder up to `/` holds `.git`, `locate` fails with `NotInGitRepo`.

The paths it gives, all absolute:

| Field | Value |
|---|---|
| `root` | The main checkout root. |
| `dir` | `<root>/.rex` |
| `db` | `<dir>/rex.db` |
| `event_log` | `<dir>/events.jsonl` |
| `orchestrators` | `<dir>/orchestrators` |

`rex init` (Lane 4) and `rex job close` (to resolve each delivery's repository, C2) call `locate`. Every other command calls `discover`.

### 3.2 `StorePaths::discover()`: the store of this session

`discover` is used by every `rex` command except `rex init`, and by `rex hook`. It returns the first store that exists:

1. **`REX_STORE`.** If the environment variable `REX_STORE` is set, it is the path of a `.rex` folder, used as is (a relative path is relative to the current folder). `dir` is that folder, `root` is its parent. If `<REX_STORE>/rex.db` does not exist, `discover` fails with `NotFound`, naming `REX_STORE`. It never falls through to the other places: an explicit choice is never ignored.
2. **`CLAUDE_PROJECT_DIR`.** If set, `locate` on it. If `<root>/.rex/rex.db` exists, that store is used.
3. **The current folder.** `locate` on it. If `<root>/.rex/rex.db` exists, that store is used.
4. Otherwise `discover` fails with `NotFound`, listing each place it tried and why each did not give a store.

An empty variable counts as not set. A place where `locate` fails (for example the current folder is not in a Git repository) is skipped and listed in the `NotFound` error.

Why `CLAUDE_PROJECT_DIR` comes before the current folder: Claude Code sets it for hook commands, so a hook that fires while an agent works in a sibling repository still writes the session's store, even if that sibling has a store of its own. It was seen NOT set in a Bash tool shell (Claude Code v2.1.322), so a `rex` command run by an agent normally resolves by the current folder. That works from the main checkout, any sub-folder and any worktree. From a sibling repository it does not: see OP13 (section 11.2) and the caller rule in C2.

### 3.3 Worked example

```
/home/ana/code/shop/.git/                         main checkout's Git folder
/home/ana/code/shop/.rex/rex.db                   the store
/home/ana/code/shop/.claude/worktrees/agent-a1b8/.git    file: gitdir: /home/ana/code/shop/.git/worktrees/agent-a1b8
/home/ana/code/shop/.git/worktrees/agent-a1b8/commondir  file: ../..
```

From `/home/ana/code/shop/.claude/worktrees/agent-a1b8/src`, `locate` walks up to the worktree folder, reads its `.git` file, finds `commondir`, resolves `/home/ana/code/shop/.git/worktrees/agent-a1b8/../..` to `/home/ana/code/shop/.git`, and gives `root = /home/ana/code/shop`. `discover` then uses `/home/ana/code/shop/.rex/rex.db`.

## 4. Opening the store

### 4.1 The one rule

Other lanes open the store only through `Store::open` (or `Store::create` for `rex init`). Never open `rex.db` with a bare `rusqlite::Connection::open` or the `sqlite3` tool for writing. The reason: `foreign_keys` and `recursive_triggers` are settings of each connection, not of the file. A connection without them can break the rules of section 6 with no error. For reading by hand, `sqlite3 .rex/rex.db` is fine.

### 4.2 Connection settings

| Setting | Where | Why |
|---|---|---|
| `PRAGMA journal_mode = WAL` | Once, in `Store::create`. It is kept in the file. | Readers do not block the writer, and the writer does not block readers. |
| `PRAGMA foreign_keys = ON` | Every connection, in `create` and `open`. | Every `job_id` points at a real job. |
| `PRAGMA recursive_triggers = ON` | Every connection, in `create` and `open`. | Without it, the delete that `INSERT OR REPLACE` does fires no trigger, and a run could move to another job (section 6). |
| `PRAGMA busy_timeout = 10000` | Every connection, in `create` and `open`. | A writer waits up to 10 seconds for another writer. |
| `BEGIN IMMEDIATE` | Every write, in `Store::write`. | A deferred transaction that later upgrades to a writer can fail at once with "database is locked", without waiting. An immediate one takes the write lock first, so `busy_timeout` applies. |

Together: when several agents run `rex` at the same moment, a second writer waits and does not fail. A writer that waits more than 10 seconds gets a `Sqlite` error with SQLite's text `database is locked`.

`schema.sql` does not set these: they are connection settings, and the file sets only the schema and `user_version`.

### 4.3 Versions

`PRAGMA user_version` holds the schema version. Version 1 is the only one.

| `user_version` | `Store::open` | `Store::create` |
|---|---|---|
| 0 (no schema) | fails with `NotInitialised` (fix: `rex init`) | applies `schema.sql` in one transaction |
| 1 | opens | opens; changes nothing (safe to run twice) |
| more than 1 | fails with `NewerSchema` | fails with `NewerSchema` |

There is no migration code until a version 2 exists.

**A store already at version 1 is left as it was** (a need of C6 5 and 12, Lane 4). `Store::open` and `Store::create` on such a store write nothing to `.rex/`:

- No schema write, and no change of `user_version`.
- `create` sets `journal_mode = WAL` only when the file is not in WAL mode yet, so a second `rex init` changes nothing. `create` makes `.rex/` and `.rex/orchestrators/` only when they are absent.
- The `rex.db-wal` and `rex.db-shm` files exist only while a connection is open. SQLite removes them when the last connection closes cleanly, which is its default; `rex` does not turn that off (it never sets a persistent WAL). So after `rex init` runs a second time, `.rex/` holds the same files with the same content as before.

Checked on macOS with SQLite 3.51.3 (Python's copy): `-wal` and `-shm` were present while the connection was open and gone after it closed. A second open for a read left `rex.db` byte for byte the same. This holds for the SQLite that `rex` links in (the `bundled` build Lane 2 asks for).

Warning: Apple's own `/usr/bin/sqlite3` (3.43.2) keeps `-wal` and `-shm` after it closes; the same check left both files. A person who reads the store with that tool can leave them behind. That is harmless: the next `rex` open takes them in, and its clean close removes them. Tests of this rule must open the store through `rex`, never through `/usr/bin/sqlite3`. A connection that a crash ends can also leave the two files behind, and is cleaned up the same way.

## 5. Schema version 1

`src/store/schema.sql` is the one copy of the SQL. This section describes it in words; it does not repeat it. Every table is `STRICT`: a value of the wrong type is an error, not a silent conversion.

### 5.1 Times

Every time is TEXT, ISO 8601 UTC, always in the one form `YYYY-MM-DDTHH:MM:SS.sssZ` (milliseconds and `Z`), for example `2026-11-10T09:14:03.512Z`. Time columns end in `_at`. One form only, so that text order is time order and the first ten characters are the date.

| Need | SQL expression |
|---|---|
| Write the time now | `strftime('%Y-%m-%dT%H:%M:%fZ', 'now')` |
| Write a time from other text (transcripts, events) | `strftime('%Y-%m-%dT%H:%M:%fZ', :text)`. It turns `2026-11-10T10:14:03+01:00`, `2026-11-10T09:14:03Z` and `2026-11-10T09:14:03.123456Z` into the one form. It gives NULL for text that is not a time, so check for NULL before you store it. |
| Duration in milliseconds from `a` to `b` | `CAST(ROUND((julianday(b) - julianday(a)) * 86400000) AS INTEGER)` |
| Date part (UTC) | `substr(x, 1, 10)`, for example `substr(created_at, 1, 10) BETWEEN :since AND :until` |

Rust needs no date code: every time is written by SQLite inside the INSERT or UPDATE. Every UPDATE of a `job` row also sets `updated_at` to now, whichever lane writes it.

### 5.2 Lists and empty values

- A list is stored as one TEXT value: the items sorted, without duplicates, joined by commas with no spaces, for example `go,rust`.
- Two lists keep their own order instead: `role.rules` and `role.conflict` keep the order that OMS gives them (Task A 2.2, 5).
- NULL means "not given". An empty list is NULL. Task B's "empty means None" is NULL here.
- Two exceptions write a word instead of NULL: `job.languages` is `none` for a job that writes no code (Task B 1.4), and `role.conflict` is `none` when OMS reports no conflict (Task A 5).

### 5.3 The value-list rule

Lists whose values the build request fixes carry a `CHECK` in the schema: the job status, the delivery outcome, and the 0/1 flags. Lists meant to grow carry no `CHECK`: work type, size, consequence, languages, preferences, OMS inputs, grid agent names, feedback causes and verdict, who asked, transition event names. For those, a Rust enum checks the value at the command line, so a new value needs no migration (Task B 5).

### 5.4 Table `job`

One row per job. Written by `rex job create`, `plan` and `close` (C2); `status` only through `status::apply` (section 7.3); `experience_recalled` by `rex recall` (C9 3.6).

| Column | Type | Null | Meaning | Source |
|---|---|---|---|---|
| `id` | TEXT | no | Primary key. Job ID: 10 characters of `[0-9a-z]`, no `-` (C2 2.4). | 5.9; build plan 2.4 |
| `request` | TEXT | no | The request text, as given. | 5.9 |
| `asked_by` | TEXT | no | Who asked: `human` or `agent`. | 5.9; Task B 3 |
| `class_version` | INTEGER | no | Version of the classification scheme. `1`. | Task B 5 |
| `work_type` | TEXT | no | `feature`, `fix`, `refactor`, `chore`, `research`, `other`. | Task B 1.1, 5 |
| `size` | TEXT | no | `small`, `medium`, `large`. | Task B 1.2, 5 |
| `consequence` | TEXT | no | `low`, `medium`, `high`. | Task B 1.3, 5 |
| `languages` | TEXT | no | Sorted list, such as `rust,typescript`, or `none`. | Task B 1.4, 5 |
| `pref_duration` | TEXT | yes | The user's stated duration: `low`, `medium`, `high`. NULL = not stated. | 5.9; Task B 2, 5 |
| `pref_budget` | TEXT | yes | Same, for budget. | 5.9; Task B 2, 5 |
| `pref_autonomy` | TEXT | yes | Same, for autonomy. | 5.9; Task B 2, 5 |
| `merge_agreed` | INTEGER | no | 1 if a merge to main was agreed, 0 if not. | 5.9; 5.16 step 1 |
| `session_id` | TEXT | no | The Claude Code session ID. | 5.6; 5.9 |
| `builder_model` | TEXT | yes | Model ID that ran the builder. NULL = unknown (OP15 in C2). | 5.9 |
| `builder_effort` | TEXT | yes | Effort level that ran the builder. NULL = unknown. | 5.9 |
| `experience_recalled` | TEXT | yes | JSON written by `rex recall` (C9 3.6). NULL = recall not run. | 5.9; C9 3.6 (OP8) |
| `breakdown` | TEXT | yes | The work breakdown, Markdown. NULL until planned. | 5.9 |
| `orchestrator_name` | TEXT | yes | Orchestrator used. NULL until planned. With `orchestrator_version`, a foreign key to `orchestrator`. | 5.9 |
| `orchestrator_version` | INTEGER | yes | Its version in the index. NULL until planned. | 5.9 (OP3) |
| `skills` | TEXT | yes | Skills selected for the job, sorted list. NULL until planned, or none chosen. | 5.9; 5.3 stage 1 |
| `status` | TEXT | no | Stored status, one of the nine: `created`, `planned`, `blocked`, `failed`, `delivered`, `accepted`, `rejected`, `analyzed`, `cancelled`. Never `running`. | 5.8 |
| `outcome` | TEXT | yes | One line from `rex job close`. NULL until closed. | 5.9 |
| `created_at` | TEXT | no | When the job was opened. | 5.9 |
| `updated_at` | TEXT | no | When the row last changed. | 5.9 |

Indexes: `job_one_open_per_session`, unique on `session_id` where `status` is `created` or `planned` (section 6). `job_created_at` on `created_at`, for lists by date.

### 5.5 Table `job_transition`

One row per change of stored status. Gives the timestamps of 5.9 for each status. Written only by `rex job create` (its first row) and `status::apply`.

| Column | Type | Null | Meaning | Source |
|---|---|---|---|---|
| `id` | INTEGER | no | Primary key, rising. | — |
| `job_id` | TEXT | no | The job. Foreign key to `job`. | 5.9 |
| `from_status` | TEXT | yes | Status before. NULL for the row of `rex job create`. Else one of the nine. | 5.8 |
| `to_status` | TEXT | no | Status after. One of the nine. | 5.8 |
| `event` | TEXT | no | What caused it: `job create`, `job plan`, `job close`, `feedback`. | 5.8 |
| `at` | TEXT | no | When. | 5.9 |

Every `rex job close` writes one row, even when the status stays (`blocked` closed as `blocked` again), so every close is in the history; with each delivery's `recorded_at`, the record shows what each close changed. Only a feedback call that leaves the status as it was writes no row. A feedback call that goes from `delivered` or `accepted` through `rejected` to `analyzed` writes two rows, with the same `at`.

### 5.6 Table `role`

One row per role of a job, written by `rex job plan` (C2). Holds the OMS record of Task A 8.

| Column | Type | Null | Meaning | Source |
|---|---|---|---|---|
| `job_id` | TEXT | no | The job. Foreign key to `job`. Primary key with `role`. | 5.9 |
| `role` | TEXT | no | Role name, `[a-z0-9-]+`, such as `engineer`. | 5.5; 5.9 |
| `runs_as_session` | INTEGER | no | 1 if the role runs at the session's model and effort (OMS not asked, Task A 4.2); 0 if OMS chose its agent. | Task A 4.2 |
| `in_duration` | TEXT | yes | OMS input, copied from `job.pref_duration`: `low`, `medium`, `high`; NULL = None. | Task A 3, 8 |
| `in_budget` | TEXT | yes | Same, from `job.pref_budget`. | Task A 3, 8 |
| `in_autonomy` | TEXT | yes | Same, from `job.pref_autonomy`. | Task A 3, 8 |
| `in_intelligence` | TEXT | yes | OMS input for the role. NULL = None. | Task A 3, 8 |
| `in_reliability` | TEXT | yes | Same. | Task A 3, 8 |
| `in_consequence` | TEXT | yes | Same. | Task A 3, 8 |
| `in_verification` | TEXT | yes | Same (ease of verification). | Task A 3, 8 |
| `agent` | TEXT | yes | Grid agent chosen, such as `model-medium-effort-high`. NULL exactly when `runs_as_session` is 1. | 5.9; Task A 5 |
| `demand` | TEXT | yes | The pair needed before caps. Set whenever `agent` is. | 5.9; Task A 5 |
| `rules` | TEXT | yes | Rules that fired, such as `T1,T2,E1,C1`, in OMS order. Set whenever `agent` is. | 5.9; Task A 2.2, 5 |
| `conflict` | TEXT | yes | `none`, `budget`, `duration` or `budget,duration`. Set whenever `agent` is. | 5.9; Task A 4.3, 5 |
| `oms_version` | INTEGER | yes | OMS function version. Set whenever `agent` is. | 5.9; Task A 8 |

### 5.7 Table `delivery`

One row per repository a job changed, written by `rex job close` (C2). A job that changed no repository has no rows (challenge 6). The column names and outcome values are those of the delivery result in C12 6 (Lane 10).

| Column | Type | Null | Meaning | Source |
|---|---|---|---|---|
| `id` | INTEGER | no | Primary key, rising. | — |
| `job_id` | TEXT | no | The job. Foreign key to `job`. | 5.9 |
| `repo` | TEXT | no | Absolute path of that repository's main checkout. Unique with `job_id`. | 5.9; 5.16; challenge 3; C12 6 |
| `branch` | TEXT | yes | The job branch in that repository. | 5.9; 5.16 step 2; C12 6 |
| `pr` | TEXT | yes | URL of the pull request. NULL = none. | 5.9; 5.16 step 5; C12 6 |
| `merge_agreed` | INTEGER | no | 1 if a merge was agreed for this repository, 0 if not. | 5.9; 5.16; C12 6 |
| `merge_commit` | TEXT | yes | The full hash of the squash commit on main. NULL = none. | 5.9; 5.16 step 6; C12 6 |
| `outcome` | TEXT | no | `merged`, `pr-open`, `kept-local` or `blocked`. | 5.9; C12 6 |
| `note` | TEXT | yes | One line: what remains to do. Required unless `outcome` is `merged`. For `merged` it may say what clean-up kept, and why. | 5.16 (more than one repository); challenge 3; C12 6 |
| `recorded_at` | TEXT | no | When this row was last written. | 5.9 |

### 5.8 Table `orchestrator`

The orchestrator index: one row per version of each orchestrator in the library. Written only by `orchestrator::index` (section 7.4, OP3).

| Column | Type | Null | Meaning | Source |
|---|---|---|---|---|
| `name` | TEXT | no | Folder name under `.rex/orchestrators/`. Primary key with `version`. | 5.5; 5.9 |
| `version` | INTEGER | no | 1, 2, 3 ... A new number each time the folder's content changes. | 5.9 (OP3) |
| `content_hash` | TEXT | no | Hash of the folder's content, 16 lower-case hex characters (section 7.4). | OP3 |
| `description` | TEXT | yes | The `description:` of `<name>/SKILL.md`'s header at index time. Empty text if absent. | Task B 4; C9 3.4 |
| `indexed_at` | TEXT | no | When this version was first indexed. | — |

### 5.9 Table `sub_agent_run`

One row per sub-agent. Written by Lane 3's intake (C5). Whether a run is open, finished or unconfirmed is worked out by C5 from these columns; it is not stored.

| Column | Type | Null | Meaning | Source |
|---|---|---|---|---|
| `id` | INTEGER | no | Primary key, rising. | — |
| `job_id` | TEXT | no | The job the run is tied to, the first time it is seen. Never changes (section 6). Foreign key to `job`. | 5.6 rule 2; challenge 2 |
| `session_id` | TEXT | no | The session ID from the hook event. | 5.6 |
| `agent_id` | TEXT | no | The sub-agent's ID. Unique with `session_id`. | 5.6; 5.9 |
| `parent_agent_id` | TEXT | yes | The parent sub-agent's ID. NULL = the main session. | 5.6; 5.9 |
| `depth` | INTEGER | yes | Nesting depth, as the transcript's metadata file gives it. | 5.6 |
| `role` | TEXT | yes | Role from the brief's tag lines (C4). | 5.5; 5.9 |
| `agent_type` | TEXT | yes | Grid agent chosen (the sub-agent type of the start event). | 5.6; 5.9 |
| `model_ran` | TEXT | yes | Model that actually ran. | 5.6; 5.9 |
| `effort_ran` | TEXT | yes | Effort that actually ran. | 5.6; 5.9 |
| `brief` | TEXT | yes | The brief. | 5.6; 5.9 |
| `skills_opened` | TEXT | yes | Skills the run opened, sorted list. | 5.6; 5.9 |
| `background` | INTEGER | yes | 1 for a background sub-agent, 0 for a foreground one. NULL = unknown. | 5.6 |
| `first_seen_at` | TEXT | no | When intake first saw the run (when the tie was made). | 5.6 |
| `started_at` | TEXT | yes | Start time. | 5.9 |
| `stopped_at` | TEXT | yes | End time. | 5.9 |
| `duration_ms` | INTEGER | yes | Duration from the completed-call event. | 5.6 |
| `tool_uses` | INTEGER | yes | Tool count. | 5.9 |
| `lines_added` | INTEGER | yes | Lines added. | 5.9 |
| `lines_removed` | INTEGER | yes | Lines removed. | 5.9 |
| `final_message` | TEXT | yes | Final message. | 5.9 |
| `transcript_path` | TEXT | yes | Path of the run's transcript file. | 5.6 |
| `tokens_input` | INTEGER | yes | Input tokens, summed from the transcript. | 5.6; 5.9 |
| `tokens_output` | INTEGER | yes | Output tokens. | 5.6; 5.9 |
| `tokens_cache_read` | INTEGER | yes | Cache-read tokens. | 5.6; 5.9 |
| `tokens_cache_creation` | INTEGER | yes | Cache-creation tokens. | 5.6; 5.9 |
| `tokens_provisional` | INTEGER | no | 1 (the default) while the totals may still grow; 0 once final. | 5.6 |
| `transcript_bytes` | INTEGER | yes | Transcript size when the tokens were last summed, to see that it is unchanged. | 5.6 |

Every run needs a job: `job_id` is required. What intake does with a run first seen while its session has no open job is C5's to settle (Lane 3). See section 13.

### 5.10 Table `feedback`

One row per `rex feedback` call. Written by Lane 7 (C9 4). Columns as C9 7 asks.

| Column | Type | Null | Meaning | Source |
|---|---|---|---|---|
| `id` | INTEGER | no | Primary key, rising. | C9 7 |
| `job_id` | TEXT | no | The job. Foreign key to `job`. | 5.9 |
| `given_by` | TEXT | no | Who gave it. | 5.9; C9 4.1 |
| `said` | TEXT | no | What they said. | 5.9 |
| `verdict` | TEXT | no | `accepted` or `rejected`. | 5.8; C9 4.1 |
| `causes` | TEXT | yes | Root causes, sorted list: `human-error`, `missing-context`, `agent-skill`, `agent-model-effort`, `agent-orchestration`. | 5.8; C9 4.1 |
| `lesson` | TEXT | yes | The lesson. | 5.9; 5.18 |
| `changed_skills` | TEXT | yes | Skills the analysis changed, sorted list. | 5.9; C9 4.1 |
| `changed_orchestrators` | TEXT | yes | Orchestrators the analysis changed, sorted list. | C9 4.1 |
| `created_at` | TEXT | no | When it was given. | C9 7 |

### 5.11 Table `meta`

Small key and value pairs.

| Column | Type | Null | Meaning | Source |
|---|---|---|---|---|
| `key` | TEXT | no | Primary key. Each key belongs to the lane that writes it. | — |
| `value` | TEXT | no | The value. | — |

Reserved prefix: `intake.` for Lane 3 (for example the event log offset). A lane that needs a key takes a prefix of its own and names it in its contract.

### 5.12 Who writes what

| Table or column | Only writer |
|---|---|
| `job` (insert, plan and close fields) | Lane 2, `rex job create`, `plan`, `close` |
| `job.status`, `job_transition` | `status::apply` (and `rex job create` for the first row) |
| `job.experience_recalled` | Lane 7, `rex recall` |
| `role` | Lane 2, `rex job plan` |
| `delivery` | Lane 2, `rex job close` |
| `orchestrator` | `orchestrator::index` |
| `sub_agent_run` | Lane 3, intake |
| `feedback` | Lane 7, `rex feedback` |
| `meta` | the lane that owns the key's prefix |

Every lane may read every table.

## 6. Invariants and where each is enforced

Lanes 3 and 7 write SQL directly, so the rules other lanes could break by mistake are in the schema, not only in Rust.

| Invariant | Enforced by | Source |
|---|---|---|
| `running` is never stored. | `CHECK` on `job.status`, `job_transition.from_status` and `to_status`: the nine stored statuses only. | 5.8 |
| A session has at most one open job (`created` or `planned`). | Unique partial index `job_one_open_per_session`. `rex job create` also checks and inserts in one `BEGIN IMMEDIATE` write (C2). | 5.6 rule 1 |
| A sub-agent keeps the job it was first tied to. | Trigger `sub_agent_run_keeps_its_job` aborts any UPDATE that changes `job_id`. Trigger `sub_agent_run_is_never_deleted` aborts every DELETE, so `INSERT OR REPLACE` (which deletes, then inserts) aborts too. Needs `recursive_triggers = ON` (section 4.2). | 5.6 rule 2; challenge 2 |
| Every run, role, delivery, transition and feedback row belongs to a real job. | `job_id NOT NULL` with foreign keys; `foreign_keys = ON`. | 5.9 |
| A job names an orchestrator version that is in the index. | Foreign key (`orchestrator_name`, `orchestrator_version`). | 5.9 |
| A role run as the session has no OMS record; a role with an agent has the full one. | `CHECK`s on `role`. | Task A 4.2, 8 |
| A `merged` delivery had a merge agreed and has its merge commit. | `CHECK` on `delivery`. | 5.16 step 6 |
| A delivery that is not `merged` says what remains (`note`). | `CHECK` on `delivery`. | 5.16; challenge 3; C12 6 |
| One delivery per repository per job. | `UNIQUE (job_id, repo)`. | challenge 3 |
| A `pr-open` delivery has its pull request URL (`pr`). | Rust, `rex job close` (C2). | 5.16 |
| The stored status changes only by the table in C2 7. | Rust, `status::apply`. No lane runs its own `UPDATE job SET status`. | 5.8 |
| Classification, preference and cause values are from their lists. | Rust enums at the command line. No `CHECK` (section 5.3). | Task B 5 |

### 6.1 The upsert form for sub-agent runs (Lane 3)

Intake writes a run with this form, and never with `INSERT OR REPLACE`:

```sql
INSERT INTO sub_agent_run (job_id, session_id, agent_id, first_seen_at, stopped_at, effort_ran)
VALUES (:job_id, :session_id, :agent_id, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
        strftime('%Y-%m-%dT%H:%M:%fZ', :stopped_at), :effort_ran)
ON CONFLICT(session_id, agent_id) DO UPDATE SET
    stopped_at = COALESCE(excluded.stopped_at, stopped_at),
    effort_ran = COALESCE(excluded.effort_ran, effort_ran);
```

- The column list is an example; take the columns the event gives.
- Do not put `job_id` or `first_seen_at` in the `DO UPDATE SET` list. On a conflict the run keeps its first job, whatever `:job_id` the late event would give. That is the delayed-event case of challenge 2, and it is what the rule asks.
- `INSERT OR REPLACE` aborts with the trigger's message. It would also wipe every column the event does not give.

## 7. Code interface

Module `src/store/`. These signatures are the contract; the bodies are built after the freeze.

### 7.1 Paths

```rust
pub struct StorePaths {
    pub root: PathBuf,          // main checkout root
    pub dir: PathBuf,           // <root>/.rex, or REX_STORE
    pub db: PathBuf,            // <dir>/rex.db
    pub event_log: PathBuf,     // <dir>/events.jsonl
    pub orchestrators: PathBuf, // <dir>/orchestrators
}
impl StorePaths {
    pub fn locate(start: &Path) -> Result<StorePaths, StoreError>;
    pub fn discover() -> Result<StorePaths, StoreError>;
}
```

- `locate`: the main checkout of `start`, by the rules of section 3.1. Does not check that `.rex/` exists.
- `discover`: the store of this session, by the rules of section 3.2. `.rex/rex.db` must exist. `rex hook` calls it on every event (and opens nothing).

### 7.2 Store

```rust
pub struct Store { /* rusqlite::Connection + StorePaths */ }
impl Store {
    pub fn create(paths: &StorePaths) -> Result<Store, StoreError>;
    pub fn open(paths: &StorePaths) -> Result<Store, StoreError>;
    pub fn paths(&self) -> &StorePaths;
    pub fn conn(&self) -> &rusqlite::Connection;
    pub fn write<T, E: From<StoreError>>(
        &mut self,
        f: impl FnOnce(&rusqlite::Transaction<'_>) -> Result<T, E>,
    ) -> Result<T, E>;
    pub fn open_job_for_session(&self, session_id: &str) -> Result<Option<JobId>, StoreError>;
}
impl From<rusqlite::Error> for StoreError { /* Sqlite, with the db path */ }

pub struct JobId(String); // Display, FromStr (checks the 10-character form), as_str()
```

- `create`: for `rex init`. Makes `.rex/` and `.rex/orchestrators/` if absent, sets WAL if not set yet, applies `schema.sql` if `user_version` is 0. On a store at version 1 it writes nothing (section 4.3).
- `open`: opens an existing `rex.db` and sets the connection settings. Never makes a file. Fails on a missing file, version 0 or a newer version.
- `paths`: the paths it was opened with, for error texts and the event log.
- `conn`: for reads. Plain SELECTs; no writes through it.
- `write`: runs `f` in a `BEGIN IMMEDIATE` transaction. Commits if `f` returns `Ok`, rolls back if it returns `Err`. `E` can be the caller's own error type, as long as it converts from `StoreError`, so a closure can use `?` on rusqlite calls and on its own errors.
- `open_job_for_session`: the job of this session whose status is `created` or `planned`, if any.
- `JobId`: a checked job ID. `"0g4cbf7k2q".parse::<JobId>()` succeeds; anything not of 10 characters of `[0-9a-z]` fails with `InvalidInput`.

### 7.3 Status (`store::status`)

```rust
pub enum Status { Created, Planned, Blocked, Failed, Delivered, Accepted, Rejected, Analyzed, Cancelled }
pub enum Closed { Delivered, Blocked, Failed, Cancelled }
pub enum Verdict { Accepted, Rejected }
pub enum Event { Plan, Close(Closed), Feedback { verdict: Verdict, has_analysis: bool } }
pub struct Change { pub previous: Status, pub current: Status }

pub fn apply(tx: &rusqlite::Transaction<'_>, job: &JobId, event: Event) -> Result<Change, StoreError>;

pub enum Shown { Stored(Status), Running }
impl Shown { pub fn of(stored: Status, open_runs: u32) -> Shown; }
pub fn shown(conn: &rusqlite::Connection, job: &JobId) -> Result<Shown, StoreError>;
```

- `Status`: the nine stored statuses. `Display` and `FromStr` use the lower-case names. There is no `Running`.
- `Closed`: the statuses `rex job close` can give.
- `Verdict`: the two feedback verdicts.
- `Event`: what happened to the job. `rex job create` is not an event: it inserts the row with `created` and its first transition row itself. `has_analysis` is true when the feedback has at least one cause and a lesson (C9 4.3).
- `Change`: the stored status before and after. `previous == current` when the status stays: a feedback call that leaves it, or a `blocked` job closed as `blocked` again.
- `apply`: the one place where a job's stored status changes after create. It reads the stored status, checks the transition table (C2 7), writes `job.status`, `job.updated_at` and one `job_transition` row per step (two for `rejected` then `analyzed`). A close always writes one row, even `blocked` to `blocked`. Only a feedback event that leaves the status writes no row and changes nothing. A transition not in the table fails with `Refused`, `NoOutcomeYet` or `AnalysisDue`, and writes nothing. Called inside the caller's `Store::write`, so the caller's own row (a delivery, a feedback row) and the status change are one transaction. Used by `rex job plan`, `rex job close` and `rex feedback` (Lane 7).
- `Shown`: the status a person or agent sees. `Running` stands for `running` (5.8).
- `Shown::of`: `Running` if and only if `stored` is `Planned` and `open_runs` is more than 0; otherwise `Stored(stored)`.
- `shown`: the one shown-status function for `rex job show`, `rex jobs` and `rex watch` (C9 7). It reads the stored status and the open-run count from C5 (Lane 3). Until C5 is merged the count is 0, so `running` never shows; it never shows with the hooks absent either.

### 7.4 Orchestrator index (`store::orchestrator`)

```rust
pub struct OrchestratorVersion { pub name: String, pub version: i64, pub description: String }

pub fn index(tx: &rusqlite::Transaction<'_>, library: &Path, name: &str) -> Result<OrchestratorVersion, StoreError>;
pub fn index_all(tx: &rusqlite::Transaction<'_>, library: &Path) -> Result<Vec<OrchestratorVersion>, StoreError>;
```

- `OrchestratorVersion`: one row of the index, as the caller needs it.
- `index`: indexes one folder `<library>/<name>/` and returns its current version. It reads every regular file under that folder, walking sub-folders, skipping symbolic links, including hidden files, sorted by relative path with `/` separators. The hash is FNV-1a 64-bit over, for each file in turn: the relative path bytes, one 0 byte, the content length as 8 bytes little-endian, the content bytes. It is written as 16 lower-case hex characters. If the hash equals that of the newest row of that name, that row is returned. Otherwise a new row is inserted with version = newest + 1 (1 for the first) and `description` = the `description:` value of the YAML header of `<name>/SKILL.md` (the C7 header form), empty if absent.
- `index_all`: `index` for every folder in `library`, sorted by name.
- FNV-1a is ten lines, the same in every Rust release (std's `DefaultHasher` is not), and needs no dependency. It detects change; it is not for security.
- `rex job plan` calls `index_all` (C2). `rex init` may call it after seeding the library (notice to Lane 4). `rex recall` only reads the table (C9 3.4).
- An orchestrator file edited after a job (for example by `rex-feedback`, Lane 10) gets a new version at the next `index` or `index_all`, because its content hash changes. No agent bumps a version. The job keeps the version it ran with: `job.orchestrator_version` is never rewritten.
- Proposal to Lane 7 (not a need): `rex feedback` may call `index` for each `--changed-orchestrator`, so that the new version is in the index at once, not only at the next `rex job plan`.

### 7.5 Classification values (`store::class`)

`store::class::WorkType`, `store::class::Size` and `store::class::Consequence` (file `src/store/class.rs`) are public `clap::ValueEnum` types. They give the values of `rex job create --type`, `--size` and `--consequence`. The exact value names are in C2 3.1. A contract test (Lane 6, C7 5) reads them with `ValueEnum::value_variants()` and `to_possible_value()`.

## 8. Errors

`StoreError` is a `thiserror` enum in `src/store/error.rs`. It joins the crate's error the way C11 (Lane 1) gives. Each text says what failed, then the fix. `rex` prints it as one line starting `error: ` and exits 1 (C11 may change the style; the facts in each text stay).

| Variant | Text | Given by |
|---|---|---|
| `NotInGitRepo { start }` | `no Git repository found at or above <start>; run rex inside the repository where SuperRex is set up, or a worktree of it` | `locate` |
| `UnsupportedLayout { folder, common_dir }` | `cannot find the main checkout of <folder>: its Git folder <common_dir> is not named .git (a worktree of a bare repository, of a repository made with --separate-git-dir, or of a submodule); set REX_STORE to the .rex folder of the main checkout` | `locate` |
| `NotFound { tried }`, with `REX_STORE` | `REX_STORE is set to <path>, but <path>/rex.db does not exist; point REX_STORE at the .rex folder of the main checkout, or unset it` | `discover` |
| `NotFound { tried }`, otherwise | `no SuperRex store found; tried <place> (<reason>), <place> (<reason>); run rex from the repository where SuperRex is set up or a worktree of it, run rex init there, or set REX_STORE to its .rex folder` | `discover`, `open` |
| `NotInitialised { db }` | `<db> has no SuperRex schema; run rex init in <root>` | `open` |
| `NewerSchema { db, found, supported }` | `<db> has schema version <found>, but this rex supports version <supported>; install a newer rex` | `open`, `create` |
| `Sqlite { path, source }` | `SQLite failed on <path>: <SQLite's message>`. A lock that outlasts 10 seconds gives the message `database is locked`; run the command again. | any call |
| `Io { path, source }` | `cannot use <path>: <system message>` | `create`, `index`, file reads |
| `JobNotFound { job, store }` | `no job with ID <job> in <store>/rex.db; run rex from the session's project folder or a worktree of it, or set REX_STORE` | `apply`, `shown`, C2 commands |
| `NoOpenJob { session, store }` | `session <session> has no open job in <store>/rex.db; give the job ID, or run rex from the session's project folder` | `rex job show` (C2) |
| `Refused { job, status, event, accepts }` | `job <job> is <status>, and <event> accepts only <accepts>; <fix>`. Each case and its fix are in C2. | `apply`, `rex job close` |
| `NoOutcomeYet { job, status }` | `job <job> is <status> and has no outcome yet; close it with rex job close first` (C9 4.5) | `apply` (feedback) |
| `AnalysisDue { job }` | `job <job> is rejected and its analysis is due; give --cause and --lesson with --verdict rejected` (C9 4.5) | `apply` (feedback) |
| `InvalidInput { what }` | `<what>`: the exact texts are in C2. | C2 commands, `JobId::from_str` |

`<place>` in `NotFound` is a path with where it came from, for example `/home/ana/code/web/.rex/rex.db (current folder)`. `<reason>` is `no rex.db` or the `locate` error, for example `not inside a Git repository`.

Worked example: an agent runs `rex job show` in a sibling repository that has no store.

```
$ cd /home/ana/code/web-client && rex job show 0g4cbf7k2q
error: no SuperRex store found; tried /home/ana/code/web-client/.rex/rex.db (current folder, no rex.db); run rex from the repository where SuperRex is set up or a worktree of it, run rex init there, or set REX_STORE to its .rex folder
```

## 9. Risk: WAL in the sandbox

WAL mode uses a shared memory file (`rex.db-shm`). It has not yet been tested inside the Claude Code sandbox. Lane 2's test of a second writer runs in WAL mode, and gate P0.6 runs live. If WAL fails in the sandbox, `Store::create` falls back to `journal_mode = DELETE`. The writer still waits, because `busy_timeout` and `BEGIN IMMEDIATE` do not depend on WAL; readers then wait for the writer too. Callers see no change in the interface. The fallback keeps the rule of section 4.3: in `DELETE` mode the `rex.db-journal` file exists only during a write and is removed when it ends, so opening a store at version 1 still leaves `.rex/` as it was.

## 10. Where C1 differs from the C9 draft

C1 adopts C9's feedback columns, its `experience_recalled` column, ISO time text, the orchestrator `description`, and one shown-status function. These points still differ; each is a change asked of C9 before the freeze.

| C9 now | C1 | Why |
|---|---|---|
| Examples show `"asked_by":"user"` (C9 5.2). | `asked_by` is `human` or `agent`. | Task B 3 names "human or agent". C9's example should change to `human`. (`given_by` on feedback is free text; C9's default `user` there is fine.) |
| Times shown as `2026-11-10T09:14:03Z` (C9 2, 3.6, 5.2). | Always `2026-11-10T09:14:03.512Z`, with milliseconds. | One form, so text order is time order. C9's rules (first ten characters are the date, print as stored) hold unchanged; only the examples change. |
| Job IDs in examples have 8 characters (`m4q7t2c9`). | 10 characters (`0g4cbf7k2q`), C2 2.4. | Examples only. "No `-`" holds. |
| C9 7 names a `set_status` beside the transition function. | One function, `status::apply(tx, job, Event::Feedback { verdict, has_analysis })`, returns the change. There is no `set_status`. | One place for the rules; `rex feedback` gives the event and gets `previous` and `status` for its output (C9 4.4) from the returned `Change`. |
| `merge_agreed` is `null` "if the record has no answer" (C9 5.2, A14). | `merge_agreed` is never NULL: `rex job create` requires `--merge yes|no`. | 5.5: every orchestrator settles it. C9 can drop the null case. |
| Delivery `repo` shown as a short name (`widgets`) (C9 5.2). | The column is `delivery.repo`, as C9 and C12 name it, but it holds the absolute path of the main checkout (C12 6). | Two sibling repositories can share a folder name. C9 may print the last path part, but should say so. |
| "Unknown job" text `no job with ID <job>` (C9 2). | `JobNotFound` adds the store path and the fix (section 8). | A wrong store must be visible at once (OP13). The C9 text is the start of C1's. |

## 11. Open points

### 11.1 OP3 — proposed

Question: who writes the orchestrator index, how does an orchestrator get its version, and where does a never-run orchestrator's description come from?

Answer: `rex job plan` writes the index through `orchestrator::index_all`; no agent writes it. The version is a number rex gives, 1 for the first index of a folder and one more each time the folder's content hash changes; rex prints it as `orchestrator_version` (C2). The description is the `description:` of the folder's `SKILL.md` header, read at index time and stored on the row, so `rex recall` reads the store only (C9 3.4). `rex init` may call `index_all` after it seeds the library. An orchestrator edited after a job, for example by `rex-feedback`, gets a new version at the next index, because its hash changes; the job keeps the version it ran with. `rex feedback` (Lane 7) may call `orchestrator::index` for each `--changed-orchestrator`, so that the new version is in the index at once (a proposal to Lane 7, not a need).

Reason: rex records facts itself (5.9). A version an agent had to bump by hand would drift from the files. `rex job plan` already reads the library, so indexing there costs nothing extra, and it reaches orchestrators that never ran, which Task B 4 asks recall to list "by name and description".

### 11.2 OP13 — proposed

Question: when the working folder is a sibling repository, how does a hook or a `rex` command find the job's store?

Answer: the discovery rule of section 3.2 (`REX_STORE`, then `CLAUDE_PROJECT_DIR`, then the current folder), plus the caller rule in C2: run `rex job ...` from the session's project folder or a worktree of it; from a sibling repository, set `REX_STORE=<project>/.rex`. Every `JobNotFound` and `NoOpenJob` error prints the store path it used, so a wrong store shows at once.

Reason: the hook always knows the session's project through `CLAUDE_PROJECT_DIR`. A command in a worktree or a sub-folder resolves by the current folder. A shell in a sibling repository has an explicit override that never falls through.

Proposal to Lane 4 (a notice; it needs a live check, on 9.1 or P0.6): `rex init` writes `"env": {"REX_STORE": "<absolute root>/.rex"}` into `.claude/settings.local.json`, so that every Bash command and hook of a session started in the repository finds the store from any folder. Lane 2 does not depend on it.

## 12. Assumptions

Where the plan leaves a detail open, C1 chose as follows. Each can change in review.

| # | Assumption |
|---|---|
| A1 | With `REX_STORE` set, `root` is the parent of that folder. |
| A2 | An empty environment variable counts as not set. A relative `REX_STORE` is relative to the current folder. |
| A3 | `discover` fails only with `NotFound` (or `Io`); a `locate` failure on one place is a reason inside that error, not an error of its own. |
| A4 | `Store::open` on a missing `rex.db` fails with `NotFound` naming that path, and never makes a file. |
| A5 | An empty list is stored as NULL, except `languages` (`none`) and `role.conflict` (`none`). |
| A6 | Every UPDATE of a `job` row sets `updated_at`, whichever lane writes it, including `experience_recalled`. |
| A7 | Two `job_transition` rows of one feedback call share one `at`. |
| A8 | `PRIMARY KEY` columns are also declared `NOT NULL`, so that no key can be NULL. |
| A9 | `job_transition.event` and `feedback.verdict` carry no `CHECK`: the value-list rule in the plan names only status, delivery outcome and the flags. |

## 13. Requests before the freeze

A nullable column added before the freeze costs nothing. After it, it needs a version 2. So:

- **Lane 3** may ask for any column it finds missing in `sub_agent_run` or `meta`, or a change of a meaning above (for example of `depth` or `background`), from its C3 and C5 drafts. Main-session tokens (build plan OP12) would need a place too, if C5 records them.
- **Lane 7** may ask for any column it finds missing in `job`, `delivery`, `feedback` or `orchestrator`, and should answer the points of section 10.
- **The open seam on runs first seen with no open job.** `sub_agent_run.job_id` is required. A run first seen while its session has no open job (a sub-agent the user starts outside the builder, or an event read after the job closed and before a new one opened) cannot be stored as it is. C5 settles what intake does: skip it, hold it, or ask C1 for a nullable `job_id`. A nullable `job_id` would need a rule for when it is set. The keep-its-job trigger would change with it, because today it also aborts a change from NULL to a job.

## 14. Source map

| Source | Covered in |
|---|---|
| Build request 5.6 (hooks; one open job per session; a run keeps its job; the hook stays fast) | Sections 3, 5.9, 6, 6.1, 7.1 |
| Build request 5.8 (statuses; `running` not stored; changes only through rex commands) | Sections 5.4, 5.5, 6, 7.3 |
| Build request 5.9 (records) | Section 5 (every table) |
| Build request 5.12 (storage; store found from any worktree) | Sections 2, 3 |
| Build request 5.16 (delivery per repository, merge agreed, remaining work) | Sections 5.7, 6 |
| Build plan 2.1, 2.2 (schema; store core; writer waits) | Sections 4, 5, 6, 7 |
| Build plan OP3 (orchestrator index) | Sections 5.8, 7.4, 11.1 |
| Build plan OP13 (store from a sibling repository) | Sections 3.2, 8, 11.2 |
| Challenge 2 (a run's tie to its job is kept) | Sections 5.9, 6, 6.1 |
| Challenge 3 (one delivery per repository) | Sections 5.7, 6 |
| Challenge 6 (a job with no repository changes has no delivery) | Section 5.7 |
| Check 14 (worktrees; hooks map a worktree back to the main repository) | Sections 3.1, 3.3 |
| Task B 1.4, 5 (classification columns; no value constraints; languages sorted) | Sections 5.2, 5.3, 5.4 |
| Task A 5, 8 (OMS record per role) | Section 5.6 |
| C9 draft 3.4, 3.6, 4.3, 7 (feedback columns, `experience_recalled`, ISO times, shown status) | Sections 5.1, 5.4, 5.10, 7.3, 10 |
| C12 draft 6, 9 (delivery fields `repo`, `pr`, `note`; outcome values with hyphens) | Sections 5.7, 6, 10 |
| C6 draft 5, 12 (a second `rex init` changes no file in `.rex/`) | Sections 4.3, 7.2, 9 |
| C7 draft 5 (the classification values test) | Section 7.5 |
