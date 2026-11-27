# WP0, assumption 3: a mod calls its own plugin's MCP server with `$.mcp.call`

Run on 2026-11-27 with Claude Code 2.1.332, node v24.4.0, and `--model haiku`. A live Claude Code session cannot run inside `cargo test`, so this proof stays here as a runner plus its quoted output.

## What is here

| File | Purpose |
| --- | --- |
| `probe-plugin/.claude-plugin/plugin.json` | Manifest. Lists one stdio server under the key `mcpServers` → `stub`. |
| `probe-plugin/server/stub.ts` | Hand-written MCP server (JSON-RPC over stdin and stdout, built-ins only). Its tool `echo` returns the nonce and appends each call to the log named by `WP0_MCP_LOG`. |
| `probe-plugin/hooks/register.ts` | Mod. Registers `mcp_probe`, which calls the stub with `$.mcp.call` and wraps the answer in `WP0-MCP-MARKER … WP0-MCP-MARKER-END`. Also registers `spawn_probe`, which starts a `super-rex:model-low-effort-low` agent with `$.agent.spawn` in another folder. |
| `run.ts` | Runner. Makes a new nonce and a new temp folder for each case, then runs one nested `claude -p` session there. |

How to run it, from the repo root: `node tests/fixtures/wp0-mcp/run.ts a b c`. This starts three nested sessions.

Why a marked answer can only have come through the mod:

- Each nested session allows only the probe tools, plus `Agent` for case b.
- Each session disallows every stub tool (`--disallowedTools mcp__plugin_wp0-mcp-probe_stub 'mcp__plugin_wp0-mcp-probe_stub__*'`).
- The init message offered the model no stub tool.
- The transcript shows no tool call other than the ones listed in each case below.
- The marker text appears only in the mod's source, not in any prompt.

## Type checks

```
$ tsc -p tests/fixtures/wp0-mcp/probe-plugin/tsconfig.json
exit 0
$ tsc -p tests/fixtures/wp0-mcp/tsconfig.json
exit 0
$ tsc -p plugin
exit 0
```

## Manifest key: `mcpServers`, PASSED (run)

```
$ claude plugin validate tests/fixtures/wp0-mcp/probe-plugin
  ❯ ./register.ts calls: $.agent.spawn (via serveSpawn), $.mcp.call (via callOnce), $.mcp.connect (via callStub), $.tool.register
Provides:
  ❯ modules: hooks/register.ts
  ❯ mcpServers: stub stdio node ["${CLAUDE_PLUGIN_ROOT}/server/stub.ts"]
✔ Validation passed
```

The only warning was `ICON_MISSING`, from the plugin directory checks. Those checks do not change the result.

In every run below, the init message listed `{"name":"plugin:wp0-mcp-probe:stub","status":"connected","source":"plugin"}`. The stub logged `initialize` from `{"name":"claude-code",…,"version":"2.1.332"}`.

## Case a: main session, PASSED (run)

```
command: (cd /var/folders/t5/jttpy31x0rq99fnq9rrj1jfr0000gn/T/wp0-mcp-a-IVLkmQ && WP0_MCP_LOG=…/stub-calls.log claude "-p" "Call the tool mcp__wp0-mcp-probe__mcp_probe once with {\"nonce\": \"eb64cbc7-6938-4954-b103-ac5f70f888f4\"}, then reply with its output exactly as it came back, and nothing else. Call no other tool." "--model" "haiku" "--plugin-dir" "…/tests/fixtures/wp0-mcp/probe-plugin" "--output-format" "stream-json" "--verbose" "--allowedTools" "mcp__wp0-mcp-probe__mcp_probe" "--disallowedTools" "mcp__plugin_wp0-mcp-probe_stub" "mcp__plugin_wp0-mcp-probe_stub__*")
nonce: eb64cbc7-6938-4954-b103-ac5f70f888f4
exit: status 0 signal null; cost usd 0.006141789999999999
stub server at init: plugin:wp0-mcp-probe:stub connected
stub tools offered to the model: none
permission denials: none
marker: WP0-MCP-MARKER caller=main server=plugin:wp0-mcp-probe:stub route=call isError=false answer=[stub-echo nonce=eb64cbc7-6938-4954-b103-ac5f70f888f4 caller=main pid=97998] WP0-MCP-MARKER-END
log: {"at":"2026-11-27T15:09:53.868Z","pid":97998,"event":"tools/call","tool":"echo","nonce":"eb64cbc7-6938-4954-b103-ac5f70f888f4","caller":"main"}
final answer: nonce=eb64cbc7-6938-4954-b103-ac5f70f888f4
verdict: PASSED
```

The transcript has one tool call: `mcp__wp0-mcp-probe__mcp_probe` from main. The marker is in that call's tool result. The model shortened the text of its final answer.

## Case b: a sub-agent's tool call, PASSED (run)

