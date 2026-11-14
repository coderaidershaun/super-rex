#!/usr/bin/env bash
# Makes the eval's workspace a repo the mod acts in: a Git repo whose .rex/config.json turns the mod on with storage off.
set -euo pipefail

git init -q -b main
mkdir -p .rex
printf '%s\n' '{ "version": 1, "storage": "off", "depth": 4, "jobs_from_sdk": true, "min_claude_code": "2.1.323" }' >.rex/config.json
