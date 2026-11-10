Status: DRAFT

# C5. The read model: runs, their state and tokens, and `rex status`

- Written by: Lane 3 (Capture).
- Read by: Lane 2 (`rex job plan`, `rex job close`, `rex job show`, and C1), Lane 7 (`rex jobs`, `rex watch`, `rex feedback`), Lane 10.
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

Marks: "tested" = seen on Claude Code 2.1.322 by a live probe or in Lane 3's recorded fixtures; "proposed" = a decision of this contract. `Store` and `JobId` are C1's (`super_rex::store`). `CaptureError` is capture's error type; it converts from C1's `StoreError`, and its form follows C11. Field names are C1's column names on purpose.

## 1. The interface

In `src/capture/mod.rs`:

```rust
/// Takes the new lines of the event log into the store. Cheap. Reads no transcript, except the
/// first record of one for a run whose start line was lost (the tie time, section 3).
pub fn take_in(store: &mut Store) -> Result<(), CaptureError>;

/// Calls `take_in`, finds runs that left no event, reads transcripts, stores totals,
/// and returns the runs of one job as they stand at `now`.
pub fn job_runs(store: &mut Store, job: &JobId, now: SystemTime) -> Result<JobRuns, CaptureError>;

/// Reads one transcript file. Knows nothing about the store or about jobs.
pub fn read_transcript(path: &Path) -> Result<TranscriptFacts, CaptureError>;

pub struct JobRuns {
    pub runs: Vec<Run>,             // ordered by start time; the tree is in `parent_agent_id`
    pub open: u32,
    pub finished: u32,
    pub unconfirmed: u32,
    pub tokens: TokenTotal,         // all runs plus the main session
    pub session_tokens: TokenTotal, // the main session alone, inside the job's time
    pub incomplete: bool,           // a run is unconfirmed, or a run's tokens are unknown
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
    pub duration_ms: Option<u64>,
    pub tool_uses: Option<u64>,
    pub lines_added: Option<u64>,
    pub lines_removed: Option<u64>,
    pub brief: Option<String>,
    pub final_message: Option<String>,
    pub skills_opened: Option<Vec<String>>, // sorted; None = no transcript was read for this run
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
    pub first_at: Option<String>,        // first and last record that carry a timestamp
    pub last_at: Option<String>,
    pub first_user_text: Option<String>, // the brief, for a sub-agent transcript
    pub last_assistant_text: Option<String>,
    pub model: Option<String>,           // of the last assistant record
    pub effort: Option<String>,
    pub skills_opened: Vec<String>,      // sorted, each name one time
}
```

Who calls what:

| Command | Calls |
| --- | --- |
| `rex job plan` | `take_in` |
| `rex job close`, `rex job show`, `rex status`, `rex jobs`, `rex watch`, `rex feedback` | `job_runs` |
| `rex job create`, `rex hook`, all others | nothing |

- `job_runs` takes in the log itself, so no caller needs a separate step before it.
- `rex job close` keeps C2's order: it calls `job_runs` first, then writes the close. So the read inside `close` is before the close time and can never make a total final (section 8). The next read after the close stores the totals again, and one more read at least `SETTLE` later makes them final.
- The shown status: callers use C1's `Shown::of(stored, runs.open)`. `running` = stored status `planned` and `open` > 0.
- With the hooks absent the log is empty: `job_runs` returns no runs, `open` is 0, and `running` is never shown.

Cost: `take_in` reads the log from the saved offset to its end, and the first record of a transcript only for a stop line with nothing before it. `job_runs` also reads, for each run of the job whose total is not final, the size of its transcript, and parses the transcript only when the size changed or a read after the close is due (section 8); the main session's transcript, IF the job's session figure is not final; and the `subagents/` folder of the job's session, for recovery (section 5). IF every total of the job is final, THEN `job_runs` reads no transcript and does no recovery for that job. This bounds the cost of `rex jobs` and `rex watch`.

## 2. Intake

