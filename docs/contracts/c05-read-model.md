Status: FROZEN (10 November 2026, by Lane 1 at gate G1, on top of commit fe951b3). A change to this file goes through Lane 1.

# C5. The read model: runs, their state and tokens, and `rex status`

- Written by: Lane 3 (Capture).
- Read by: Lane 2 (`rex job plan`, `rex job close`, `rex job show`, and C1), Lane 7 (`rex jobs`, `rex watch`), Lane 10.
- Version: draft 1, 10 November 2026.
- Tested on: Claude Code 2.1.322, macOS.

## What this is for

Hooks append one line for each sub-agent event to the **event log**, `.rex/events.jsonl` (C3). `take_in` takes the new lines into the **store** (the SQLite database `.rex/rex.db`, C1). `job_runs` calls `take_in`, finds sub-agents that left no event, and reads their **transcripts** (the files where Claude Code writes each conversation). Callers get the runs of one job as they stand now. This contract fixes those functions, the rules behind what they return, and the output of `rex status`.

Words used here:

- **Run**: one sub-agent, from its start to its stop. Key: the session ID and the agent ID. Stored as one row of C1's table `sub_agent_run`.
- **Intake**: taking the new lines of the event log into the store.
- **Tie**: the job a run belongs to. Written once.
- **State**: `open`, `finished` or `unconfirmed`. Worked out at each read, never stored.
- **Provisional**: a token total that may still grow. Shown with `~`.
- **Incomplete**: the job's history may miss facts; one flag on the result.
- **Metadata file**: `agent-<id>.meta.json`, which Claude Code writes beside each sub-agent transcript.

Marks: "tested" = seen on Claude Code 2.1.322 by a live probe or in Lane 3's recorded fixtures; "settled" = a decision of this contract. `Store` and `JobId` are C1's (`super_rex::store`). `CaptureError` is capture's error type (section 1); it converts from C1's `StoreError` and from `rusqlite::Error`, and its form follows C11. Field names are C1's column names on purpose.

## 1. The interface

In `src/capture/mod.rs`:

```rust
/// Takes the new lines of the event log into the store. It replaces the stub
/// `take_in_events` of C11 2. Reads no transcript, except the first record of one
/// for a run whose start line was lost (the tie time, section 3).
pub fn take_in(store: &mut Store) -> Result<(), CaptureError>;

/// Calls `take_in`, finds runs that left no event, reads transcripts, stores totals,
/// and returns the runs of one job as they stand at `now`.
/// Fails with `CaptureError::Store(StoreError::JobNotFound)` for a job not in the store.
pub fn job_runs(store: &mut Store, job: &JobId, now: SystemTime) -> Result<JobRuns, CaptureError>;

/// Reads one transcript file. `Ok(None)` = no file there. Knows nothing about the store or jobs.
pub fn read_transcript(path: &Path) -> Result<Option<TranscriptFacts>, CaptureError>;

/// Reads the main session's transcript: the tokens of the requests in [from, to)
/// (times as the store holds them; `to` = None while the job is open), and the
/// model and effort of the `rex job create` call that printed `job=<job>` (section 4).
pub fn read_main_transcript(path: &Path, job: &JobId, from: &str, to: Option<&str>)
    -> Result<Option<MainFacts>, CaptureError>;

pub struct JobRuns {
    pub runs: Vec<Run>,             // by `started_at`, then `agent_id`; the tree is in `parent_agent_id`
    pub open: u32,
    pub finished: u32,
    pub unconfirmed: u32,
    pub tokens: TokenTotal,         // every run plus the main session
    pub session_tokens: TokenTotal, // the main session alone, inside the job's time
    pub incomplete: bool,           // section 7
    pub damaged_log_lines: u64,
}
pub struct Run {
    pub agent_id: String,
    pub parent_agent_id: Option<String>, // None = the main session
    pub depth: Option<u32>,              // 1 = started by the main session
    pub role: Option<String>,            // from Rex-Role
    pub tag_job: Option<String>,         // from Rex-Job, only when it differs from the tie
    pub agent_type: Option<String>,      // the agent type that ran; a grid agent or a built-in one
    pub model_ran: Option<String>,
    pub effort_ran: Option<String>,
    pub background: Option<bool>,
    pub state: RunState,
    pub started_at: Option<String>,      // text, `YYYY-MM-DDTHH:MM:SS.sssZ`
    pub stopped_at: Option<String>,
    pub duration_ms: Option<u64>,        // the completed call, else stop minus start (section 2)
    pub tool_uses: Option<u64>,
    pub lines_added: Option<u64>,
    pub lines_removed: Option<u64>,
    pub brief: Option<String>,
    pub final_message: Option<String>,
    pub skills_opened: Vec<String>,      // sorted; empty = none seen, or no transcript read (NULL in the store)
    pub tokens: TokenTotal,
}
pub enum RunState { Open, Finished, Unconfirmed }
pub enum TokenTotal { Unknown, Provisional(TokenCounts), Final(TokenCounts) }
pub struct TokenCounts { pub input: u64, pub output: u64, pub cache_read: u64, pub cache_creation: u64 }

pub struct TranscriptFacts {
    pub tokens: TokenCounts,             // each message ID counted once, all models added up
    pub requests: u64,                   // count of distinct message IDs
    pub tool_uses: u64,
    pub lines_added: u64,
    pub lines_removed: u64,
    pub first_at: Option<String>,        // `timestamp` of the first and last record that has one, as written
    pub last_at: Option<String>,
    pub first_user_text: Option<String>, // the brief, for a sub-agent transcript
    pub last_assistant_text: Option<String>,
    pub model: Option<String>,           // of the last assistant record
    pub effort: Option<String>,
    pub skills_opened: Vec<String>,      // sorted, each name one time
    pub damaged_lines: u64,              // lines that are not valid JSON, a cut last line included
    pub bytes: u64,                      // size of the file that was read
}
pub struct MainFacts {
    pub tokens: TokenCounts,             // the requests inside the window
    pub damaged_lines: u64,              // with records whose `timestamp` is not of the store's form
    pub bytes: u64,
    pub builder_model: Option<String>,
    pub builder_effort: Option<String>,
}

#[derive(thiserror::Error, Debug)]
pub enum CaptureError {
    #[error("failed to read transcript {}", path.display())]
    ReadTranscript { path: PathBuf, #[source] source: std::io::Error },
    #[error("failed to read the event log {}", path.display())]
    ReadLog { path: PathBuf, #[source] source: std::io::Error },
    #[error(transparent)]
    Store(#[from] StoreError),
}
impl From<rusqlite::Error> for CaptureError { /* CaptureError::Store(StoreError::from(e)) */ }
impl CommandError for CaptureError {} // never a usage error: exit 1
```

