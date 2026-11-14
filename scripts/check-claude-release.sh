#!/usr/bin/env bash
# Checks the plugin against the Claude Code on PATH: type check of the mod, strict validate, the mod's tests, and with --live one headless session on Sonnet.
# Run it from the repo root after each Claude Code release; --renew-types first copies in the declarations that release wrote.
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/check-claude-release.sh [--renew-types] [--live]

  --renew-types  Copy the declarations of the Claude Code on PATH into
                 typings/claude-code/index.d.ts before the checks. They are the
                 file the plugin-authoring skill lays out when a session of that
                 build loads it (type /plugin-authoring once in any session).
                 Commit the renewed file on its own.
  --live         Also run one headless session on Sonnet with this plugin and
                 the binary of this tree, capped at $0.50, in a scratch repo
                 below $CHECK_SCRATCH (default: $TMPDIR). It costs about $0.10.
EOF
}

is_live=false
is_renew=false
for arg in "$@"; do
  case "$arg" in
    --live) is_live=true ;;
    --renew-types) is_renew=true ;;
    -h | --help) usage; exit 0 ;;
    *)
      echo "check-claude-release: no option $arg; run with --help for the options" >&2
      exit 2
      ;;
  esac
done

fail() {
  echo "check-claude-release: $*" >&2
  exit 1
}

if [ ! -f plugin/.claude-plugin/plugin.json ] || [ ! -f typings/claude-code/index.d.ts ]; then
  fail "run it from the repo root, the folder that holds plugin/ and typings/; got $(pwd)"
fi
for tool in claude tsc python3; do
  command -v "$tool" >/dev/null 2>&1 || fail "$tool is not on PATH; install it, then run this again"
done

claude_version="$(claude --version | sed -n 's/^\([0-9][0-9.]*\).*/\1/p')"
[ -n "$claude_version" ] || fail "claude --version printed no version; got: $(claude --version)"

declared_version() {
  sed -n '1s/^\/\/ Written by Claude Code \([0-9][0-9.]*\)\.$/\1/p' "$1"
}

