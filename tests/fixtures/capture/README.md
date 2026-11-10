# Capture fixtures

Real hook events, transcripts and sub-agent metadata files from one headless Claude Code
session, with everything private removed. Tests for `rex hook`, intake and the transcript
reader run against them.

- Recorded: 2026-11-10.
- Claude Code: 2.1.322 (`claude --version`).
- Platform: macOS 15.7.9, Darwin arm64.
- Session ID: `041bd529-f189-451f-9eaa-239f46bb7b36`. Recorded in one try. Cost $0.19 by Claude Code's own estimate.

## How it was recorded

- A new throwaway Git repo (two small files, `notes.txt` and `bg.txt`), with the 20 grid agent
  definitions copied into `.claude/agents/` and an empty `.rex/` folder.
- Hooks registered only in the repo's `.claude/settings.local.json`: `record/settings.local.json`.
  It is the registration that `rex init` will write (three events, `"timeout": 10`), with
  `record/hook-log.sh` in place of `rex hook`. The script writes the line `rex hook` will write:
  `{"v":1,"at_ms":<ms since epoch>,"event":<the hook's standard input, as it came>}`.
- The command line and the exact prompt: `record/prompt.md`. Flags that keep the owner's
  setup out of the session: `--setting-sources project,local` (no user settings, so no user
  hooks, plugins or user `CLAUDE.md`) and `--strict-mcp-config --mcp-config '{"mcpServers":{}}'`
  (no MCP servers). Both were accepted; the login and the hooks worked with them.
- After the session ended, the files were checked to have stopped changing (transcripts are
  written a little after the stop event), then copied out and run through `record/sanitize.py`.
  `record/expected.py` then wrote `tree/expected.json` from the sanitized files.

## The scenario

The main session (sonnet, effort low) started four agents in this order, and waited for the
background one to finish. Every brief except PLAIN's starts with the tag block
`Rex-Job: k3x9p2m7qd` / `Rex-Role: <role>` / blank line.

| Name | Agent ID | Agent type | Started by | Depth | Run | Role tag | Model | Effort | What it did |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| BG | `a70d8a0f630d8701c` | `model-low-effort-medium` | main | 1 | background | `background-worker` | haiku | medium | `sleep 5`, edited `bg.txt`, started BG-CHILD |
| BG-CHILD | `a0741c86685f0104e` | `model-low-effort-low` | BG | 2 | foreground | `worker` | haiku | low | read `bg.txt` |
| LEAD | `acefc7f2b8d9b97bf` | `model-medium-effort-low` | main | 1 | foreground | `lead` | sonnet | low | read `notes.txt`, `wc -l`, one Edit (3 lines -> 4), started WORKER |
| WORKER | `a8b103ab4ef48cdf5` | `model-low-effort-low` | LEAD | 2 | foreground | `worker` | haiku | low | wrote `worker.txt` |
| REVIEWER | `ab99f22c637e9bce7` | `model-high-effort-low` | main | 1 | foreground | `reviewer` | opus | low | read `notes.txt` |
| PLAIN | `a2caf7515d48b82f2` | `general-purpose` | main | 1 | foreground | none | sonnet | low | no tool |

REVIEWER and PLAIN ran at the same time. The session gave one result record.

## Files

```
record/hook-log.sh            the hook script used
record/settings.local.json    the hook registration used
record/prompt.md              the command line and the exact prompt
record/sanitize.py            raw copies -> tree/ or skills/ (removes and replaces private data)
record/expected.py            tree/ -> tree/expected.json; `expected.py skills skills` -> skills/expected.json
tree/events.jsonl             the event log: 18 lines, one for each hook call
tree/result.json              the session's final result record(s), as a JSON array
tree/expected.json            the tree, roles, models, efforts, request counts, tokens, toolStats
tree/transcripts/<session>.jsonl                          the main session's transcript
tree/transcripts/<session>/subagents/agent-<id>.jsonl     one transcript for each sub-agent
tree/transcripts/<session>/subagents/agent-<id>.meta.json one metadata file for each sub-agent
skills/                       a second, smaller session: which skills each agent opened (see below)
```