- `TotalCheck`, `Reading`, `RunActivity`, `Step` and `SETTLE` are capture's own. No other lane uses them.
- **JSON form of a total** (C2 6.3, C9 5.2): `TokenTotal::Unknown` is `null`; otherwise `{"input":n,"output":n,"cache_read":n,"cache_creation":n,"provisional":bool}`. In plain text a total is the sum of the four counts, or `none`.

Who calls what:

| Command | Calls |
| --- | --- |
| `rex job plan` | `take_in` |
| `rex job close`, `rex job show`, `rex status`, `rex jobs`, `rex watch` | `job_runs` |
| `rex job create`, `rex hook`, `rex recall`, `rex feedback`, all others | nothing |

- `job_runs` takes in the log itself, so no caller needs a separate step before it.
- `rex job close` keeps C2's order: it calls `job_runs` first, then writes the close. So the read inside `close` is before the close time and can never make a total final (section 8). The next read after the close stores the totals again, and one more read at least `SETTLE` later makes them final.
- The shown status: callers use C1's `Shown::of(stored, open_runs)` (C1 7.3), with `open_runs` = `JobRuns.open`. `running` = stored status `planned` and `open` > 0. The store imports nothing from capture.
- With the hooks absent the log is empty: `job_runs` returns no runs, `open` is 0, and `running` is never shown.

Cost: `take_in` reads the log from the saved offset to its end, and the first record of a transcript only for a stop line with nothing before it. `job_runs` also reads, for each run of the job whose total is not final, the size of its transcript, and parses the transcript only when the size changed or a read after the close is due (section 8); the main session's transcript, IF the job's session figure is not final; and the `subagents/` folder of the job's session, for recovery (section 5). IF every total of the job is final, THEN `job_runs` reads no transcript and does no recovery for that job. This bounds the cost of `rex jobs` and `rex watch`.

**Failures.** `take_in` and `job_runs` return `Err` only when the store fails (`CaptureError::Store`) or the event log exists and cannot be read (`CaptureError::ReadLog`). A transcript, metadata file or `subagents/` folder that exists but cannot be read is treated as absent for that read, and sets `incomplete` (section 7). So a hidden or removed file never stops `rex job close`, `rex job show`, `rex status` or a view.

## 2. Intake

