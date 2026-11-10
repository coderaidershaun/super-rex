Status: FROZEN (10 November 2026, by Lane 1 at gate G1, on top of commit 136720b). A change to this file goes through Lane 1.

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

1. The tag block must be at the very start of the brief. Lines before it that are empty or hold only spaces, tabs or `\r` are skipped.
2. A line is a tag line IF it has this form: `Rex-`, then a letter, then letters or `-`, then `:`, then any spaces or tabs, then at least one character that is not a space. As a regular expression: `^Rex-([A-Za-z][A-Za-z-]*):[ \t]*(\S.*)$`. The code checks this form by hand; it adds no `regex` crate (C11 section 7). A `\r` at the end of a line is ignored. Writers put exactly one space after the colon.
3. The tag block is the first lines that match. It ends at the first line that does not match, blank or not.
4. A blank line after the block is recommended, so that the brief's first sentence is never read as part of it.
5. The known keys are `Rex-Job` and `Rex-Role`. Keys are case-sensitive: `rex-job` is not a tag line, and `Rex-job` is an unknown key.
6. The order of the lines is free.
7. IF a key is given twice, THEN the first one wins.
8. An unknown `Rex-…` key is ignored. So keys can be added later without breaking old readers.
9. The value is the text after the colon, with spaces and tabs at both ends removed.
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
- Second source, when that event is absent: the first user record of the sub-agent's own transcript. The event is absent when it was lost, and also while a foreground run is still going, because its `completed` event comes only when the run returns (C5 section 4). So `rex status` can give the role of an open foreground run.

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
- A run that starts while its session has no open job: not stored at all, whatever the tags say (C5 section 3).
- A `Rex-Job` value that is not of C2's job-ID form, or a `Rex-Role` value that is not of `[a-z0-9-]+`: recorded as given, as text. `rex` does not correct it or change it to lower case.

## What Lanes 9 and 10 must put in their skills

- Every brief that starts a sub-agent starts with the block, as its first characters: `Rex-Job: <job ID>`, a line end, `Rex-Role: <role>`, a line end, one blank line, then the brief. No heading, quote mark or code fence comes before it. One space after each colon.
- The orchestrator (Lane 9) takes `Rex-Job` from what `rex job create` printed, and `Rex-Role` from the role table.
- Any skill of Lanes 9 and 10 that tells an agent to start a sub-agent says: copy `Rex-Job` from the tag block of your own brief (or use the ID that `rex job create` printed), and write the new sub-agent's own role as `Rex-Role`.
- A role that runs as the session does starts as agent type `general-purpose` with no model. For such a role the tag block is the ONLY source of the role. Without it the run shows as `general-purpose` with no role.
- The sub-agent that stage 1 of the builder starts to write a skill, before the plan is recorded, gets `Rex-Role: skill-writer`. That role is in no role table. It is recorded as text, and nothing fails.
- The builder runs `rex job create` before it starts its first sub-agent. A sub-agent that starts while its session has no open job is not stored at all, whatever its tags say (C5 section 3).
- An agent may read the tag block of its own brief, to copy `Rex-Job`, and as the sign that a job is already open (`orchestration-builder`). The skills must not tell agents to check, correct or send again the tags of a brief already sent. `rex` reads them by itself.

## What was tested and what was not

Tested on Claude Code 2.1.322, macOS: a recorded session whose briefs start with the block, for runs in the foreground, in the background, and at depth 2, plus one brief with no block. The brief arrives unchanged in `tool_input.prompt` of the `Agent` event, in both forms (`completed` and `async_launched`), and as the first user text of the sub-agent's transcript.

Settled at gate G1, not yet built: the parsing rules above. The two variants (a key given twice; the order reversed with an unknown key) are proved by Lane 3's tests, which write them into two recorded briefs. Not tested: briefs with `\r\n` line ends; the first user record of a worktree sub-agent (`isolation: "worktree"`), where Claude Code may put text before the brief. IF it does, THEN the transcript gives no tags for that run, and only the hook event does.

## Limits

- The role is only as good as the brief. An agent that leaves the block out gives a run with no role.
- `Rex-Job` is a cross-check, not a source of the tie.