`expected.json`: the sum of tokens over the main transcript and the six sub-agent
transcripts, with each `message.id` counted once, equals `modelUsage` of the result record
for all three models (`sum_equals_model_usage` is true for each).
Each agent also has `transcript_counts` (`tool_uses`, `lines_added`, `lines_removed`), worked out from its own transcript by the rule under "Facts" below;
for each foreground run the script stops with an error IF these differ from the run's `totalToolUseCount` and `toolStats`, so BG's counts (no event gives them) come from the same checked rule.

## What was removed or replaced

Replaced, in every file (the values are read on the recording machine at run time):

- the throwaway repo path -> `/home/fixture/work/repo`; the sibling folder path -> `/home/fixture/work/sibling` (not used in this session);
- the project folder name under `~/.claude/projects/` -> `-home-fixture-work-repo`;
- the home folder -> `/home/fixture`;
- `/private/tmp/claude-<uid>/` -> `/tmp/claude-fixture/`;
- the user name, the Git user name and email, and the host name, where they stand alone -> `fixture`;
- each `msg_…` message ID and each `req_…` request ID -> `msg_fixtureNNNN` / `req_fixtureNNNN`. One placeholder for one ID in every file, so equal IDs stay equal (the records of one request share one `message.id`) and different IDs stay different.

Kept as they are, because they are random and the files point at each other by them:
`session_id`, `agent_id`, `tool_use_id`, `toolUseId`, `uuid`, `parentUuid`, `promptId`.
Hook events are kept whole (only the replacements above).

Removed from the transcripts:

- whole records of type `api-request-blob` (request bodies: system prompt and messages), `api-request-shape` (system prompt and tool definitions), `attachment` (skill, agent and tool listings, environment, organisation, budget), `session-owner` (account and organisation IDs), `queue-operation` and `last-prompt` (copies of the prompt), `atis-latch`. Any record type not named in the kept list is dropped;
- from `api-request`: `base`, `tail`, `cacheControl`, `keep`, `requestDigest`, `shapeHash`, and `params` except `model`, `max_tokens`, `thinking`, `output_config`;
- from `assistant`: `advisorModel`, `promptCacheRequested`, `requestRef`, `thinkingDisplay`, `thinkingDurationMs`, `toolInputStreamMs`, `wireToolInputs`; from its `message`: `container`, `context_management`, `diagnostics`, `input_transformations`, `stop_details`; the `signature` of thinking blocks;
- from `user`: `permissionDecision`, `sandboxed`, `turnCompanion` (`skills/` only).

Kept in the transcripts: record type, IDs, `timestamp`, `message.id`, `message.model`,
`message.usage`, every content block (tool calls with their input, tool results, text, the
brief as the first user text, the final text), `toolUseResult` (main transcript only),
`effort`, `perTurnEffort`, `requestedModel`, `cost-state`, `tools-used`, and in `skills/` also
`attributionSkill` and `sourceToolUseID`.

Removed from the result record: `api_error_status`, `credential_expires_at`,
`fast_mode_disabled_reason`, `fast_mode_state`, `first_content_frame_ms`,
`queued_turn_count`, `safety_stops`, `time_to_request_ms`, `ttft_ms`, `ttft_stream_ms`.
Only `type: "result"` records of the stream were kept.

## One command for each case

Run from this folder. The outputs are the real ones.

All six sub-agents started and stopped, and each has a completed-call event:

```
$ grep -c '"hook_event_name":"SubagentStart"' tree/events.jsonl
6
$ grep -c '"hook_event_name":"SubagentStop"' tree/events.jsonl
6
$ jq -r 'select(.event.tool_name=="Agent") | .event.tool_response.status' tree/events.jsonl | sort | uniq -c
   1 async_launched
   5 completed
```