- The store: `rex hook` and every `rex` command find it with the same `StorePaths::discover(cwd)` (C1, OP13): `REX_STORE`, then `CLAUDE_PROJECT_DIR`, then the working folder. So the lines of the hook and the jobs of a session are in one store. Intake reads the log at the store's `event_log` path.
- `take_in` first reads `intake.offset` with a plain SELECT and compares it with the size of the log. IF they are equal, THEN it returns and takes no lock. ELSE it begins one `Store::write` (a `BEGIN IMMEDIATE` transaction), reads `intake.offset` again inside it, reads the log from that byte offset to the last `\n`, applies each line, saves the new offset and commits. So two commands at the same moment never take in, or count, one line two times, and a read with nothing new writes nothing.
- Saved values, in C1's table `meta`: `intake.offset` (bytes), `intake.damaged_lines` (a count) and `intake.transcript.<session_id>` (the main session's transcript path, section 8).
- A line that is not valid JSON, has no `v`, or has a `v` that is not `1`, is counted as damaged and skipped. Bytes after the last `\n` wait for the next intake. IF the file is shorter than the offset, THEN intake starts again at 0.
- A `PostToolUse` line counts only when `tool_name` is exactly `Agent` or `Task`.
- Each line becomes an upsert on one run, in C1's form: `INSERT … ON CONFLICT(session_id, agent_id) DO UPDATE SET …`. Never `INSERT OR REPLACE`. Never `job_id` or `first_seen_at` in the update list. An upsert sets fields; it never adds to a counter.
- The order of sources is fixed for each column, as section 4 gives it, so that the stored run is the same whatever order its lines come in, and a line taken in twice changes nothing. A line of the column's first source writes it (`x = COALESCE(excluded.x, x)`). A line of a later source writes it only while it is NULL (`x = COALESCE(x, excluded.x)`). Examples: `final_message` is the stop line's `last_assistant_message`, also when the completed call comes after it; `stopped_at` is the stop line's `at_ms`, also when the completed call comes after it.
- `duration_ms` is stored only from a completed call. With no completed call it stays NULL, and `Run.duration_ms` is `stopped_at` minus `started_at`, worked out at the read.
- Second sources from `job_runs` (transcript, metadata file) write only while the column is NULL, with three exceptions: `tool_uses`, `lines_added` and `lines_removed` are written at each parse while `duration_ms` is NULL (a background run, or a lost completed call); `skills_opened` and the tokens are written at each parse; `final_message` from the transcript is written only once the run is finished.
- Times: the store holds every time as text `YYYY-MM-DDTHH:MM:SS.sssZ` (C1 5.1). Intake converts `at_ms` in SQL: `strftime('%Y-%m-%dT%H:%M:%fZ', :at_ms / 1000.0, 'unixepoch')`. A transcript `timestamp` is stored through `strftime('%Y-%m-%dT%H:%M:%fZ', :text)`, and a NULL result is not stored. A Rust time (`now`, a file's modification time) goes into SQL as milliseconds since the epoch, and a stored time comes back to Rust the same way (C1 5.1). Rust does no calendar arithmetic. Stored times are compared as text in SQL. The one comparison in Rust is the main-session window (section 8): a record whose `timestamp` is not exactly 24 bytes of the form `YYYY-MM-DDTHH:MM:SS.sssZ` counts as a damaged line there.

What each line sets on the run (C1 column names):

| Line | Run it names | Columns set |
| --- | --- | --- |
| `SubagentStart` | `agent_id` | `agent_type`, `started_at` (`at_ms`); on insert `job_id` (the tie) and `first_seen_at` |
| `PostToolUse`, tool `Agent`, `tool_response.status` = `completed` | `tool_response.agentId` | `parent_agent_id` = the event's `agent_id` (absent = main session), `brief`, `role`, `tag_job`, `model_ran` (`resolvedModel`), `duration_ms` (`totalDurationMs`), `tool_uses` (`totalToolUseCount`), `lines_added` and `lines_removed` (`toolStats`), `final_message` (text of `content`), `background` = 0, `stopped_at` (`at_ms`) |
| `PostToolUse`, tool `Agent`, `status` = `async_launched` | `tool_response.agentId` | `parent_agent_id`, `brief`, `role`, `tag_job`, `model_ran`, `background` = 1 |
| `SubagentStop` | `agent_id` | `effort_ran` (`effort.level`), `final_message` (`last_assistant_message`), `transcript_path` (`agent_transcript_path`), `stopped_at` (`at_ms`) |
| `PostToolUse`, tool `Agent`, any other `status` | `tool_response.agentId`, IF present | `parent_agent_id`, `brief`, `role`, `tag_job`. `stopped_at` is NOT set. |
| any other event or tool | none | ignored |

The tags are the two lines at the start of a brief: `Rex-Job: <job ID>` and `Rex-Role: <role>` (format in C4). The brief is `tool_input.prompt`.

**Transcript path.** `agent_transcript_path` of the stop line. Before a stop line, intake derives it: `<folder of transcript_path>/<session_id>/subagents/agent-<agent_id>.jsonl`, where `transcript_path` is the main session's transcript that every event carries.

## 3. The tie

**When a job is open.** From `job.created_at` until the `at` of its first `job_transition` row whose `to_status` is not `created` or `planned`. That `at` is the job's close time. A job never goes back to an open status (C2 7). `blocked` (closed, waiting on something outside the session) ends the window: a `blocked` job is not an open job. A question to the user in the same conversation leaves the job `created` or `planned` (C2 7, OP10).

**Rule 1 (first seen).** When a line names a run that has no row, its job = the job of that `session_id` that was open at the run's tie time. A session has at most one open job (C1's unique index), so the rule finds at most one.

**Tie time** = the best estimate of when the run started, from the line that creates the row:

1. `SubagentStart`: its `at_ms`;
2. `async_launched`: its `at_ms` (it fires at launch, tested);
3. a completed call: `at_ms` minus `tool_response.totalDurationMs`;
4. a stop line with nothing before it: the time of the first record of `agent_transcript_path` (one line read);
5. otherwise: the line's `at_ms`.

For a run found by recovery: the time of the first record of its transcript. Reason: when the start line was lost, the first line seen comes from the END of the run, and the run may have ended after its job closed.

