#!/usr/bin/env python3
"""Turns the raw copies of one recorded session into fixtures that hold nothing private.

Usage: sanitize.py <raw folder> <fixture folder> <repo path> <sibling path>

The raw folder holds transcripts/ (the main transcript and its subagents/ folder), copied
from the machine, and when the session has them, events.jsonl and result.jsonl (the final
result records of the stream).
The owner's values (home folder, user name, Git user, host name) are read on this machine
at run time, so this file itself holds none of them.
"""
import getpass
import json
import os
import re
import socket
import subprocess
import sys
from pathlib import Path

MAX_FILE_BYTES = 1024 * 1024

# Records not listed here are dropped. They hold the system prompt, tool definitions,
# full request bodies, attachments (skills, agents, organisation) or the account IDs.
KEEP_RECORD_FIELDS = {
    "user": {
        "type", "uuid", "parentUuid", "timestamp", "sessionId", "agentId", "isSidechain",
        "isMeta", "promptId", "sourceToolAssistantUUID", "sourceToolUseID", "message", "toolUseResult", "cwd",
        "version", "gitBranch", "entrypoint", "userType", "permissionMode", "promptSource",
        "turnOrigin", "turnPosition",
    },
    "assistant": {
        "type", "uuid", "parentUuid", "timestamp", "sessionId", "agentId", "isSidechain",
        "requestId", "message", "effort", "perTurnEffort", "requestedModel", "attributionAgent", "attributionSkill",
        "apiBlockIndex", "cwd", "version", "gitBranch", "entrypoint", "userType",
    },
    "api-request": {"type", "id", "timestamp", "sessionId", "agentId", "querySource", "version", "params"},
    "cost-state": None,
    "tools-used": None,
}
KEEP_MESSAGE_FIELDS = {"id", "model", "role", "type", "content", "usage", "stop_reason", "stop_sequence"}
KEEP_REQUEST_PARAMS = {"model", "max_tokens", "thinking", "output_config"}
KEEP_RESULT_FIELDS = {
    "type", "subtype", "is_error", "duration_ms", "duration_api_ms", "num_turns", "result",
    "result_index", "stop_reason", "terminal_reason", "session_id", "total_cost_usd", "usage",
    "modelUsage", "model_usage_since_last_result", "subagent_stats", "permission_denials", "uuid",
}

dropped = {}


def note_drop(what):
    dropped[what] = dropped.get(what, 0) + 1


def keep_only(obj, allowed, where):
    for key in sorted(set(obj) - allowed):
        note_drop(f"{where}.{key}")
    return {k: v for k, v in obj.items() if k in allowed}


def clean_transcript_record(record):
    kind = record.get("type")
    if kind not in KEEP_RECORD_FIELDS:
        note_drop(f"record {kind}")
        return None
    allowed = KEEP_RECORD_FIELDS[kind]
    if allowed is None:
        return record
    record = keep_only(record, allowed, kind)
    message = record.get("message")
    if isinstance(message, dict):
        message = keep_only(message, KEEP_MESSAGE_FIELDS, f"{kind}.message")
        if isinstance(message.get("content"), list):
            for block in message["content"]:
                if isinstance(block, dict) and block.pop("signature", None) is not None:
                    note_drop(f"{kind}.message.content[].signature")
        record["message"] = message
    if kind == "api-request" and isinstance(record.get("params"), dict):
        record["params"] = keep_only(record["params"], KEEP_REQUEST_PARAMS, "api-request.params")
    return record


def project_name(path):
    return re.sub(r"[^A-Za-z0-9]", "-", path)


def git_config(key):
    found = subprocess.run(["git", "config", key], capture_output=True, text=True)
    return found.stdout.strip()


class Replacer:
    """Swaps private text for fixture text, and each message or request ID for a placeholder.

    One placeholder stands for one ID in every file, so equal IDs stay equal and
    different IDs stay different.
    """

    def __init__(self, repo, sibling):
        home = os.path.expanduser("~")
        # Longest first, so a path is replaced before the home folder inside it.
        self.pairs = [
            (sibling, "/home/fixture/work/sibling"),
            (repo, "/home/fixture/work/repo"),
            (project_name(sibling), "-home-fixture-work-sibling"),
            (project_name(repo), "-home-fixture-work-repo"),
            (home, "/home/fixture"),
            (project_name(home), "-home-fixture"),
        ]
        self.words = [git_config("user.email"), git_config("user.name"),
                      socket.gethostname(), socket.gethostname().split(".")[0], getpass.getuser()]
        self.ids = {}

    def placeholder(self, match):
        found = match.group(0)
        prefix = found.split("_")[0]
        if found not in self.ids:
            count = sum(1 for known in self.ids if known.startswith(prefix + "_")) + 1
            self.ids[found] = f"{prefix}_fixture{count:04d}"
        return self.ids[found]

    def apply(self, text):
        for old, new in self.pairs:
            text = text.replace(old, new)
        text = re.sub(r"/(?:private/)?tmp/claude-\d+/", "/tmp/claude-fixture/", text)
        for word in self.words:
            if word:
                text = re.sub(rf"(?<![A-Za-z0-9]){re.escape(word)}(?![A-Za-z0-9])", "fixture", text)
        return re.sub(r"\b(?:msg|req)_[A-Za-z0-9]{16,}\b", self.placeholder, text)


def compact(obj):
    return json.dumps(obj, separators=(",", ":"), ensure_ascii=False)


def write_checked(path, text):
    if path.suffix == ".jsonl":
        for line in text.splitlines():
            json.loads(line)
    else:
        json.loads(text)
    size = len(text.encode("utf-8"))
    if size >= MAX_FILE_BYTES:
        sys.exit(f"{path} is {size} bytes, over the limit of {MAX_FILE_BYTES}")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def sanitize_jsonl(source, target, replacer, clean):
    lines = []
    for raw in source.read_text(encoding="utf-8").splitlines():
        record = clean(json.loads(raw))
        if record is not None:
            lines.append(replacer.apply(compact(record)))
    write_checked(target, "".join(line + "\n" for line in lines))


def main():
    raw, out, repo, sibling = (Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3], sys.argv[4])
    replacer = Replacer(repo, sibling)

    if (raw / "events.jsonl").exists():
        sanitize_jsonl(raw / "events.jsonl", out / "events.jsonl", replacer, lambda line: line)

    transcripts = sorted((raw / "transcripts").rglob("*.jsonl"))
    for source in transcripts:
        target = out / source.relative_to(raw)
        sanitize_jsonl(source, target, replacer, clean_transcript_record)

    for source in sorted((raw / "transcripts").rglob("*.meta.json")):
        text = replacer.apply(compact(json.loads(source.read_text(encoding="utf-8"))))
        write_checked(out / source.relative_to(raw), text + "\n")

    if (raw / "result.jsonl").exists():
        results = []
        for raw_line in (raw / "result.jsonl").read_text(encoding="utf-8").splitlines():
            results.append(keep_only(json.loads(raw_line), KEEP_RESULT_FIELDS, "result"))
        text = replacer.apply(json.dumps(results, indent=2, ensure_ascii=False))
        write_checked(out / "result.json", text + "\n")

    for what, count in sorted(dropped.items()):
        print(f"dropped {count:4d}  {what}")
    print(f"replaced {len(replacer.ids)} message and request IDs")


main()
