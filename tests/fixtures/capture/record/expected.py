#!/usr/bin/env python3
"""Works out the values that tests compare against, from the sanitized fixture files only.

Usage: expected.py <fixture tree folder>             (writes expected.json in that folder)
       expected.py skills <fixture skills folder>    (the skills opened by each agent)

It shares no code with the crate on purpose, so a bug in the crate's reader cannot hide
in both places at once.
"""
import json
import re
import sys
from pathlib import Path

TAG_LINE = re.compile(r"^Rex-[A-Za-z][A-Za-z-]*: \S.*$")


def read_jsonl(path):
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line]


def tag_block(brief):
    tags = {}
    for line in brief.split("\n"):
        if not TAG_LINE.match(line):
            break
        key, value = line.split(": ", 1)
        tags.setdefault(key, value.strip())
    return tags


def tokens_by_model(records):
    """Adds up usage with each message ID counted once, because one request is split over
    several records that all repeat the same usage."""
    totals = {}
    seen = set()
    for record in records:
        message = record.get("message") or {}
        if record.get("type") != "assistant" or message.get("id") in seen:
            continue
        seen.add(message["id"])
        usage = message["usage"]
        model = totals.setdefault(message["model"], {"input": 0, "output": 0, "cache_write": 0, "cache_read": 0})
        model["input"] += usage["input_tokens"]
        model["output"] += usage["output_tokens"]
        model["cache_write"] += usage["cache_creation_input_tokens"]
        model["cache_read"] += usage["cache_read_input_tokens"]
    return totals, len(seen)


def piece_count(text):
    return len(text.split("\n"))


def transcript_counts(records):
    """Tool calls and lines changed, counted the way Claude Code counts them in toolStats:
    pieces of text split on newlines, not a line diff."""
    seen = set()
    lines_added = 0
    lines_removed = 0
    for record in records:
        if record.get("type") != "assistant":
            continue
        for block in record["message"]["content"]:
            if block.get("type") != "tool_use" or block["id"] in seen:
                continue
            seen.add(block["id"])
            tool_input = block["input"]
            if block["name"] == "Edit":
                lines_added += piece_count(tool_input["new_string"])
                lines_removed += piece_count(tool_input["old_string"])
            elif block["name"] == "Write":
                lines_added += piece_count(tool_input["content"])
    return {"tool_uses": len(seen), "lines_added": lines_added, "lines_removed": lines_removed}


def check_against_event(agent_id, counts, call):
    tool_stats = call["tool_response"].get("toolStats") or {}
    from_event = {
        "tool_uses": call["tool_response"]["totalToolUseCount"],
        "lines_added": tool_stats.get("linesAdded", 0),
        "lines_removed": tool_stats.get("linesRemoved", 0),
    }
    if counts != from_event:
        sys.exit(f"agent {agent_id}: counts from its transcript {counts} differ from its event {from_event}")


def add_into(total, part):
    for model, counts in part.items():
        into = total.setdefault(model, {"input": 0, "output": 0, "cache_write": 0, "cache_read": 0})
        for key, value in counts.items():
            into[key] += value