**Rule 2 (kept).** The tie is written once. A later line for that run never changes it, also after the job closed and another opened. C1's trigger aborts any change of `job_id`.

**A run with no job is NOT stored.** C1's `sub_agent_run.job_id` is required. For a line that names a run with no row: IF its session has no job at all, THEN the line is skipped. ELSE intake works out the tie time and looks for the job of that session that was open then; none = the line is skipped. Skipped lines are not damaged. This still obeys "a run keeps the job it was first seen under": the tie time is the run's START, so every later line for that run gives the same answer, and a run first seen with no open job stays with no job, with no row. This is C5's answer to the seam of C1 section 13, which C1 now holds as settled (section 11).

Limit: source 5 (the line's own time) can tie a run to a later job when the start line was lost AND no duration and no transcript are there.

The `Rex-Job` tag never moves a run. IF it differs from the tied job, THEN the run keeps its tie and `tag_job` holds the tag.

## 4. The run record

| Field of `Run` | First source | Second source (`job_runs`, when the first is absent) |
| --- | --- | --- |
| `agent_id` | the event's `agent_id`, or `tool_response.agentId` | the name of the metadata file |
| `parent_agent_id` | `agent_id` of the `Agent` call's event (absent = main session) | metadata `parentAgentId` (absent at depth 1) |
| `depth` | metadata `spawnDepth` | 1 + the parent's depth |
| `role`, `tag_job` | the tag block of `tool_input.prompt` (`tag_job` only when it differs from the tie) | the first user record of the transcript |
| `agent_type` | `agent_type` of the start or stop event | metadata `agentType` |
| `model_ran` | `tool_response.resolvedModel` | `message.model` of the transcript's last assistant record |
| `effort_ran` | `effort.level` of the stop event | `effort` of the transcript's assistant records |
| `background` | `tool_input.run_in_background`, or status `async_launched` | metadata `requestShape` = `background` |
| `state` | section 6 | |
| `started_at` | `at_ms` of the start event | the tie-time estimate (section 3) |
| `stopped_at` | `at_ms` of the stop event, else of the completed call | metadata `finished: true`; the time is the transcript's last record |
| `duration_ms` | `totalDurationMs` of the completed call | `stopped_at` minus `started_at` |
| `tool_uses` | `totalToolUseCount` | the transcript (always for a background run) |
| `lines_added`, `lines_removed` | `toolStats.linesAdded`, `toolStats.linesRemoved` | the transcript (always for a background run) |
| `brief` | `tool_input.prompt` | the first user record of the transcript |
| `final_message` | `last_assistant_message` of the stop event | the text of `tool_response.content`, then the transcript's last assistant text |
| `skills_opened` | the run's transcript, by the rule below | none |
| `tokens` | the transcript (section 8) | the stored total |

Facts behind the second sources (tested):

- The metadata file holds `agentType`, `spawnDepth`, `requestShape` (`foreground` or `background`), `description`, `toolUseId`, and at depth 2 `parentAgentId`. It is rewritten, not appended. `finished` is absent while the run works and `true` at the end.
- `model` is in the metadata file only sometimes, so it is not used. `pickedEffortModel` is the main session's model, not the run's: it must never be used.
- `tool_response.usage` and `totalTokens` cover the last request only. They are never a run's tokens.

**`agent_type`** stays text: a run can be a built-in agent such as `general-purpose`. A reader that needs the grid pair parses it with `super_rex::oms::GridAgent` (Lane 6). Capture writes no parser for it.

**`tool_uses`** is one count, Claude Code's `totalToolUseCount`. From a transcript it is the number of `tool_use` blocks; an `Agent` call counts. A run that called no tool has 0.

**Lines added and removed** are Claude Code's own count, not a line diff, so that foreground runs (from the event) and background runs (from the transcript) agree. An `Edit` adds the number of pieces of `new_string` split on `\n` and removes the number of pieces of `old_string`. A `Write` adds the number of pieces of `content` (a final newline counts as one more). Example (tested): an Edit of `alpha\nbeta\ngamma` to `alpha\nBETA\ngamma\ndelta` gives `linesAdded 4, linesRemoved 3`; a diff would give 2 and 1.

**Skills opened.** A run's skills opened = the names from its `Skill` tool calls (`input.skill`), plus, for each `Read` call that did not fail (its `tool_result` has no `is_error: true`) and whose `file_path` ends in `/SKILL.md`, the name of the folder that holds that file. Each name one time, sorted; stored as C1 5.2 says (joined by commas; NULL when empty). No hook is registered for this (C3, OP9): the transcript already holds the calls. Limits: a skill read by a shell command, or preloaded by an agent definition's `skills:` field, is not seen; the transcript must still exist at the first read.

**Models.** A run can name more than one model (tested, one session: a request was stopped by a safety classifier and the run went on with another model). Its tokens are then the sum over both models. `model_ran` is `resolvedModel` when a completed-call event gave it, else the model of the transcript's last assistant record.

**`builder_model` and `builder_effort`.** When `job_runs` reads the main session's transcript, `read_main_transcript` finds the `Bash` call whose result's first line is `job=<this job's ID>` and that has no `existing=yes` line: the `rex job create` that opened the job. `job_runs` writes `message.model` of the assistant record that holds that call into `job.builder_model`, and that record's `effort` into `job.builder_effort`. Each column has its own statement and is written only while it is NULL: `UPDATE job SET builder_model = :model, updated_at = :now WHERE id = :job AND builder_model IS NULL` (the same for effort). IF the main transcript holds no such call (the builder ran in a sub-agent), THEN both stay NULL. The flags of `rex job create` come first (`--builder-model`; `--builder-effort`, else the environment variable `CLAUDE_EFFORT`; C2 OP15); this fills only what they left NULL.

**The tree.** A run whose `parent_agent_id` names no run of the same job (its parent started before the job, so it was not stored) is a root of the job's tree, like a run that the main session started. `parent_agent_id` keeps the ID. `rex status` prints its `<id`, and C9's tree shows it at the top level.

## 5. Recovery

Recovery finds runs that left no event. It runs inside `job_runs`.

- It runs only for a session that has the key `intake.transcript.<session_id>` (section 8): a session that logged at least one line. A session with no line is never searched: with the hooks off there are no runs and no `running`.
- Its sub-agents are in `<folder of that transcript>/<session_id>/subagents/`, as `agent-<id>.jsonl` (transcript) and `agent-<id>.meta.json` (metadata).
- Each metadata file with no run row gives one, by the same upsert, IF a job of the session was open at its tie time (the first record of its transcript). Else it is skipped.
- Limit: a session that lost every one of its events is not recovered.

## 6. State

Worked out at each read from stored facts and transcript activity. Never stored. `now` is a parameter, so tests need no sleep.

- **finished**: `stopped_at` is set (by a stop line, a completed call, or the metadata file's `finished: true`; tested 5 of 5 runs: foreground, background, worktree, sibling folder, sub-folder).
- **open**: not finished, AND activity in the last `QUIET` = 15 minutes. Activity = the newest of: `started_at`, the modification time of the run's transcript, and the same for each run below it in the tree (a lead that waits for a worker is active).
- **unconfirmed**: not finished and not open.

`QUIET` is one named constant. 15 minutes is longer than the longest shell command an agent can run in the foreground (10 minutes). Because state is not stored, a quiet run that becomes active again is shown as open again.

Capture never changes a job's stored status. A sub-agent that starts before the plan is recorded gets a run and leaves the status as it is.

## 7. Incomplete

`incomplete` is `true` when any of these holds: a run is unconfirmed; a run that is not open has tokens `Unknown`; a read of a transcript of the job (a run's or the main session's) found damaged lines; a file of the job could not be read at this read (section 1, Failures); the job has runs and `session_tokens` is `Unknown`. An open run whose transcript is not written yet does not set it. `damaged_log_lines` is the value of `intake.damaged_lines`, beside it. It counts the whole log, not one job, so it does not set the flag.

## 8. Tokens

- A total = four counts: `input` (`input_tokens`), `output` (`output_tokens`), `cache_read` (`cache_read_input_tokens`), `cache_creation` (`cache_creation_input_tokens`). All models added up.
- Reader rule: over the `assistant` records of one transcript, take ONE record for each `message.id`, add up. Each request is split over several records that repeat the same ID and usage (tested). In the recorded fixture session (three models), these sums per model over the main transcript and every sub-agent transcript equal Claude Code's own `modelUsage` exactly; they are lower when a request was stopped (section 13).
- Totals are added up in `job_runs`, never in the hook, and stored with the read: on the run, C1's `tokens_input`, `tokens_output`, `tokens_cache_read`, `tokens_cache_creation`, `tokens_provisional`, `transcript_bytes`, `transcript_damaged_lines` and `tokens_read_at`. `tokens_read_at` is the `now` of that `job_runs` call.

**Provisional rule for a run.** At each read, `job_runs` takes the size of the run's transcript and takes the first step that applies (`TotalCheck::next_step`):

1. No file now: the stored total stays as it is. Nothing stored = `Unknown`.
2. Nothing stored, or the size differs from `transcript_bytes`: parse the file and store the totals, `transcript_bytes`, `transcript_damaged_lines` and `tokens_read_at`. The total is provisional.
3. The job is open, or the run is open: keep, provisional.
4. `tokens_read_at` is before the job's close time: parse again and store, as in step 2.
5. `transcript_damaged_lines` > 0: keep, provisional. A damaged transcript never gives a final total.
6. `now` is at least `SETTLE` = 10 seconds after `tokens_read_at`: final (`tokens_provisional` = 0). Else keep.

A total stored before the close never makes a total final. Reason: the transcript is written a little behind the agent (tested: for two of three agents the last request was missing when the stop event fired). A final total is never read again.

**The main session's figure** (`session_tokens`; OP12).

- Where it is: `take_in` stores the `transcript_path` of a session's lines in `meta` under `intake.transcript.<session_id>` the first time it sees a line of that session, also a line it skips. Recovery (section 5) uses the same key.
- What it counts: `read_main_transcript` (section 1) over that file, from `job.created_at` to the job's close time (no end while open): each message ID once, counted when its first record's `timestamp` is inside the window.
- Stored in table `session_tokens`, one row for each job, written only by `job_runs`: the four counts, `tokens_provisional`, `transcript_bytes`, `transcript_damaged_lines`, `tokens_read_at`.
- Steps 1 to 6 of the run rule apply, with the session checked as not open (`RunActivity::NotOpen`). One step differs, because the main transcript goes on growing after the job: at a read after the close where step 2 finds the size changed, `job_runs` parses the file again. IF the new figure equals the stored one, AND the stored `tokens_read_at` is at or after the close, AND `now` is at least `SETTLE` after it, AND no damaged line was found, THEN the stored figure becomes final. ELSE the new reading is stored. So the figure becomes final at the second read after the close, and is never parsed again.
- No `intake.transcript.<session_id>` key: `session_tokens` is `Unknown` and no row is written.

`JobRuns.tokens` = the sum of every run's total and `session_tokens`. It is `Final` only when all of them are final, `Unknown` only when all are unknown, else `Provisional`.

- `job_runs` parses transcripts and reads metadata files outside any transaction. Then it stores what changed in one `Store::write`, and begins no write when nothing changed. A reading is stored with `UPDATE … WHERE id = :id AND tokens_provisional = 1`. The step to final is `UPDATE sub_agent_run SET tokens_provisional = 0 WHERE id = :id AND tokens_provisional = 1 AND transcript_bytes = :bytes AND tokens_read_at = :read_at` (the same on `session_tokens`, by `job_id`). So the writer lock is held for milliseconds, never while a file is parsed. A reading from a second `rex` at the same moment never undoes a final total, and a step to final that a newer reading has passed changes nothing.

## 9. `rex status [job]`

Plain text, short, for agents. It calls `job_runs` with the current time.

```
job k3x9p2m7qd  planned (running)  2 open  3 finished  1 unconfirmed  incomplete
open
  a66aa314f72652050  background-worker  model-low-effort-medium  2m05s  bg  8.4k tokens~
  aec6fb0b61019a391  worker             model-low-effort-low     41s  <a66aa314f72652050  3.1k tokens~
finished
  a0c8cde396de066a3  lead               model-medium-effort-low  3m12s  61.0k tokens~
  a5f19c27d0e3b8a64  worker             model-low-effort-low     38s  <a0c8cde396de066a3  14.2k tokens~
  ad73e0b5c91f4a2e8  reviewer           model-high-effort-low    52s  18.9k tokens~
unconfirmed
  ab12cd34ef56ab78c  -                  general-purpose          no stop event, quiet 31m00s  9.8k tokens~
```

- One header line for each job: its ID, its stored status, `(running)` when `Shown::of` gives `Running`, the counts `open`, `finished`, `unconfirmed` of `JobRuns`, and the word `incomplete` when the flag is set.
- A section (`open`, `finished`, `unconfirmed`) is printed only when it has a run.
- One line for each run: `agent_id` in full, `role` or `-`, `agent_type`, then the duration (open: since `started_at`; finished: `duration_ms`; unconfirmed: `no stop event, quiet <duration>` since the last activity). Then `bg` when `background`, then `<id` for `parent_agent_id` (left out for the main session), then the tokens.
- Duration: below one minute `41s`; below one hour `2m05s`; else `1h02m`.
- Tokens are printed on every run line whose total is not `Unknown`, also for open and unconfirmed runs (an open run's total is always provisional). The number is the sum of the four counts: below 1000 the number itself (`950`), below one million thousands with one decimal (`61.0k`), else millions with one decimal (`1.2M`). The word `tokens` follows, and `~` comes right after the word when the total is provisional.
- No job given: every job that is `created` or `planned`. None = the one line `no open job`.
- A last line `event log: N damaged lines`, only when N > 0.
- Handler (C11 2): `pub fn run(cwd: &Path, options: StatusOptions) -> Result<(), StatusError>`, with `StatusOptions { job: Option<JobId> }`, positional, parsed by `JobId::from_str` as clap's value parser. `StatusError` lives in `src/commands/status.rs`, with the `#[error(transparent)]` variants `Store(#[from] StoreError)`, `Capture(#[from] CaptureError)` and `Rex(#[from] RexError)`. No variant is a usage error.
- Exit 0: the output above, also `no open job`. Exit 2: `<job>` is not of the job ID form (clap's message). Exit 1: no store (C1 `NotFound`); no such job (C1 `JobNotFound`, with the text of C2 6.4); a store or event-log failure. Each exit that is not 0 writes one `error: ` line (C11 4).

## 10. Open points

### OP12 — settled

**(1) The main session's tokens are recorded**, as one figure for each job (`session_tokens`), not as a run: the requests of the main session's transcript, each message ID counted once, whose `timestamp` is at or after `job.created_at` and before the job's close time (no end while the job is open). Requests before `rex job create`, and between two jobs, count for no job. `JobRuns.tokens` = every run of the job plus this figure.

**(2) Every read that parses a transcript stores what it found**, provisional or final. A final total is never read again, and it stays in the store after Claude Code removes the transcript.

Table `session_tokens` (C1): one row for each job, written only by `job_runs`: `job_id`, the four counts, `tokens_provisional`, `transcript_bytes`, `transcript_damaged_lines`, `tokens_read_at`.

Limits: (a) when one session runs several jobs, its main-session tokens are split by time; (b) the figure is known only for a session that logged at least one event line, so a job with no sub-agent, or a session started in a `git worktree add` folder, has `session_tokens` `Unknown`; (c) a request that Claude Code bills but does not write to any transcript (seen: one stopped by a safety classifier) is not counted, so a sum can be lower than Claude Code's `modelUsage`.

Alternatives not chosen: tokens on sub-agent runs only (a job's cost then misses the orchestrator, and no total can be checked against `modelUsage`); storing only the final value (a job whose transcripts were removed before a final read then has no cost at all).

## 11. What C1, C2 and C9 hold for C5

Settled at gate G1. Each point where C5 differed from the C1, C2 and C9 drafts is now held there:

- C1 7.3: `Shown::of(stored, open_runs)` is the one shown-status rule, with `open_runs` = `JobRuns.open`. There is no `status::shown`. `store` imports nothing from `capture` or `commands`.
- C1 13: a run first seen while its session has no open job is not stored (section 3). `sub_agent_run.job_id` stays NOT NULL, and both triggers stay as they are.
- C1 5.9 and 5.13: `sub_agent_run.tag_job TEXT` (after `role`), `sub_agent_run.transcript_damaged_lines INTEGER` and `sub_agent_run.tokens_read_at TEXT` (after `transcript_bytes`); table `session_tokens`, one row for each job: `job_id`, the four counts, `tokens_provisional`, `transcript_bytes`, `transcript_damaged_lines`, `tokens_read_at`. The meanings of `depth`, `agent_type`, `skills_opened` and `started_at` are those of section 4. `duration_ms` is from the completed call only, NULL when none was taken in (section 2). `skills_opened` NULL = none seen, or no transcript read.
- C1 5.12, the writers: `meta` keys `intake.offset`, `intake.damaged_lines`, `intake.transcript.<session_id>`: Lane 3, `capture::take_in`. `session_tokens`: Lane 3, `capture::job_runs`. `job.builder_model` and `job.builder_effort`, only while NULL: Lane 3, `capture::job_runs` (section 4).
- C1 5.1: one time for one write, and Rust times into and out of SQL as milliseconds since the epoch (section 2).
- C2 2.6 and 6.5: `rex job plan` calls `take_in`; `rex job close` (before its write) and `rex job show` call `job_runs`; `rex job create` calls neither.
- C2 6.2 and 6.3: `rex job show` gives, for each run, `agent_id`, `state`, `parent_agent_id`, `depth`, `background`, `role`, `tag_job`, `agent_type`, `model_ran`, `effort_ran`, `started_at`, `stopped_at`, `duration_ms`, `tool_uses`, `lines_added`, `lines_removed`, `brief`, `skills_opened`, `final_message`, `tokens`; at the top level `tokens`, `session_tokens`, `incomplete`, `damaged_log_lines`. Totals in the JSON form of section 1. Runs in `JobRuns.runs` order (by `started_at`, then `agent_id`), not by `first_seen_at`.
- C9 2: `rex jobs` and `rex watch` call `capture::job_runs` for each job they show, and write nothing except what `job_runs` stores. `rex recall` and `rex feedback` call nothing of capture.
- C9 5.2: `tokens` is `JobRuns.tokens` (every run plus the main session) in the JSON form of section 1, `null` when unknown; `incomplete` is `JobRuns.incomplete`.
- C9 6 and 7: a run whose parent is not a run of the job is shown at the top level (section 4); `~` after a provisional total; the tag job beside a run when `tag_job` is set; the shown status by `Shown::of`.

## 12. What each other lane must do

**Lane 2 (store and jobs).**

- `rex job plan` calls `take_in`. `rex job close` (in C2's order, before its write) and `rex job show` call `job_runs`. Shown status by `Shown::of(stored, open_runs)`, with `open_runs` = `JobRuns.open`.
- `rex job create` keeps a session to one open job (C1's unique index already does).

What C1 gives capture: `StorePaths::discover` and `event_log`; `Store::open`, `Store::write`, `Store::conn`; `Shown::of`; the upsert form of C1 6.1 with its two triggers; table `sub_agent_run` with every column C5 uses (`job_id`, `session_id`, `agent_id`, `parent_agent_id`, `depth`, `role`, `tag_job`, `agent_type`, `model_ran`, `effort_ran`, `brief`, `skills_opened`, `background`, `first_seen_at`, `started_at`, `stopped_at`, `duration_ms`, `tool_uses`, `lines_added`, `lines_removed`, `final_message`, `transcript_path`, the four `tokens_*` counts, `tokens_provisional`, `transcript_bytes`, `transcript_damaged_lines`, `tokens_read_at`); table `session_tokens`; `job.id`, `job.session_id`, `job.created_at`, `job.builder_model`, `job.builder_effort`, `job.updated_at`; `job_transition.job_id`, `to_status`, `at`; table `meta` with the prefix `intake.`. What C5 asked of C1 and C2 is settled (section 11).

**Lane 7 (views).** `rex jobs` and `rex watch` call `job_runs(&mut store, &job, now)` for each job they show. It takes in the log itself, and a read with nothing new takes no lock (section 2). `rex recall` and `rex feedback` call nothing of capture. From `JobRuns`: the counts `open`, `finished`, `unconfirmed`; `tokens` (every run plus the main session) and `session_tokens`, each `null` in JSON when `Unknown` (section 1); `incomplete`; `damaged_log_lines`; the tree by `parent_agent_id` (section 4, "The tree"). Shown status by `Shown::of(stored, open_runs)`, with `open_runs` = `JobRuns.open`. `rex watch` shows `~` for `Provisional`, nothing for `Unknown`, and `tag_job` beside a run when it is set.

**Lane 10 (delivery skills).** `rex-feedback` reads, for each run, the role, the brief, the model and effort that ran, the skills opened and the final message. C5 gives all five on `Run` (`role`, `brief`, `model_ran`, `effort_ran`, `skills_opened`, `final_message`). They reach the skill through the `runs` objects of `rex job show <job> --json` (C2 6.3, section 11). An empty field is `null`. A skill that needs to know how a job is going tells the agent to run `rex status <job>` (section 9).

## 13. What was tested and what was not

Tested on Claude Code 2.1.322, macOS, in recorded headless sessions: the fields each event carries; `finished: true` in the metadata file for 5 of 5 runs; the metadata keys above; the reader rule (one record per message ID), over the main transcript and every sub-agent transcript, equals Claude Code's `modelUsage` for each model in the fixture session (three models); a run that names two models after a stopped request; the lines-changed rule on three runs; the transcript lags the stop event; a background run's completed-call event carries launch data only; transcripts hold each `Skill` call with its name and each `Read` call with its path and result, in the main session and in sub-agents.

Settled, not yet built: intake, the tie, recovery, state, the provisional rule, `rex status`. Lane 3's tests prove them on the recorded fixtures, one test for each case of plan 3.12, each with its result:

1. A sub-agent of a session with open job A runs `rex job create`: it gets A with `existing=yes` (C2), and the runs it starts are tied to A. No second job exists.
2. Run R starts in job A; A closes; B opens; then R's stop line and completed call are taken in: R stays in A, and B has no R. The same with R's start line lost (tie time = completed call minus `totalDurationMs`).
3. A sub-agent starts after `rex job create` and before `rex job plan`: it is tied to the job, and the shown status stays `created`.
4. The session ends at once (stop lines lost): runs whose metadata file says `finished` are finished. The others are open until `QUIET`, then unconfirmed, and `incomplete` is true.
5. Each of the three lines of a run is lost in turn: the same tree and fields. All three lost: the run is recovered from its metadata file.
6. A transcript's last record is appended after the close: the total is provisional at the read inside `rex job close`, and after two reads at least `SETTLE` apart it is final and includes that record.
7. A second connection holds the writer lock while an agent finishes: `rex hook` still appends at once, and the line is in the log. A `job_runs` that waits past `busy_timeout` fails with exit 1 and writes nothing. The next read gives the full history.

Not tested: a run that is killed or interrupted (does `finished` still appear?); `replace_all`, `MultiEdit` and `NotebookEdit` in the lines-changed rule; a skill read by a shell command; Linux.

## 14. Limits

- A session that lost every event is not recovered.
- A run first seen while its session had no open job is not stored.
- A run whose start line was lost, with no duration and no transcript, can be tied to a later job (tie source 5).
- A killed run was not tested; it may stay unconfirmed.
- `SETTLE` = 10 seconds: a transcript record written later than that after a read could be missed by a total already final.
- The read inside `rex job close` comes before the close time, so totals become final only after two more reads, the second at least `SETTLE` after the first. Until a view reads the job again, its totals stay provisional.
- `QUIET` = 15 minutes: a live run that is silent for longer is shown as unconfirmed until its next activity.
- The event log grows by about three lines for each sub-agent; it is not truncated in version 1.
- A sub-agent resumed after its totals became final: the new requests are not counted.
- When one session runs several jobs, its main-session tokens are split by time.
- Requests that Claude Code bills but does not write to the transcript (seen one time: a request stopped by a safety classifier) are not counted. A sum can then be lower than Claude Code's own `modelUsage` for that model.
- A session started in a folder made with `git worktree add` has no `.claude/settings.local.json`, so no hook fires there (C3 7) and the safety settings do not apply: start a session in the main checkout. Its jobs have no runs, and `session_tokens` is `Unknown`. `incomplete` stays false, because nothing tells this case apart from a job that started no sub-agent.
- A job whose session logged no line (no sub-agent, or the hooks off) has `session_tokens` `Unknown`: its main-session cost is not recorded.
- Requests of the main session before `rex job create`, or between two jobs, count for no job.
- A run that started before its session's job was created is not stored, also when the builder runs inside it. Its children are roots of the job's tree (section 4).
- A deleted and regrown event log is not detected: IF a person deletes the log and it grows past the saved offset before the next intake, THEN its first lines are skipped with no sign.