A sub-agent that starts a sub-agent (the caller's `agent_id` -> the child):

```
$ jq -r 'select(.event.tool_name=="Agent" and .event.agent_id != null) | "\(.event.agent_type) \(.event.agent_id) -> \(.event.tool_response.agentId)"' tree/events.jsonl
model-low-effort-medium a70d8a0f630d8701c -> a0741c86685f0104e
model-medium-effort-low acefc7f2b8d9b97bf -> a8b103ab4ef48cdf5
```

The background sub-agent (launch event, its stop event, its metadata file):

```
$ jq -r 'select(.event.tool_response.status=="async_launched") | .event.tool_response.agentId' tree/events.jsonl
a70d8a0f630d8701c
$ jq -r 'select(.event.hook_event_name=="SubagentStop" and .event.agent_id=="a70d8a0f630d8701c") | .event.last_assistant_message' tree/events.jsonl
BG DONE
$ jq -r .requestShape tree/transcripts/*/subagents/agent-a70d8a0f630d8701c.meta.json
background
```

Sub-agents started from grid agents, with the model that ran:

```
$ jq -r 'select(.event.tool_name=="Agent") | "\(.event.tool_input.subagent_type) \(.event.tool_response.resolvedModel)"' tree/events.jsonl | sort | uniq -c
   1 general-purpose claude-sonnet-5-5
   1 model-high-effort-low claude-opus-5-5
   2 model-low-effort-low claude-haiku-5-5
   1 model-low-effort-medium claude-haiku-5-5
   1 model-medium-effort-low claude-sonnet-5-5
```

The tag block at the start of the brief (five tagged, PLAIN not):

```
$ jq -r 'select(.event.tool_name=="Agent") | .event.tool_input.prompt | test("^Rex-Job: k3x9p2m7qd\nRex-Role: [a-z-]+\n\n")' tree/events.jsonl | sort | uniq -c
   1 false
   5 true
$ jq -r 'select(.event.tool_name=="Agent") | "\(.event.tool_input.description): \(.event.tool_input.prompt | split("\n")[0:2] | join(" | "))"' tree/events.jsonl
bg: Rex-Job: k3x9p2m7qd | Rex-Role: background-worker
bg-child: Rex-Job: k3x9p2m7qd | Rex-Role: worker
worker: Rex-Job: k3x9p2m7qd | Rex-Role: worker
lead: Rex-Job: k3x9p2m7qd | Rex-Role: lead
reviewer: Rex-Job: k3x9p2m7qd | Rex-Role: reviewer
plain: You are a test agent. Do not use any tool. Reply with the one line PLAIN DONE.
```

## Facts these files show (for the reader and intake)

- Lines added and removed in `toolStats` are not a line diff. An Edit adds the number of lines of `new_string` and removes the number of lines of `old_string`; a Write adds the number of lines of `content`, where lines = pieces after splitting on `\n` (so a final newline counts as one more line). LEAD: one Edit of 3 lines to 4 lines -> `linesAdded 4, linesRemoved 3`. WORKER: Write `"w1\nw2\n"` -> `linesAdded 3`.
- `editFileCount` counts Edit and Write. The `Agent` call is counted in `totalToolUseCount` but in no `toolStats` group (LEAD: 3 in groups, total 4).
- A run with no tool call has no `toolStats` key and `totalToolUseCount: 0` (PLAIN).
- Sub-agent transcripts have no `toolUseResult`. The main transcript has it; for an `Agent` call it repeats the hook's `tool_response`.
- The metadata file gets `finished: true` when the run ends (absent before). `parentAgentId` appears only at depth 2. `model` is in BG-CHILD's file only: not in WORKER's (also depth 2), and in no depth-1 file. `pickedEffortModel` is the main session's model, not the run's.

## The `skills` fixture

Skills opened are read from the transcript; no hook.

- Session `a0a5f6eb-59bf-4dc8-96aa-38434efdcda0`, recorded 2026-11-10 with Claude Code 2.1.322 on the same machine, with the same command line and flags as `tree/` (see `record/prompt.md`). It was a hook probe, so its hook log had another format. This fixture has NO `events.jsonl` and NO `result.json`: only transcripts, metadata files and `expected.json`.
- The throwaway repo held `.claude/skills/probe-skill/SKILL.md` (with `reference.md` beside it), `.rex/orchestrators/probe-orch/SKILL.md` (outside `.claude/skills/`, read as a plain file), `plain.txt` and a project `CLAUDE.md`.

| Agent | Agent ID | Agent type | How it opened a skill | `skills_opened` |
| --- | --- | --- | --- | --- |
| main session | (none) | (main, sonnet) | Skill tool `probe-skill`; Read of `.rex/orchestrators/probe-orch/SKILL.md` | `["probe-skill", "probe-orch"]` |
| s1-skill | `a5360226e437795bf` | `model-low-effort-low` | Skill tool `probe-skill` | `["probe-skill"]` |
| s2-read | `ae4856bf1b48c7b0b` | `model-low-effort-low` | Read of `.claude/skills/probe-skill/SKILL.md`; also Read `reference.md` (the same skill again, so it adds nothing) and `plain.txt` (not Markdown) | `["probe-skill"]` |

`skills/expected.json` holds, for the main session and each sub-agent, every tool call as `{name, skill or file_path, failed}` and `skills_opened`. The rule for `skills_opened`:
- take the `input.skill` of each `Skill` call;
- for each `Read` whose `tool_result` (same `tool_use_id`) is not `is_error: true`, add the name of the folder holding the file when the `file_path` ends in `/SKILL.md`, or `<name>` when it is another `.md` file at any depth below `.claude/skills/<name>/` or `.rex/orchestrators/<name>/`. A Markdown file anywhere else, such as `.claude/skills/README.md`, adds nothing;
- list each name once, in order of first use.

No Read failed in this session, so the "failed" branch has no case here.

The Skill call and the SKILL.md Reads are in the transcripts:

```
$ jq -r 'select(.type=="assistant") | .message.content[] | select(.type=="tool_use" and (.name=="Skill" or .name=="Read")) | "\(input_filename | split("/") | last)  \(.name)  \(.input.skill // .input.file_path)"' skills/transcripts/*.jsonl skills/transcripts/*/subagents/*.jsonl
a0a5f6eb-59bf-4dc8-96aa-38434efdcda0.jsonl  Skill  probe-skill
a0a5f6eb-59bf-4dc8-96aa-38434efdcda0.jsonl  Read  /home/fixture/work/repo/.rex/orchestrators/probe-orch/SKILL.md
agent-a5360226e437795bf.jsonl  Skill  probe-skill
agent-ae4856bf1b48c7b0b.jsonl  Read  /home/fixture/work/repo/.claude/skills/probe-skill/SKILL.md
agent-ae4856bf1b48c7b0b.jsonl  Read  /home/fixture/work/repo/.claude/skills/probe-skill/reference.md
agent-ae4856bf1b48c7b0b.jsonl  Read  /home/fixture/work/repo/plain.txt
```

Two traps these files show:
- A Skill call's `tool_result` is only `Launching skill: probe-skill`. The skill's text comes in a separate `user` record whose `sourceToolUseID` is the call's ID; it starts with `Base directory for this skill: …`.
- `attributionSkill` on assistant records is NOT "skills opened". Every record of both sub-agents says `probe-skill`. That includes s2-read, which never used the Skill tool, and s1-skill's first record, which comes before its Skill call. It is carried over from the main session.

To build it again from the raw copies: put the main transcript and its `subagents/` folder under `<raw>/transcripts/`, then run `python3 -I record/sanitize.py <raw> skills <repo path> <sibling path>` and `python3 -I record/expected.py skills skills`.

## How to record again

1. Make a new folder outside any real repo, `git init` it, add `notes.txt` (`alpha`, `beta`, `gamma`, one per line) and `bg.txt` (`one`), commit, and `mkdir .rex`.
2. Copy the grid agents into `.claude/agents/`, `record/hook-log.sh` into `.claude/hooks/`, and `record/settings.local.json` into `.claude/`. When `rex hook` exists, use `"command": "rex hook"` instead and the hook script is not needed.
3. Save the prompt from `record/prompt.md` to a file. To change the tag lines, edit them in that prompt only.
4. Start the session with the command line in `record/prompt.md`, from that folder, with no `CLAUDE*` variables of an outer session set.
5. Check: six `SubagentStart`, six `SubagentStop`, six `PostToolUse` Agent, one `async_launched`, five tagged briefs (the commands above). If the model did not obey, record again.
6. Wait until the transcripts stop changing. Copy into one raw folder: `.rex/events.jsonl` as `events.jsonl`; the `type: "result"` lines of the stream as `result.jsonl`; `~/.claude/projects/<project>/<session>.jsonl` and `<session>/subagents/` under `transcripts/`.
7. `python3 -I record/sanitize.py <raw folder> tree <repo path> <sibling path>` (remove the old `tree/` first), then `python3 -I record/expected.py tree`.
8. Search `tree/` for private values (home path, user name, account values, key patterns), update the agent IDs in this file, then remove the throwaway folder and its folders under `~/.claude/projects/` and `/private/tmp/claude-<uid>/`.