```
command: (cd /var/folders/t5/jttpy31x0rq99fnq9rrj1jfr0000gn/T/wp0-mcp-b-vERjqy && … claude "-p" "Use the Agent tool once, with subagent_type \"general-purpose\" and model \"haiku\", … nonce \"48ec98e4-0592-4c4b-a9ff-5f91fa90783b\" …" "--model" "haiku" "--plugin-dir" "…/probe-plugin" "--output-format" "stream-json" "--verbose" "--allowedTools" "mcp__wp0-mcp-probe__mcp_probe" "Agent" "--disallowedTools" "mcp__plugin_wp0-mcp-probe_stub" "mcp__plugin_wp0-mcp-probe_stub__*")
nonce: 48ec98e4-0592-4c4b-a9ff-5f91fa90783b
exit: status 0 signal null; cost usd 0.008270524999999999
stub server at init: plugin:wp0-mcp-probe:stub connected
stub tools offered to the model: none
permission denials: none
tool calls in the transcript: Agent in main, mcp__wp0-mcp-probe__mcp_probe in toolu_01BK4GFX1k1cZikMH8u7tN6M
marker: WP0-MCP-MARKER caller=a5878faf761b44d35 server=plugin:wp0-mcp-probe:stub route=call isError=false answer=[stub-echo nonce=48ec98e4-0592-4c4b-a9ff-5f91fa90783b caller=a5878faf761b44d35 pid=98511] WP0-MCP-MARKER-END
log: {"at":"2026-11-27T15:10:40.904Z","pid":98511,"event":"tools/call","tool":"echo","nonce":"48ec98e4-0592-4c4b-a9ff-5f91fa90783b","caller":"a5878faf761b44d35"}
final answer: WP0-MCP-MARKER caller=a5878faf761b44d35 server=plugin:wp0-mcp-probe:stub route=call isError=false answer=[stub-echo nonce=48ec98e4-0592-4c4b-a9ff-5f91fa90783b caller=a5878faf761b44d35 pid=98511] WP0-MCP-MARKER-END
verdict: PASSED (sub-agent a5878faf761b44d35)
```

## Case c: a builder started by a mod, PASSED (run, proxy)

```
command: (cd /var/folders/t5/jttpy31x0rq99fnq9rrj1jfr0000gn/T/wp0-mcp-c-Kvc7K7 && … claude "-p" "Call the tool mcp__wp0-mcp-probe__spawn_probe once with {\"nonce\": \"bf783526-12ed-4611-ad7d-41f52bd1a9b5\", \"cwd\": \"/var/folders/t5/jttpy31x0rq99fnq9rrj1jfr0000gn/T/wp0-mcp-builder-v5tqpj\"}. …" "--model" "haiku" "--plugin-dir" "…/probe-plugin" "--plugin-dir" "…/plugin" "--output-format" "stream-json" "--verbose" "--allowedTools" "mcp__wp0-mcp-probe__mcp_probe" "mcp__wp0-mcp-probe__spawn_probe" "--disallowedTools" "mcp__plugin_wp0-mcp-probe_stub" "mcp__plugin_wp0-mcp-probe_stub__*")
nonce: bf783526-12ed-4611-ad7d-41f52bd1a9b5
exit: status 0 signal null; cost usd 0.00685934
stub server at init: plugin:wp0-mcp-probe:stub connected
stub tools offered to the model: none
permission denials: none
tool calls in the transcript: mcp__wp0-mcp-probe__spawn_probe in main, mcp__wp0-mcp-probe__mcp_probe in toolu_plugin_5e0f87f73d2e447da713bf628a4ab19a
builder folder: /var/folders/t5/jttpy31x0rq99fnq9rrj1jfr0000gn/T/wp0-mcp-builder-v5tqpj
spawned agent: a1558edb08c89ebe4
marker: WP0-MCP-MARKER caller=a1558edb08c89ebe4 server=plugin:wp0-mcp-probe:stub route=call isError=false answer=[stub-echo nonce=bf783526-12ed-4611-ad7d-41f52bd1a9b5 caller=a1558edb08c89ebe4 pid=98587] WP0-MCP-MARKER-END
log: {"at":"2026-11-27T15:10:50.246Z","pid":98587,"event":"tools/call","tool":"echo","nonce":"bf783526-12ed-4611-ad7d-41f52bd1a9b5","caller":"a1558edb08c89ebe4"}
verdict: PASSED (spawned agent a1558edb08c89ebe4)
```

From the same transcript:

```
spawn_probe: started agent=a1558edb08c89ebe4 type=super-rex:model-low-effort-low model=claude-haiku-5-5 cwd=/var/folders/t5/jttpy31x0rq99fnq9rrj1jfr0000gn/T/wp0-mcp-builder-v5tqpj
"subtype":"task_started","task_id":"a1558edb08c89ebe4",…,"subagent_type":"super-rex:model-low-effort-low","is_backgrounded":true,"spawn_depth":1,"task_type":"local_agent"
"subtype":"task_notification","task_id":"a1558edb08c89ebe4",…,"status":"completed",…,"summary":"WP0-MCP-MARKER caller=a1558edb08c89ebe4 server=plugin:wp0-mcp-p…
"permissionMode":"default"
```

The main model's final text was written before the builder ended, so it holds no marker. The marker is in the builder's tool result and in the task's completion notice.

Case c is a proxy for a real builder. It keeps these parts of the real build path:

- The spawn is made by `$.agent.spawn` from inside a mod's `tool.call` hook of the main loop, as `rex_build` does.
- The agent type is the grid type `super-rex:model-low-effort-low`, from the repo's own `plugin/`. That plugin was loaded with `--plugin-dir`, because a nested session here does not load an installed super-rex.
- The `cwd` is a folder other than the session's.
- The session is headless, with allow rules for the probe tool and permission mode `default`.

It does not do these things:

- No master tree and no charter.
- No `rex_build` gate and no `rex_tree_open`.
- No job, no worktree and no builder brief.
- No `pendingSpawns` entry, no `linkBuilder` and no `permissionRefusal` check. Those run only for super-rex's own spawns.
- No `bypassPermissions`.
- The builder's working folder was not checked separately, because the builder had no tool that could print it.
