#!/bin/sh
# Stands in for `rex hook` while recording fixtures. It appends each hook event to
# .rex/events.jsonl in the session's project folder as one line:
#   {"v":1,"at_ms":<milliseconds since the epoch>,"event":<the event object>}
# The whole line goes out in one append-mode write, so lines from hooks that run at once do not mix.
python3 -c '
import json, os, sys, time

raw = sys.stdin.buffer.read()
if not raw.strip():
    sys.exit(0)
try:
    body = {"event": json.loads(raw)}
except ValueError:
    body = {"raw": raw.decode("utf-8", "replace")}
line = {"v": 1, "at_ms": int(time.time() * 1000), **body}
data = (json.dumps(line, separators=(",", ":"), ensure_ascii=False) + "\n").encode("utf-8")

log_folder = os.path.join(os.environ.get("CLAUDE_PROJECT_DIR", "."), ".rex")
if os.path.isdir(log_folder):
    fd = os.open(os.path.join(log_folder, "events.jsonl"), os.O_WRONLY | os.O_APPEND | os.O_CREAT, 0o644)
    os.write(fd, data)
    os.close(fd)
'
exit 0