- Intake opens the store with `Store::open` only, and writes inside one `Store::write` (a `BEGIN IMMEDIATE` transaction) that is begun BEFORE it reads the saved offset. It reads the log from that byte offset to the last `\n`, applies each line, saves the new offset and commits. So two commands at the same moment never take in, or count, one line two times.
- Saved values, in C1's table `meta`: `intake.offset` (bytes) and `intake.damaged_lines` (a count).
- A line that is not valid JSON, has no `v`, or has a `v` that is not `1`, is counted as damaged and skipped. Bytes after the last `\n` wait for the next intake. IF the file is shorter than the offset, THEN intake starts again at 0.
- A `PostToolUse` line counts only when `tool_name` is exactly `Agent` or `Task`.
- Each line becomes an upsert on one run, in C1's form: `INSERT … ON CONFLICT(session_id, agent_id) DO UPDATE SET …`. Never `INSERT OR REPLACE`. Never `job_id` or `first_seen_at` in the update list. An upsert sets fields; it never adds to a counter. So a line taken in twice changes nothing.
- A field already set from an event is not overwritten by a weaker source (the second sources in section 4).
- Times: the log carries `at_ms`; the store holds every time as text `YYYY-MM-DDTHH:MM:SS.sssZ`. Intake converts in SQL: `strftime('%Y-%m-%dT%H:%M:%fZ', :at_ms / 1000.0, 'unixepoch')`. Transcript `timestamp` text is already that form. Comparisons with job times are text comparisons in SQL.

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

**When a job is open.** From `job.created_at` until the `at` of its first `job_transition` row whose `to_status` is not `created` or `planned`. That `at` is the job's close time. A job never goes back to an open status (C2 7).

**Rule 1 (first seen).** When a line names a run that has no row, its job = the job of that `session_id` that was open at the run's tie time. A session has at most one open job (C1's unique index), so the rule finds at most one.

**Tie time** = the best estimate of when the run started, from the line that creates the row:

1. `SubagentStart`: its `at_ms`;
2. `async_launched`: its `at_ms` (it fires at launch, tested);
3. a completed call: `at_ms` minus `tool_response.totalDurationMs`;
4. a stop line with nothing before it: the time of the first record of `agent_transcript_path` (one line read);
5. otherwise: the line's `at_ms`.

For a run found by recovery: the time of the first record of its transcript. Reason: when the start line was lost, the first line seen comes from the END of the run, and the run may have ended after its job closed.

**Rule 2 (kept).** The tie is written once. A later line for that run never changes it, also after the job closed and another opened. C1's trigger aborts any change of `job_id`.

**A run with no job is NOT stored.** C1's `sub_agent_run.job_id` is required. For a line that names a run with no row: IF its session has no job at all, THEN the line is skipped. ELSE intake works out the tie time and looks for the job of that session that was open then; none = the line is skipped. Skipped lines are not damaged. This still obeys "a run keeps the job it was first seen under": the tie time is the run's START, so every later line for that run gives the same answer, and a run first seen with no open job stays with no job, with no row. This is C5's answer to the open seam of C1 section 13.

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

**`builder_model`.** When `job_runs` reads the main session's transcript and `job.builder_model` is NULL, it writes the `message.model` of the first assistant record inside the job's time, and `builder_effort` from that record's `effort` when that is NULL too, and sets `job.updated_at` (C2 OP15).

## 5. Recovery

Recovery finds runs that left no event. It runs inside `job_runs`.

- It runs only for a session that has one or more lines in the log. A session with no line is never searched: with the hooks off there are no runs and no `running`.
- The session's folder = the folder of `transcript_path` from such a line. Its sub-agents are in `<that folder>/<session_id>/subagents/`, as `agent-<id>.jsonl` (transcript) and `agent-<id>.meta.json` (metadata).
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

`incomplete` is `true` when a run is unconfirmed, or a run's tokens are `Unknown`. `damaged_log_lines` is the value of `intake.damaged_lines`, beside it.

## 8. Tokens

- A total = four counts: `input` (`input_tokens`), `output` (`output_tokens`), `cache_read` (`cache_read_input_tokens`), `cache_creation` (`cache_creation_input_tokens`). All models added up.
- Reader rule: over the `assistant` records of one transcript, take ONE record for each `message.id`, add up. Each request is split over several records that repeat the same ID and usage (tested). In the recorded fixture session (three models), these sums per model over the main transcript and every sub-agent transcript equal Claude Code's own `modelUsage` exactly; they are lower when a request was stopped (section 13).
- Totals are added up in `job_runs`, never in the hook, and stored with the read: on the run, C1's `tokens_input`, `tokens_output`, `tokens_cache_read`, `tokens_cache_creation`, `tokens_provisional`, `transcript_bytes`, and `tokens_read_at` (asked of C1).

**Provisional rule for a run.** Its total becomes final (`tokens_provisional` = 0) at a read where all hold:

1. the job is closed;
2. the run is not open;
3. `tokens_read_at` is at or after the job's close time;
4. `now` is at least `SETTLE` = 10 seconds after `tokens_read_at`;
5. the transcript's size equals `transcript_bytes`.

- IF the size differs, OR `tokens_read_at` is before the close, THEN the transcript is read again, and the totals, `transcript_bytes` and `tokens_read_at` are written.
- IF the size is equal and the total is not final, THEN the file is not parsed again.
- A total stored before the close never makes a total final. Reason: the transcript is written a little behind the agent (tested: for two of three agents the last request was missing when the stop event fired).
- A final total is not read again.
- Transcript absent: the stored total stays as it is (a provisional one stays provisional). Nothing stored and no transcript = `Unknown`.

**The main session's figure** (`session_tokens`; OP12): the requests of the main session's transcript whose `timestamp` is from `job.created_at` to the close time (or to `now` while open). Stored in a table `session_tokens` asked of C1: one row for each job, with the four counts, `tokens_provisional` and `tokens_read_at`, written only by Lane 3. The same rule applies without conditions 2 and 5: its "unchanged" test is equal totals, because the main transcript goes on growing after the job.

`JobRuns.tokens` = the sum of every run's total and `session_tokens`. It is `Final` only when all of them are final, `Unknown` only when all are unknown, else `Provisional`.

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
- Exit code and error text for an unknown job: as C11 says.

## 10. Open points

### OP12 — proposed: the main session's tokens, and storing totals

**(1) The main session's tokens are recorded**, as one figure on the job (`session_tokens`), not as a run: the requests in the main session's transcript whose time is inside the job's time. Reasons: the orchestrator's use is part of what a job costs, often the larger part, and "how an orchestrator performed" is wrong without it; the acceptance gate compares against Claude Code's own totals, which include the main session (tested: the sums matched only with the main transcript added); the cost is one more file through the same reader. Limit: when one session runs several jobs, the split between them is by time.

**(2) A total is stored before and after it becomes final.** Every read that parses a transcript stores what it found, and the final value stays in the store. Reason: Claude Code removes old transcripts on its own schedule, and recall and feedback must still see what a job cost.

C1's draft already has the four counts, `tokens_provisional` and `transcript_bytes` on `sub_agent_run`. C5 asks C1 to add `sub_agent_run.tokens_read_at TEXT` and the table `session_tokens` (section 8).

Alternatives not chosen: (1) tokens on sub-agent runs only. Then a job's cost misses the orchestrator, and the totals can never match Claude Code's own. (2) Reading totals from transcripts only, or storing only the final value. Then a job whose transcripts were removed, or were removed before a final read, has no cost at all.

## 11. Where C5 differs from the C1, C2 and C9 drafts

| Their draft now | C5 | Why |
| --- | --- | --- |
| C1 13: what intake does with a run first seen while its session has no open job is open. | Settled: such a run is not stored (section 3). `job_id` stays required. | The tie time is the run's start, so later lines give the same answer; no nullable `job_id` and no change of the trigger is needed. |
| C1 7.3: `status::shown(conn, job)` reads the open-run count from C5. | Callers use `Shown::of(stored, runs.open)`. C5 asks C1 to drop `status::shown`, or give it the count as an argument. | Import direction is commands -> capture -> store. `store` must never import `capture`. |
| C1 5.9 has no `tag_job` or `tokens_read_at`, and no place for the main session's tokens. | Asks `sub_agent_run.tag_job TEXT`, `sub_agent_run.tokens_read_at TEXT`, and the table `session_tokens`. | `Run.tag_job`; the provisional rule (section 8); OP12. |
| C1 5.9 gives no source for `skills_opened`, `started_at`, `duration_ms`; `agent_type` reads "grid agent chosen". | `skills_opened` is filled from the transcript (section 4); `started_at` = the start line, else the tie-time estimate; `duration_ms` = the completed call, else stop minus start; `agent_type` = the agent type that ran, a grid agent or a built-in one. | So that C1's meanings say what intake writes. |
| C2 2.6: `rex job show` and `rex job close` "take in" the log. | They call `job_runs`, which takes in the log itself. `rex job plan` calls `take_in`. | One entry point for reads; same effect as C2 asks. |
| C2 6.3: the `runs` objects of `rex job show --json` have no `brief`, `skills_opened`, `final_message`, `background`, `depth`. | `Run` has them. C5 asks C2 to add them to the `runs` objects (`null` when empty). | Lane 10's `rex-feedback` reads brief, skills opened and final message for each run. |
| C9 7: an intake entry point as the first step of `rex jobs` and `rex watch`. | `take_in` is that entry point; `job_runs` calls it itself, so those commands need only `job_runs`. | No second step to forget. |