if [ "$is_renew" = true ]; then
  # A session that loads a bundled skill writes its files below <temp>/claude-<uid>/bundled-skills/<version>/, where <temp> is /tmp, or $TMPDIR for a session started inside a sandbox.
  bundle_roots=("/tmp/claude-$(id -u)/bundled-skills/$claude_version")
  if [ -n "${TMPDIR:-}" ]; then
    bundle_roots+=("${TMPDIR%/}/claude-$(id -u)/bundled-skills/$claude_version")
  fi
  renewed=""
  for root in "${bundle_roots[@]}"; do
    for candidate in "$root"/*/plugin-authoring/types/claude-code.d.ts; do
      [ -f "$candidate" ] && renewed="$candidate"
    done
  done
  [ -n "$renewed" ] || fail "no declarations of Claude Code $claude_version below ${bundle_roots[*]}; type /plugin-authoring once in a session of that build, then run this again"
  [ "$(declared_version "$renewed")" = "$claude_version" ] || fail "$renewed does not open with '// Written by Claude Code $claude_version.'; got: $(head -n 1 "$renewed")"
  cp "$renewed" typings/claude-code/index.d.ts
  echo "check-claude-release: renewed typings/claude-code/index.d.ts from $renewed"
fi

types_version="$(declared_version typings/claude-code/index.d.ts)"
[ -n "$types_version" ] || fail "typings/claude-code/index.d.ts does not open with '// Written by Claude Code <version>.'; got: $(head -n 1 typings/claude-code/index.d.ts)"
echo "check-claude-release: claude $claude_version, declarations of $types_version"
if [ "$types_version" != "$claude_version" ]; then
  echo "check-claude-release: note: the mod is typed against $types_version, not the $claude_version on PATH; --renew-types types it against $claude_version"
fi

logs="$(mktemp -d "${TMPDIR:-/tmp}/check-claude-release.XXXXXX")"

# A tsc that finds nothing prints nothing, so any output at all is a failure even when its exit code is 0.
for project in plugin plugin/evals; do
  if ! tsc -p "$project" >"$logs/tsc.txt" 2>&1 || [ -s "$logs/tsc.txt" ]; then
    cat "$logs/tsc.txt" >&2
    fail "tsc -p $project found type errors (above), against the declarations of Claude Code $types_version"
  fi
  echo "check-claude-release: tsc -p $project: no errors"
done

if ! python3 -I scripts/gen-shared-fixtures.py --check >"$logs/fixtures.txt" 2>&1; then
  cat "$logs/fixtures.txt" >&2
  fail "plugin/hooks/book/shared.ts differs from fixtures/shared; run python3 scripts/gen-shared-fixtures.py and commit the copy"
fi
echo "check-claude-release: plugin/hooks/book/shared.ts equals fixtures/shared"

if ! claude plugin validate --strict plugin >"$logs/validate.txt" 2>&1; then
  cat "$logs/validate.txt" >&2
  fail "claude plugin validate --strict plugin refused the plugin (report above)"
fi
echo "check-claude-release: claude plugin validate --strict plugin: $(tail -n 1 "$logs/validate.txt")"

if ! claude plugin test plugin >"$logs/test.txt" 2>&1; then
  grep -E '^\(fail\)|^ *[0-9]+ (pass|fail)$' "$logs/test.txt" >&2 || cat "$logs/test.txt" >&2
  fail "claude plugin test plugin failed; full output in $logs/test.txt"
fi
echo "check-claude-release: claude plugin test plugin: $(grep -E '^Ran [0-9]+ tests' "$logs/test.txt" || tail -n 1 "$logs/test.txt")"

if [ "$is_live" = true ]; then
  repo_root="$(pwd)"
  command -v cargo >/dev/null 2>&1 || fail "cargo is not on PATH; --live runs the binary of this tree, so install Rust, then run this again"
  cargo build --locked --quiet || fail "cargo build --locked failed (above); --live needs the binary of this tree"
  scratch="$(mktemp -d "${CHECK_SCRATCH:-${TMPDIR:-/tmp}}/rex-release-smoke.XXXXXX")"
  # The scratch repo holds only what the mod reads at session start; the deny rule keeps sub-agents, and so other models, out of the run.
  git -C "$scratch" init -q
  mkdir -p "$scratch/.rex" "$scratch/.claude" "$scratch/store"
  printf '%s\n' '{ "version": 1, "storage": "off", "depth": 4, "jobs_from_sdk": true, "min_claude_code": "2.1.323" }' >"$scratch/.rex/config.json"
  printf '%s\n' '{ "env": { "CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH": "4" }, "permissions": { "deny": ["Agent"] } }' >"$scratch/.claude/settings.local.json"
  smoke_prompt='Call the tool mcp__super-rex__rex_status with no arguments. Then reply with the first line of its result, exactly as the tool gave it, and nothing else.'
  # Each step of the run that would use Haiku goes to Sonnet, so the run uses no other model.
  if ! (cd "$scratch" && PATH="$repo_root/target/debug:$PATH" REX_STORE="$scratch/store" ANTHROPIC_DEFAULT_HAIKU_MODEL=claude-sonnet-5-5 \
    claude -p "$smoke_prompt" --model sonnet --effort low --plugin-dir "$repo_root/plugin" \
      --setting-sources project,local --no-session-persistence --disallowedTools Agent \
      --max-budget-usd 0.50 --output-format stream-json --verbose --debug-file "$logs/smoke-debug.log" \
      </dev/null >"$logs/smoke.jsonl" 2>"$logs/smoke.stderr"); then
    cat "$logs/smoke.stderr" >&2
    fail "claude -p (the live smoke) exited non-zero; stream in $logs/smoke.jsonl, debug log in $logs/smoke-debug.log"
  fi
  grep -q 'hooks module super-rex@inline loaded' "$logs/smoke-debug.log" \
    || fail "the live smoke's debug log has no line 'hooks module super-rex@inline loaded': the mod did not load; see $logs/smoke-debug.log"
  if grep -E 'super-rex' "$logs/smoke-debug.log" | grep -E 'hook failed|bypassed by|unloaded' >"$logs/smoke-hook-trouble.txt"; then
    cat "$logs/smoke-hook-trouble.txt" >&2
    fail "the live smoke's debug log says a super-rex hook failed or was skipped (lines above)"
  fi
  python3 -I - "$logs/smoke.jsonl" <<'PY' || fail "the live smoke's answer is not the mod's (reason above); stream in $logs/smoke.jsonl"
import json
import sys

results = []
tool_texts = []
with open(sys.argv[1], encoding="utf-8") as stream:
    for line in stream:
        event = json.loads(line)
        if event.get("type") == "result":
            results.append(event)
        content = event.get("message", {}).get("content", []) if event.get("type") == "user" else []
        for block in content if isinstance(content, list) else []:
            if isinstance(block, dict) and block.get("type") == "tool_result":
                inner = block.get("content")
                parts = inner if isinstance(inner, list) else [{"text": inner}]
                tool_texts.extend(str(part.get("text", "")) for part in parts if isinstance(part, dict))
if not results:
    sys.exit("check-claude-release: the live smoke wrote no result line")
last = results[-1]
cost = max(float(result.get("total_cost_usd", 0)) for result in results)
models = sorted({model for result in results for model in result.get("modelUsage", {})})
print(f"check-claude-release: live smoke cost ${cost:.4f} on {', '.join(models) or 'no model'}; it answered {str(last.get('result', '')).strip()[:80]!r}")
if last.get("is_error"):
    sys.exit(f"check-claude-release: the live smoke ended in an error: {last.get('subtype')}")
if any("sonnet" not in model for model in models):
    sys.exit(f"check-claude-release: the live smoke used a model other than Sonnet, got {models}")
if not any(text.startswith("job=") for text in tool_texts):
    sys.exit(f"check-claude-release: no rex_status result in the stream starts with job=, got {tool_texts[:3]}")
if not str(last.get("result", "")).strip().startswith("job="):
    sys.exit(f"check-claude-release: the session's answer does not start with job=, got {str(last.get('result', ''))[:200]!r}")
PY
  rm -rf -- "$scratch"
fi

echo "check-claude-release: ok for Claude Code $claude_version; logs in $logs"
