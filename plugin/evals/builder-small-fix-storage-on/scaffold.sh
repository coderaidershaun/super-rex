#!/usr/bin/env bash
# Makes the eval's workspace a repo with one spelling fault, a bare remote as origin, and rex init with storage, so a builder can deliver to main.
set -euo pipefail

command -v rex >/dev/null 2>&1 || { echo "scaffold: rex is not on PATH; put the job binary (target/debug) first on PATH" >&2; exit 1; }
git init -q -b main
git config user.name "rex eval"
git config user.email "eval@example.invalid"
printf 'The parser will recieve each line.\n' >NOTES.md
git add NOTES.md
git commit -q -m "notes"
remote="$TMPDIR/rex-eval-remote.git"
git init -q --bare "$remote"
git remote add origin "$remote"
git push -q origin main
git remote set-head origin main
# With the plugin already enabled in the project settings, rex init installs nothing through the claude command.
mkdir -p .claude
printf '%s\n' '{ "enabledPlugins": { "super-rex@super-rex": true } }' >.claude/settings.json
rex init --with-storage
