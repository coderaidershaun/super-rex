Status: FROZEN (10 November 2026, by Lane 1 at gate G1, on top of commit 136720b). A change to this file goes through Lane 1.

# C3. The hook: registration, and the line `rex hook` appends

- Written by: Lane 3 (Capture).
- Read by: Lane 4 (`rex init` writes the registration), Lane 1 (owns `main.rs` and the `hook` sub-command), Lane 2 (C1 gives the log path and the store search).
- Version: draft 1, 10 November 2026.
- Tested on: Claude Code 2.1.322, macOS.

## What this is for

A hook is a command that Claude Code runs by itself at a set point, such as when a sub-agent starts, and it passes the command a JSON description of the event on standard input. The model never sees this, so it costs no tokens. SuperRex registers one command, `rex hook`, for three events. `rex hook` appends each event as one line to the event log. Later, a `rex` command that reads a job takes the new lines into the store (C5). This contract fixes the registration, what `rex hook` does, and the line format.

Words used here:

- **Store**: the SQLite database of SuperRex, `.rex/rex.db`, in the main checkout of the repo. C1 (Lane 2) defines it.
- **Main checkout**: the repo folder that holds the `.git` folder, as opposed to a Git worktree of it.
- **Event log**: `.rex/events.jsonl`, an append-only text file beside the store. One line for each hook call.
- **Run**: one sub-agent, from its start to its stop, as the store records it (C5).

Marks: "tested" = seen on Claude Code 2.1.322 by a live probe; "docs" = from the Claude Code documentation, not tested here; "settled" = a decision of this contract, settled at gate G1.

## 1. The registration

`rex init` (Lane 4) merges exactly these three entries into the repo's `.claude/settings.local.json`. The value of the `hooks` key below is what C6 merges, by C6 rule 3a: for each event, IF any hook in the user's `hooks.<Event>` array already has the command string of rule 1, THEN nothing is added for that event; ELSE this matcher group is appended. So the registration can be held in code as one `serde_json::Value`.

```json
{
  "hooks": {
    "SubagentStart": [{ "hooks": [{ "type": "command", "command": "rex hook >/dev/null 2>&1 || true", "timeout": 10 }] }],
    "SubagentStop":  [{ "hooks": [{ "type": "command", "command": "rex hook >/dev/null 2>&1 || true", "timeout": 10 }] }],
    "PostToolUse":   [{ "matcher": "Agent|Task", "hooks": [{ "type": "command", "command": "rex hook >/dev/null 2>&1 || true", "timeout": 10 }] }]
  }
}
```

| Key | Value | Why |
| --- | --- | --- |
| `SubagentStart` | no `matcher` | Fires for every sub-agent, at every depth (tested). |
| `SubagentStop` | no `matcher` | Same. |
| `PostToolUse` | `matcher` `Agent\|Task` | Fires after a call of the sub-agent tool. Matches the tool names `Agent` and `Task` exactly; `ListAgents` and `TaskStop` did not match (tested). |
| `type` | `command` | A shell command. |
| `command` | `rex hook >/dev/null 2>&1 \|\| true` | See below. |
| `timeout` | `10` | Seconds (docs). The key is accepted and all three entries fire with it (tested). A hook that runs past it is killed; the agent waits that long, then goes on with the normal result; nothing is blocked or shown to the model (tested). So the worst case is 10 seconds for each event. `rex hook` takes milliseconds. |
| `async` | absent | An async hook still running when the session exits is killed (tested), so its event is lost. |

Rules:

1. The `command` must be exactly `rex hook >/dev/null 2>&1 || true`. Claude Code runs a hook command through a shell, so the suffix gives exit code 0 and no output whatever `rex` is found on `PATH`. Reason: another program named `rex` may be on `PATH` (rex-skills 0.1.9 installs one). It has no `hook` command, prints an error and exits 2 (tested). Exit 2 from `SubagentStop` stops the sub-agent from finishing and gives it the error text; exit 2 from `PostToolUse` gives the error text to the model (docs). Tested on a `PreToolUse` hook: a bare exit 2 blocked the call and gave the model the command line and its standard error; with this suffix the same exit 2 did not block.
2. An entry is SuperRex's own IF and only IF its `command` is exactly that string. `rex init` uses this to add each event's group one time only (C6 rule 3a) and to give the warning of C6 4.6. A group that the user changed, for example its `timeout`, still counts as SuperRex's own and is not added again.
3. The three entries are merged into a file the user may already have. Every key and every other hook in that file stays. The merge rule itself is in C6 (Lane 4).
4. IF the user runs `rex init --no-hooks` (C6 4.6), THEN `rex init` writes none of the three entries, and removes none that an earlier run wrote. Then no line is ever logged, the store has no runs, and the job status `running` is never shown. Jobs are still recorded, because their status changes only through `rex` commands.