def write_tree_expected(tree):
    events = [line["event"] for line in read_jsonl(tree / "events.jsonl")]
    results = json.loads((tree / "result.json").read_text(encoding="utf-8"))
    session_id = results[-1]["session_id"]
    session_folder = tree / "transcripts" / session_id

    starts = {e["agent_id"]: e for e in events if e["hook_event_name"] == "SubagentStart"}
    stops = {e["agent_id"]: e for e in events if e["hook_event_name"] == "SubagentStop"}
    calls = {e["tool_response"]["agentId"]: e for e in events
             if e["hook_event_name"] == "PostToolUse" and e["tool_name"] == "Agent"}

    def depth(agent_id):
        parent = calls[agent_id].get("agent_id")
        return 1 if parent is None else depth(parent) + 1

    all_tokens = {}
    agents = []
    for agent_id in starts:
        call = calls[agent_id]
        meta = json.loads((session_folder / "subagents" / f"agent-{agent_id}.meta.json").read_text(encoding="utf-8"))
        records = read_jsonl(session_folder / "subagents" / f"agent-{agent_id}.jsonl")
        tokens, requests = tokens_by_model(records)
        add_into(all_tokens, tokens)
        models = sorted(tokens)
        background = call["tool_response"]["status"] == "async_launched"
        entry = {
            "agent_id": agent_id,
            "agent_type": starts[agent_id]["agent_type"],
            "description": call["tool_input"]["description"],
            "parent": call.get("agent_id"),
            "depth": depth(agent_id),
            "meta_spawn_depth": meta["spawnDepth"],
            "background": background,
            "role": tag_block(call["tool_input"]["prompt"]).get("Rex-Role"),
            "tag_job": tag_block(call["tool_input"]["prompt"]).get("Rex-Job"),
            "model": models[0] if len(models) == 1 else models,
            "resolved_model": call["tool_response"]["resolvedModel"],
            "effort": stops[agent_id]["effort"]["level"],
            "requests": requests,
            "tokens": tokens,
            "transcript_counts": transcript_counts(records),
        }
        if not background:
            check_against_event(agent_id, entry["transcript_counts"], call)
            # A run that used no tool has no toolStats key at all.
            entry["toolStats"] = call["tool_response"].get("toolStats")
            entry["totalToolUseCount"] = call["tool_response"]["totalToolUseCount"]
        agents.append(entry)

    main_tokens, main_requests = tokens_by_model(read_jsonl(tree / "transcripts" / f"{session_id}.jsonl"))
    add_into(all_tokens, main_tokens)

    model_usage = results[-1]["modelUsage"]
    names = {"input": "inputTokens", "output": "outputTokens",
             "cache_write": "cacheCreationInputTokens", "cache_read": "cacheReadInputTokens"}
    reported = {model: {ours: usage[theirs] for ours, theirs in names.items()} for model, usage in model_usage.items()}
    sum_equals = {model: all_tokens.get(model) == reported.get(model) for model in sorted(set(all_tokens) | set(reported))}

    expected = {
        "session_id": session_id,
        "agents": agents,
        "main_session": {"requests": main_requests, "tokens": main_tokens},
        "sum_over_transcripts": all_tokens,
        "modelUsage": reported,
        "sum_equals_model_usage": sum_equals,
    }
    (tree / "expected.json").write_text(json.dumps(expected, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(sum_equals))


def tool_calls(records):
    """Every tool call of one transcript, in order, marked failed when its result is an error."""
    failed_ids = set()
    for record in records:
        content = (record.get("message") or {}).get("content")
        if record.get("type") == "user" and isinstance(content, list):
            for block in content:
                if block.get("type") == "tool_result" and block.get("is_error") is True:
                    failed_ids.add(block["tool_use_id"])
    calls = []
    for record in records:
        if record.get("type") != "assistant":
            continue
        for block in record["message"]["content"]:
            if block.get("type") != "tool_use":
                continue
            call = {"name": block["name"], "failed": block["id"] in failed_ids}
            if "skill" in block["input"]:
                call["skill"] = block["input"]["skill"]
            if "file_path" in block["input"]:
                call["file_path"] = block["input"]["file_path"]
            calls.append(call)
    return calls


def skills_opened(calls):
    """Names from Skill calls, plus the folder name of each SKILL.md read that did not fail."""
    names = []
    for call in calls:
        if call["name"] == "Skill":
            name = call.get("skill")
        elif call["name"] == "Read" and not call["failed"] and call.get("file_path", "").endswith("/SKILL.md"):
            name = Path(call["file_path"]).parent.name
        else:
            continue
        if name not in names:
            names.append(name)
    return names


def write_skills_expected(folder):
    main_transcript = next((folder / "transcripts").glob("*.jsonl"))
    session_id = main_transcript.stem
    main_calls = tool_calls(read_jsonl(main_transcript))
    agents = [{"agent_id": None, "agent_type": None, "description": "main session",
               "tool_calls": main_calls, "skills_opened": skills_opened(main_calls)}]
    for transcript in sorted((folder / "transcripts" / session_id / "subagents").glob("agent-*.jsonl")):
        meta = json.loads(transcript.with_suffix(".meta.json").read_text(encoding="utf-8"))
        calls = tool_calls(read_jsonl(transcript))
        agents.append({"agent_id": transcript.stem.removeprefix("agent-"), "agent_type": meta["agentType"],
                       "description": meta["description"], "tool_calls": calls,
                       "skills_opened": skills_opened(calls)})
    expected = {"session_id": session_id, "agents": agents}
    (folder / "expected.json").write_text(json.dumps(expected, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({a["description"]: a["skills_opened"] for a in agents}))


if len(sys.argv) == 3 and sys.argv[1] == "skills":
    write_skills_expected(Path(sys.argv[2]))
else:
    write_tree_expected(Path(sys.argv[1]))