## 12. What each other lane must do

**Lane 2 (store and jobs).**

- `rex job plan` calls `take_in`. `rex job close` (in C2's order, before its write) and `rex job show` call `job_runs`. Shown status by `Shown::of(stored, runs.open)`.
- `rex job create` keeps a session to one open job (C1's unique index already does).

What C1's draft already gives capture: `StorePaths::discover` and `event_log`; `Store::open`, `Store::write`, `Store::conn`; `Shown::of`; the upsert form of C1 6.1 with its two triggers; table `sub_agent_run` with every column C5 uses (`job_id`, `session_id`, `agent_id`, `parent_agent_id`, `depth`, `role`, `agent_type`, `model_ran`, `effort_ran`, `brief`, `skills_opened`, `background`, `first_seen_at`, `started_at`, `stopped_at`, `duration_ms`, `tool_uses`, `lines_added`, `lines_removed`, `final_message`, `transcript_path`, the four `tokens_*` counts, `tokens_provisional`, `transcript_bytes`); `job.id`, `job.session_id`, `job.created_at`, `job.builder_model`, `job.builder_effort`, `job.updated_at`; `job_transition.job_id`, `to_status`, `at`; table `meta` with the prefix `intake.`.

Asked of C1 before the freeze:

- [ ] Column `sub_agent_run.tag_job TEXT`.
- [ ] Column `sub_agent_run.tokens_read_at TEXT`.
- [ ] Table `session_tokens`: one row for each job (`job_id`), the four counts, `tokens_provisional`, `tokens_read_at`; written only by Lane 3.
- [ ] Meanings in 5.9: `skills_opened` filled from the transcript; `started_at` = the start line, else the tie-time estimate; `duration_ms` = the completed call, else stop minus start; `agent_type` = the agent type that ran.
- [ ] Drop `status::shown(conn, job)`, or give it the open-run count as an argument.

Asked of C2 before the freeze:

- [ ] `rex job show --json` `runs` objects also give `brief`, `skills_opened`, `final_message`, `background`, `depth`. The plain `run.` line can stay short.

**Lane 7 (views).** `take_in` is the intake entry point C9 7 asks for; `rex jobs`, `rex watch` and `rex feedback` call `job_runs`, which calls it. `JobRuns` gives the counts (`open`, `finished`, `unconfirmed`), the totals with the provisional mark (`tokens`, `session_tokens`), the tree (`parent_agent_id`) and the incomplete flag. Shown status by `Shown::of`. Show `~` for `Provisional`, nothing for `Unknown`, and the tag job beside a run when `tag_job` is set.

**Lane 10 (delivery skills).** `rex-feedback` reads, for each run, the role, the brief, the model and effort that ran, the skills opened and the final message. C5 gives all five on `Run` (`role`, `brief`, `model_ran`, `effort_ran`, `skills_opened`, `final_message`). They reach the skill through `rex job show --json` once C2 adds `brief`, `skills_opened` and `final_message` to its `runs` objects (asked of Lane 2). An empty field is `null`. A skill that needs to know how a job is going tells the agent to run `rex status <job>` (section 9).

## 13. What was tested and what was not

Tested on Claude Code 2.1.322, macOS, in recorded headless sessions: the fields each event carries; `finished: true` in the metadata file for 5 of 5 runs; the metadata keys above; the reader rule (one record per message ID), over the main transcript and every sub-agent transcript, equals Claude Code's `modelUsage` for each model in the fixture session (three models); a run that names two models after a stopped request; the lines-changed rule on three runs; the transcript lags the stop event; a background run's completed-call event carries launch data only; transcripts hold each `Skill` call with its name and each `Read` call with its path and result, in the main session and in sub-agents.

Proposed, not yet built: intake, the tie, recovery, state, the provisional rule, `rex status`. Lane 3's tests prove them on the recorded fixtures, with lost start lines, lost stop lines, a last transcript record that arrives after the close, and a held writer lock.

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
- Requests that Claude Code bills but does not write to the transcript (seen: requests stopped by a safety classifier) are not counted. The stored total is then lower than Claude Code's own `modelUsage` for that model.