## 2. What `rex hook` does

`rex hook` uses no arguments and no flags (it accepts and ignores any, section 7). It must:

1. Read all of standard input. Claude Code gives one compact JSON line ended by `\n` (tested, 34 of 34 events).
2. IF standard input is empty, THEN write nothing.
3. Find the store (section 3). IF none is found, THEN write nothing. It never creates a folder or a file other than the log: IF `.rex/rex.db` is absent, THEN it does nothing.
4. Build one line (section 4) and append it to the event log.
5. Exit with code 0.

It never:

- prints anything on standard output or standard error (standard output of a hook can be given to the model, which costs tokens);
- exits with a code other than 0: not on bad input, no store, a write error, or a panic. It sets a panic hook that prints nothing and runs its work inside `catch_unwind`, because the default panic hook prints to standard error and exits 101;
- opens the database (a writer lock held by another process would make the agent wait);
- looks at the event name. It appends ANY event it is given, so a later registration needs no change to the hook or to the line.

`hook::run` returns `Ok(())` on every path. The signature stays `pub fn run(cwd: &Path, options: HookOptions) -> Result<(), RexError>` (C11 handler shape).

## 3. How `rex hook` finds the store

`rex hook` calls C1's `StorePaths::discover(cwd)` (C1 3.2), the same function, in the same order, as every `rex` command, and appends to its `event_log`. `discover` returns the first store that exists, in this order:

1. `REX_STORE`, IF set: the path of a `.rex` folder, used as is. IF its `rex.db` is absent, THEN `discover` fails and never falls through.
2. `$CLAUDE_PROJECT_DIR`. Claude Code sets it for every hook process (tested). It is the folder the session was started in.
3. The working folder of the hook process.

For 2 and 3, `discover` walks up to the main checkout (from a sub-folder, and from a Git worktree to its main checkout), and uses the store there only IF `.rex/rex.db` exists. It reads files only, runs no `git`, and opens no database. When `discover` fails, `rex hook` writes nothing and exits 0.

Why the hook reads `REX_STORE` too: one function finds the store for every `rex` command and for the hook, so a session whose shell sets `REX_STORE` has its jobs and its events in the same store.

What the probe saw, in four cases (`<repo>` = the repo root):

| Case (no `REX_STORE`) | `$CLAUDE_PROJECT_DIR` | Working folder of the hook |
| --- | --- | --- |
| Session started in the repo root | `<repo>` | `<repo>` |
| Session started in `<repo>/sub` | `<repo>/sub` | `<repo>/sub` |
| Sub-agent in a worktree (`<repo>/.claude/worktrees/agent-…`) | `<repo>` | the worktree |
| Agent working in a sibling folder | `<repo>` | the sibling folder |

The working folder of the hook always equals the event's `cwd`. In the sibling case only the variable leads to the store. In the sub-folder case the variable is not the repo root, so the finder must walk up.

## 4. The event log

**Path.** `StorePaths::event_log` of C1: `.rex/events.jsonl`, beside `.rex/rex.db`, in the main checkout.

**Line format.** One event = one line, UTF-8, ended by `\n`:

- `v`: the line format version, the number `1`.
- `at_ms`: when the hook ran, in milliseconds since the Unix epoch, UTC. No event carries a time (tested), so the hook stamps it.
- `event`: the JSON object from standard input, every field kept, written compact (no raw newline inside the line).
- IF standard input is not a JSON object, THEN the line has `raw` in place of `event`: the input as one JSON string, with bytes that are not UTF-8 replaced by U+FFFD. Nothing is dropped.

Worked lines (trimmed: real events carry more fields; IDs and paths are made up):

```json
{"v":1,"at_ms":1794313980123,"event":{"session_id":"9b2e7c41-3d5a-4f8e-a1c6-2f7d0e9b4a13","transcript_path":"/home/user/.claude/projects/-home-user-work-repo/9b2e7c41-3d5a-4f8e-a1c6-2f7d0e9b4a13.jsonl","cwd":"/home/user/work/repo","agent_id":"a0c8cde396de066a3","agent_type":"model-medium-effort-low","hook_event_name":"SubagentStart"}}
```

