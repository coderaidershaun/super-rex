Status: DRAFT

# C4. The tag lines: job ID and role at the start of a brief

- Written by: Lane 3 (Capture).
- Read by: Lane 9 (builder and orchestrator skills), Lane 10 (lead and delivery skills).
- Version: draft 1, 10 November 2026.
- Tested on: Claude Code 2.1.322, macOS.

## What this is for

A **brief** is the text an agent gives to a sub-agent when it starts it: the `prompt` of the `Agent` tool. The **tag block** is two short lines at the very start of the brief. They tell `rex` which role the sub-agent fills, and which job the starting agent thinks it works for. Hook events do not carry the role, so without the block a run is recorded with no role. The block costs the agents two lines of text and nothing else.

A **run** is one sub-agent, from its start to its stop, as the store (the SQLite database of SuperRex) records it. A **job** is one request, opened by `rex job create`.

## The format

```
Rex-Job: k3x9p2m7qd
Rex-Role: engineer

<the rest of the brief>
```

Rules:

1. The tag block must be at the very start of the brief. Empty lines before it are skipped.
2. A tag line matches the regular expression `^Rex-[A-Za-z][A-Za-z-]*: \S.*$`. A `\r` at the end of a line is ignored.
3. The tag block is the first lines that match. It ends at the first line that does not match, blank or not.
4. A blank line after the block is recommended, so that the brief's first sentence is never read as part of it.
5. The known keys are `Rex-Job` and `Rex-Role`. Keys are case-sensitive: `rex-job` is not a tag line, and `Rex-job` is an unknown key.
6. The order of the lines is free.
7. IF a key is given twice, THEN the first one wins.
8. An unknown `Rex-…` key is ignored. So keys can be added later without breaking old readers.
9. The value is the text after `: `, with spaces at both ends removed.
10. `Rex-Job:` is the same key as the trailer that delivery puts on the squash commit.

Values:

- `Rex-Job`: the job ID exactly as `rex job create` printed it. C2 (Lane 2) defines its form: 10 characters, lower-case letters and digits, for example `k3x9p2m7qd`.
- `Rex-Role`: the role name exactly as in the role table given to `rex job plan`. Role names should use lower-case letters, digits and `-` only, for example `engineer`, `code-lead`.

## Examples

Normal block. Role `engineer`, job `k3x9p2m7qd`:

```
Rex-Job: k3x9p2m7qd
Rex-Role: engineer

Add the parser for the config file.
```

Order reversed, with an unknown key. Role `reviewer`, job `k3x9p2m7qd`; `Rex-Note` is ignored:

```
Rex-Role: reviewer
Rex-Job: k3x9p2m7qd
Rex-Note: second pass

Read the diff and report.
```

A key given twice. Role `lead` (the first wins), job `k3x9p2m7qd`:

```
Rex-Job: k3x9p2m7qd
Rex-Role: lead
Rex-Role: worker

Plan the change, then start the workers.
```

No block. No role, no tag job:

```
Find every caller of open_store.
```

A block that is not at the start is not a block. No role, no tag job:

```
Please do the task below.
Rex-Job: k3x9p2m7qd
Rex-Role: engineer
```

## Where `rex` reads it, and what each tag does

- First source: `tool_input.prompt` of the hook event that Claude Code sends after an `Agent` call. Foreground calls (status `completed`) and background calls (status `async_launched`) both carry it.
- Second source, when that event was lost: the first user record of the sub-agent's own transcript.

`Rex-Role` gives the run's role.

`Rex-Job` is recorded and compared with the job the run is tied to. It NEVER moves a run to another job. The tie comes from two rules that use only facts Claude Code gives (the session ID and times):

- A run is tied to the job of its session that was open when the run started.
- The tie is written once and kept, also after that job closed and another opened.

Reason: a lead may leave the block out of a worker's brief or copy a wrong one, and the brief is not in the start event. IF `Rex-Job` differs from the tied job, THEN the run keeps its tie and the read model (C5) shows the tag job beside it.

## What happens when the block is absent or wrong

Nothing fails. No `rex` command refuses a brief.

- No block: the run is recorded with no role.
- A role that is not in the job's role table: recorded as given, as text.
- A `Rex-Job` that differs from the tied job: the run stays where the tie rules put it; the tag job is shown beside it.

## What Lanes 9 and 10 must put in their skills

- The orchestrator (Lane 9) starts every brief it writes with the block: `Rex-Job` = the job ID that `rex job create` printed, `Rex-Role` = the role from the role table, then a blank line.
- Any skill of Lanes 9 and 10 that tells an agent to start a sub-agent says: the starting agent copies `Rex-Job` from its own brief (or uses the ID that `rex job create` printed) and writes the new sub-agent's own role as `Rex-Role`.
- A sub-agent that the builder starts before the plan is recorded (for example one that writes a skill) still gets `Rex-Job` and a role name such as `skill-writer`. That role is not in a role table; it is recorded as text, and nothing fails.
- The skills must not tell agents to change or check the tags later. `rex` reads them by itself.

## What was tested and what was not

Tested on Claude Code 2.1.322, macOS: a recorded session whose briefs start with the block, for runs in the foreground, in the background, and at depth 2, plus one brief with no block. The brief arrives unchanged in `tool_input.prompt` of the `Agent` event, in both forms (`completed` and `async_launched`), and as the first user text of the sub-agent's transcript.

Proposed, not yet built: the parsing rules above. The two variants (a key given twice; the order reversed with an unknown key) are proved by Lane 3's tests, which write them into two recorded briefs. Not tested: briefs with `\r\n` line ends.

## Limits

- The role is only as good as the brief. An agent that leaves the block out gives a run with no role.
- `Rex-Job` is a cross-check, not a source of the tie.