```json
{"v":1,"at_ms":1794313992456,"event":{"session_id":"9b2e7c41-3d5a-4f8e-a1c6-2f7d0e9b4a13","cwd":"/home/user/work/repo","agent_id":"a0c8cde396de066a3","agent_type":"model-medium-effort-low","effort":{"level":"low"},"hook_event_name":"SubagentStop","agent_transcript_path":"/home/user/.claude/projects/-home-user-work-repo/9b2e7c41-3d5a-4f8e-a1c6-2f7d0e9b4a13/subagents/agent-a0c8cde396de066a3.jsonl","last_assistant_message":"LEAD DONE"}}
```

```json
{"v":1,"at_ms":1794313992490,"event":{"session_id":"9b2e7c41-3d5a-4f8e-a1c6-2f7d0e9b4a13","cwd":"/home/user/work/repo","hook_event_name":"PostToolUse","tool_name":"Agent","tool_input":{"description":"lead","prompt":"Rex-Job: k3x9p2m7qd\nRex-Role: lead\n\nRead notes.txt.","subagent_type":"model-medium-effort-low","run_in_background":false},"tool_response":{"status":"completed","agentId":"a0c8cde396de066a3","resolvedModel":"claude-sonnet-5-5","totalDurationMs":12333,"totalToolUseCount":4,"toolStats":{"readCount":1,"searchCount":0,"bashCount":1,"editFileCount":1,"linesAdded":4,"linesRemoved":3,"otherToolCount":0},"content":[{"type":"text","text":"LEAD DONE"}]}}}
```

```json
{"v":1,"at_ms":1794313993001,"raw":"not json"}
```

**Append rule.** `rex hook` builds the whole line in memory, ended by `\n`, opens the log with `OpenOptions::new().append(true).create(true)`, and writes the line with ONE `write` call. It does not use `write_all`, which can split the line into several calls that lines of other hooks can come between. No lock file, no `flock`. IF that call writes fewer bytes than the line (for example on a full disk), THEN `rex hook` makes one more `write` of a single `\n` and stops. So the damage stays in that one line, and the next hook's line starts on a line of its own. Reason: Linux and macOS put each append-mode write to a local file at the end of the file as one step, so lines from hooks that run at the same moment do not mix, and a hook never waits. POSIX does not promise this for large writes on every file system, so Lane 3's tests prove it on each CI platform (32 hooks at once, each line over 200 KB). IF that test fails on a platform, THEN `rex hook` takes a `flock` with a wait of about 100 ms and then appends in any case.

**What a reader of the log must tolerate.**

- A damaged line (not valid JSON, or no `v`): count it and skip it. A short write, for example on a full disk, can leave one.
- A line whose `v` is not `1`: skip it and count it with the damaged lines.
- Bytes after the last `\n`: an unfinished line. Leave them for the next read.
- An event name it does not know, and a `raw` line: skip it.
- A field it does not know: ignore it.

The log is never truncated or rotated in version 1.

## 5. The fields intake uses

From the probe and the recorded fixtures (`tests/fixtures/capture/`). "Caller" = the agent that made the `Agent` call.

| Event | Fields intake uses |
| --- | --- |
| `SubagentStart` | `session_id`, `transcript_path` (the main session's transcript), `agent_id`, `agent_type`. No model, no parent, no brief, no time. |
| `SubagentStop` | `session_id`, `transcript_path`, `agent_id`, `agent_type`, `effort.level` (the effort that ran), `last_assistant_message`, `agent_transcript_path` (the sub-agent's own transcript). No model. |
| `PostToolUse`, `tool_name` `Agent`, `tool_response.status` `completed` (a foreground call returned) | `session_id`, `transcript_path`, `agent_id` (the caller; absent when the caller is the main session), `tool_input.prompt` (the brief), `tool_input.subagent_type`, `tool_input.run_in_background`, `tool_response.agentId` (the child), `tool_response.resolvedModel`, `tool_response.totalDurationMs`, `tool_response.totalToolUseCount`, `tool_response.toolStats` (`readCount`, `searchCount`, `bashCount`, `editFileCount`, `linesAdded`, `linesRemoved`, `otherToolCount`; absent when the run called no tool), `tool_response.content` (final text blocks). |
| `PostToolUse`, `tool_name` `Agent`, `tool_response.status` `async_launched` (a background call was launched) | `session_id`, `transcript_path`, `agent_id` (the caller), `tool_input.prompt`, `tool_input.subagent_type`, `tool_input.run_in_background` (`true`), `tool_response.agentId`, `tool_response.resolvedModel`. No `toolStats`, `usage`, `content` or duration. |

- Intake converts `at_ms` to the store's time text (`YYYY-MM-DDTHH:MM:SS.sssZ`, C1) in SQL.
- `tool_response.usage` and `tool_response.totalTokens` cover the last request only. Intake must never use them as a run's tokens; tokens come from transcripts (C5).
- Every event from inside a sub-agent carries that sub-agent's `agent_id`. Events from the main session have no `agent_id` at all.
- Order for a foreground child: `SubagentStart` (child), the child's own events, `SubagentStop` (child), then `PostToolUse` in the caller.
- Order for a background child: the `async_launched` line can come BEFORE that run's `SubagentStart` (seen in the fixture, with the same `at_ms`). A reader must not assume the start line is first.

## 6. Open points

### OP9 — settled: which events are registered, and how the command finds `rex`

Answer: three registrations only, as section 1 gives them: `SubagentStart` (no matcher), `SubagentStop` (no matcher), `PostToolUse` with matcher `Agent|Task`. Each runs exactly `rex hook >/dev/null 2>&1 || true`, with `timeout` 10 and no `async`, and finds `rex` on `PATH`. `PostToolUseFailure` is not registered. No hook for skills opened.

**(1) `PostToolUseFailure` is not registered.** Reason: it adds no fact. Evidence (tested): an `Agent` call that fails before a run starts (an unknown agent type) fires ONLY `PostToolUseFailure`: no start, no stop, no completed call, no transcript, no metadata file. No run exists, so nothing is lost. A run that fails after it started leaves a transcript and a metadata file, and intake reads those for background runs and lost events in any case (C5). Not tested: a failure or an interrupt in the middle of a run. Alternative not chosen: register it "to be safe". It costs a hook call and a line for each failure and gives intake nothing to set.

**(2) Skills opened: no registration.** The skills a run opened are read from its transcript, which `job_runs` reads in any case for tokens. The rule is in C5, section 4.

Reason: a hook adds no fact and has a cost. Evidence (tested): the transcripts hold each `Skill` call with the skill name and each `Read` call with its path and its result, in the main session and in sub-agents. What the hook options gave, in a session where agents loaded a skill by the Skill tool and read a `SKILL.md`, a second file of the skill and a plain file:

| Registration | Skill tool | Read of a `SKILL.md` | Read of another file | Size of the event |
| --- | --- | --- | --- | --- |
| `InstructionsLoaded`, no matcher | no | no | no | 446 bytes |
| `PreToolUse`, matcher `Read\|Skill` | yes | yes | yes | 520 to 676 bytes |
| `PostToolUse`, matcher `Read\|Skill` | yes | yes | yes | up to 1060 bytes (holds the whole file) |
| `PreToolUse`, matcher `Read`, `"if": "Read(**/SKILL.md)"` | not matched | yes | no | 605 to 672 bytes |
| `PostToolUse`, matcher `Read`, `"if": "Read(**/SKILL.md)"` | not matched | yes | no | 942 to 1014 bytes (holds the whole file) |

- `InstructionsLoaded` fires once, at session start, for `CLAUDE.md` only.
- A hook with the `if` filter works, but it costs one process for each skill read; a `PostToolUse` line holds the whole skill file, so the log grows fast; a `PreToolUse` line also records a read that then fails, which is the case feedback most needs to see correctly.

Alternative not chosen: two entries (`Skill`, and `Read` with the `if`), on `PreToolUse` or `PostToolUse`, for the reasons above. IF a later version adds a hook after all, THEN it is one more registration; `rex hook` and the line format stay as they are, because the hook appends any event.

Limits: a skill read by a shell command, or preloaded by an agent definition's `skills:` field, is not seen (a hook would not see it either). The transcript must still exist at the first read.

**(3) The command finds the binary by `PATH`.** The command is `rex hook`, not an absolute path. Reasons: (a) skills and agents call `rex` by `PATH` too, so the hook and every other command are the same binary and the same store version; (b) an absolute path goes stale when `rex` is installed again in another place, and then every hook fails with no sign; (c) the settings file holds no machine path. Cost: IF `rex` is not on the `PATH` of the Claude Code process, THEN the hook fails, Claude Code goes on (the suffix makes it exit 0), and no event is logged. `rex init` therefore gives a warning, and only a warning, when the `rex` on `PATH` is absent or is not the binary that runs `init` (section 7; C6 section 2 holds the text). Alternative not chosen: an absolute path written by `rex init`, for reasons (b) and (c). Lane 4 (C6) also asked for the command by `PATH`: with an absolute path, C6's merge rule would register a second entry each time the binary moves, and every event would be logged twice.

### OP13 — settled (hook part): the store when the agent works in another folder

Answer: one rule for every `rex` command and for `rex hook`: C1's `StorePaths::discover(cwd)` (section 3): `REX_STORE`, then `$CLAUDE_PROJECT_DIR`, then its working folder. The hook reads `REX_STORE` too (one function; a session whose shell sets `REX_STORE` has its jobs and its events in the same store). No `--store` argument, and `rex init` writes no `env` key (C6 4.1). C3 agrees with C1's OP13 (C1 11.2).

Reason: the session's start folder is where `rex init` registered the hook, so its store is the right one for every event of that session, wherever the agent has gone. Evidence (tested): the variable is set for every hook process and stays the session's start folder for a worktree sub-agent and for an agent in a sibling folder; in the sibling case the working folder does not lead to the store. Alternative not chosen: a `--store <path>` argument written into the registration. It breaks when the repo is moved, and the probe showed it is not needed.

The same question for a `rex` command that an agent runs in its shell from a sibling folder belongs to C1 (Lane 2). Fact for C1 (tested): `CLAUDE_PROJECT_DIR` is NOT set in the Bash tool, in the main session, a sub-agent, a worktree sub-agent, or a sub-agent in a sibling folder. Only hooks get it.

## 7. What each other lane must do

**Lane 4 (`rex init`, C6).**

- Write exactly the three entries of section 1, merged into the existing `.claude/settings.local.json`.
- Find SuperRex's own entries by the exact `command` string, and add each event's group one time only (C6 rule 3a).
- Under `--no-hooks`, write none of the three entries and remove none (C6 4.6).
- At `rex init`, give a warning on stderr IF no `rex` is found on `PATH`, or IF the first one found is not the binary that runs `init`. It is a warning only: the install goes on. C6 section 2 holds the method and the text.
- A session started inside a job worktree under the main checkout fires the hooks, because it gets the settings of the main checkout (tested live). A worktree outside the main checkout's folder is not tested: tell the user to start a session in the main checkout. (Worktrees that Claude Code makes for sub-agents are not affected: the session's settings apply.)

**Lane 2 (C1).** Nothing more is asked. C1 gives what the hook needs: `StorePaths::discover(cwd)` (opens no database, runs no `git`) and `StorePaths::event_log`.

**Lane 1 (spine, `main.rs`).** Nothing more is asked. The `hook` sub-command is in place: `HookOptions` accepts and ignores every argument and has no help flag, so clap never fails on `rex hook …`. Keep it so. `hook::run` always returns `Ok(())` (section 2), so `main.rs` needs no special case.

## 8. What was tested and what was not

Tested on Claude Code 2.1.322, macOS, in headless sessions with the hooks only in the repo's `.claude/settings.local.json`:

- the three entries with `"timeout": 10` fire, once for each sub-agent, at depth 1 and 2, for foreground, background and worktree sub-agents;
- hooks in the repo root's settings fire for a session started in a sub-folder;
- the matcher matches `Agent` exactly, not `ListAgents` or `TaskStop`;
- standard input is one compact line ended by `\n`;
- `$CLAUDE_PROJECT_DIR` and the hook's working folder in the four cases of section 3; the variable is not set in the Bash tool;
- a failed `Agent` call (unknown agent type) fires only `PostToolUseFailure`;
- an async hook running at session exit is killed;
- rex-skills 0.1.9 run as `rex hook` exits 2 with an error on standard error;
- a hook past its `timeout` is killed and delays the agent by the timeout only; a bare exit 2 from `PreToolUse` blocks the call, the suffixed form does not;
- the skills-opened options of OP9.

Not tested: Linux; a failure or an interrupt in the middle of a run; a `PATH` with no `rex`; the `PATH` of Claude Code when the desktop app or an IDE starts it; a bare exit 2 from `PostToolUse` or `SubagentStop`; a session started with `claude --worktree`; 32 large appends at the same moment (Lane 3's tests prove it). The check notes do not say which event the timeout probe ran on. A time-out on the other two events is not tested; appendix B (docs) says a time-out can block a call. The Git-worktree limit in section 7 is reasoned from Git's ignore rule, not tested.

## 9. Limits

- The log grows by about three lines for each sub-agent and is never truncated in version 1.
- IF `rex` is not on the `PATH` of Claude Code, or is not SuperRex, THEN no event is logged. Nothing fails. `rex init` warns about it (C6 section 2).
- A short write (full disk) can leave one damaged line. Intake counts it; recovery from transcripts fills the gap (C5).
- A session started inside a job worktree under the main checkout (`<repo>/.rex/worktrees/<name>`, made with `git worktree add`) gets the settings of the main checkout: the sandbox applies and the hooks fire (tested live, Claude Code 2.1.322). A worktree outside the main checkout's folder is not tested: start a session in the main checkout.
- A `REX_STORE` that the user's shell exports applies to the hook too: every event of the session goes to that store.
