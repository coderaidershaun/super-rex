# WP5, live: Claude Code starts the plugin's library server, and a mod's `$.mcp.call` of `head` gets its answer

Run on 2026-11-27 on branch `rex/0h0f8vnww4` at commit `d7ce24d`, with Claude Code 2.1.332, node v24.4.0, tsc 5.7.3 and `--model haiku`. The `rex` binary is `target/debug/rex`, built by `cargo build` at that commit (`rex 0.14.0`). A live Claude Code session cannot run inside `cargo test` or `claude plugin test`, so this proof stays here as a runner plus its quoted output.

Nested sessions used: 3 (two proof runs and one control run).

## Verdicts

1. `PASSED`: the init message of the nested session lists `{"name":"plugin:super-rex:library","status":"connected","source":"plugin"}`, the server of the `super-rex` plugin's `library` key, as connected. Same in both proof runs.
2. `PASSED`: the probe mod's `$.mcp.call("plugin:super-rex:library", "head", {})` returned exactly the five fields `{"protocol":"skills/1","library":"super-rex","version":"sha256:ed1fae5f8d82e14527263fd17a0ca9f0a1767c93ced20c6e213b4f2884aa4a56","skills":25,"vocabulary":"1"}`, inside a marker span that holds the run's marker `a0c9a79f-d839-4486-a1c2-1da526632d23` (run 1) and `478dfe09-35a1-4349-8f48-9543ef5a4e10` (run 2). The direct call worked (`route=call`); the `$.mcp.connect` fallback was not needed.
3. `PASSED`: the connected server is this checkout's. The live `version` equals the version that this build gives for `<worktree>/plugin/library` when asked directly. The only `super-rex` plugin at init has the path `<worktree>/plugin`. The one server process that the session started runs `<worktree>/target/debug/rex` with `--library <worktree>/plugin/library` and carries the run's marker in its environment.
4. `NOT EXERCISED`: "the mod's own call is not refused inside a job". The nested session has no job (its folder has no `.rex/config.json`, so the super-rex mod is inert), and the guard on the raw tools has nothing to refuse. The in-job case is proven by the unit test `the guard sits on tool.call only: a $.mcp.call to the library server passes while a job is open, made inside a tool call of an agent of that job` in `plugin/hooks/library-guard.test.ts`.

## What is here

| File | Purpose |
| --- | --- |
| `probe-plugin/.claude-plugin/plugin.json` | Manifest of the throwaway plugin `wp5-live-probe`. It declares no MCP server of its own. |
| `probe-plugin/hooks/register.ts` | Mod. Registers one tool, `head_probe`, that calls `$.mcp.call("plugin:super-rex:library", "head", {})` and wraps the raw answer in `WP5-LIVE-MARKER marker=<marker> … WP5-LIVE-MARKER-END`. If the direct call throws, it tries `$.mcp.connect` first, as WP0 did, and names the route it took. |
| `run.ts` | Runner. Makes a new marker and a new temp folder, asks this build for `head` directly, then runs one nested `claude -p` session with this repo's `plugin/` and the probe plugin, and watches the process table while the session runs. |
| `node-builtins.d.ts` | The node built-ins that the runner uses, typed by hand, so the type check needs no new dependency. |
| `tsconfig.json` | One type check for the runner and the probe mod. |

How to run it, from the repo root: `node tests/fixtures/wp5-live/run.ts` for the proof, or `node tests/fixtures/wp5-live/run.ts control` for the control run. Each starts one nested session. Run `cargo build` first.

How the run puts the marker at each layer:

- The runner makes a new UUID for each run and puts it in the prompt and in the session's environment (`WP5_LIVE_MARKER`).
- The model passes it to `head_probe` (the tool input is quoted below).
- The probe mod puts it in the marker span around the live answer (the tool result is quoted below).
- The server process that Claude Code started has it in its environment, which ties that process to this run.

Why a marked answer can only have come through the mod:

- The session allows only `mcp__wp5-live-probe__head_probe`.
- It disallows every raw library tool (`--disallowedTools mcp__plugin_super-rex_library 'mcp__plugin_super-rex_library__*'`), and the init message offered the model no raw library tool.
- The transcript has one tool call, `head_probe` from main.
- The marker text appears only in the probe mod's source, not in any prompt.

## Type check

```
$ tsc -p tests/fixtures/wp5-live/tsconfig.json
exit 0
```

## Static check of the probe plugin

```
$ claude plugin validate tests/fixtures/wp5-live/probe-plugin
  ❯ ./register.ts hooks: session.start{cwd=/"(?:)"/}, tool.call{tool=?}
  ❯ ./register.ts gating hook with .catch: tool.call{tool=?}
  ❯ ./register.ts calls: $.mcp.call (via callHead), $.mcp.connect (via headOfLibrary), $.tool.register
Provides:
  ❯ modules: hooks/register.ts
✔ Validation passed
exit 0
```

The only warning was `ICON_MISSING`, from the plugin directory checks, which do not change the result.

## Run 1, proof

```
command: (cd /var/folders/t5/jttpy31x0rq99fnq9rrj1jfr0000gn/T/wp5-live-HL8viK && PATH=/Users/shaun/Code/DEVELOPMENT/super-rex.worktrees/0h0f8vnww4/target/debug:$PATH WP5_LIVE_MARKER=a0c9a79f-d839-4486-a1c2-1da526632d23 claude "-p" "Call the tool mcp__wp5-live-probe__head_probe once with {\"marker\": \"a0c9a79f-d839-4486-a1c2-1da526632d23\"}, then reply with its output exactly as it came back, and nothing else. Call no other tool." "--model" "haiku" "--max-turns" "3" "--plugin-dir" "/Users/shaun/Code/DEVELOPMENT/super-rex.worktrees/0h0f8vnww4/tests/fixtures/wp5-live/probe-plugin" "--plugin-dir" "/Users/shaun/Code/DEVELOPMENT/super-rex.worktrees/0h0f8vnww4/plugin" "--output-format" "stream-json" "--verbose" "--debug-file" "/var/folders/t5/jttpy31x0rq99fnq9rrj1jfr0000gn/T/wp5-live-HL8viK/debug.log" "--allowedTools" "mcp__wp5-live-probe__head_probe" "--disallowedTools" "mcp__plugin_super-rex_library" "mcp__plugin_super-rex_library__*")
marker: a0c9a79f-d839-4486-a1c2-1da526632d23
rex on the session PATH: /Users/shaun/Code/DEVELOPMENT/super-rex.worktrees/0h0f8vnww4/target/debug/rex
head, direct: {"protocol":"skills/1","library":"super-rex","version":"sha256:ed1fae5f8d82e14527263fd17a0ca9f0a1767c93ced20c6e213b4f2884aa4a56","skills":25,"vocabulary":"1"}
exit: status 0 signal null; cost usd 0.006919980000000001; unparsed transcript lines 0
raw library tools offered to the model: none
permission denials: none
final answer: WP5-LIVE-MARKER marker=a0c9a79f-d839-4486-a1c2-1da526632d23 caller=main server=plugin:super-rex:library route=call isError=false answer=[{"protocol":"skills/1","library":"super-rex","version":"sha256:ed1fae5f8d82e14527263fd17a0ca9f0a1767c93ced20c6e213b4f2884aa4a56","skills":25,"vocabulary":"1"}] WP5-LIVE-MARKER-END
debug: 2026-11-27T17:51:06.526Z [DEBUG] MCP server "plugin:super-rex:library": Starting connection with timeout of 30000ms
debug: 2026-11-27T17:51:06.542Z [DEBUG] MCP server "plugin:super-rex:library": Successfully connected (transport: stdio) in 26ms
debug: 2026-11-27T17:51:06.542Z [DEBUG] MCP server "plugin:super-rex:library": Connection established with capabilities: {"hasTools":true,"hasPrompts":false,"hasResources":false,"hasResourceSubscribe":false,"serverVersion":{"name":"super-rex-skills","version":"0.14.0"},"protocolEra":"legacy","negotiatedProtocolVersion":"2025-11-25"}
debug: 2026-11-27T17:51:09.454Z [DEBUG] MCP server "plugin:super-rex:library": Calling MCP tool: head
debug: 2026-11-27T17:51:09.469Z [DEBUG] MCP server "plugin:super-rex:library": Tool 'head' completed successfully in 15ms
debug: 2026-11-27T17:51:10.812Z [DEBUG] MCP server "plugin:super-rex:library": MCP server process exited cleanly

(1) library server connected at init: PASSED
    init mcp_servers entry: {"name":"plugin:super-rex:library","status":"connected","source":"plugin"}
(2) the mod's $.mcp.call of head: PASSED
    tool calls in the transcript: mcp__wp5-live-probe__head_probe in main
    probe tool input: {"marker":"a0c9a79f-d839-4486-a1c2-1da526632d23"}
    probe tool result: WP5-LIVE-MARKER marker=a0c9a79f-d839-4486-a1c2-1da526632d23 caller=main server=plugin:super-rex:library route=call isError=false answer=[{"protocol":"skills/1","library":"super-rex","version":"sha256:ed1fae5f8d82e14527263fd17a0ca9f0a1767c93ced20c6e213b4f2884aa4a56","skills":25,"vocabulary":"1"}] WP5-LIVE-MARKER-END
(3) the connected server is this checkout's: PASSED
    version, live: sha256:ed1fae5f8d82e14527263fd17a0ca9f0a1767c93ced20c6e213b4f2884aa4a56
    version, direct from /Users/shaun/Code/DEVELOPMENT/super-rex.worktrees/0h0f8vnww4/target/debug/rex for /Users/shaun/Code/DEVELOPMENT/super-rex.worktrees/0h0f8vnww4/plugin/library: sha256:ed1fae5f8d82e14527263fd17a0ca9f0a1767c93ced20c6e213b4f2884aa4a56
    super-rex plugins at init: [{"name":"super-rex","path":"/Users/shaun/Code/DEVELOPMENT/super-rex.worktrees/0h0f8vnww4/plugin","source":"super-rex@inline","version":"0.14.0"}]
    claude pid: 40407
    server of the session: pid 40461 ppid 40407 binary /Users/shaun/Code/DEVELOPMENT/super-rex.worktrees/0h0f8vnww4/target/debug/rex marker in its environment true command rex skills serve --name super-rex --library /Users/shaun/Code/DEVELOPMENT/super-rex.worktrees/0h0f8vnww4/plugin/library
    rex skills serve processes seen that the session did not start: 0
(4) the mod's own call is not refused inside a job: NOT EXERCISED: the nested session has no job, so the guard on the raw tools has nothing to refuse; the in-job case is proven by plugin/hooks/library-guard.test.ts
```

The debug lines are a subset: the runner prints every line that names `plugin:super-rex:library`, and four lines about the connection cache and the shutdown are left out here. The init message listed eight `claude.ai` servers too; they are the person's account connectors and play no part.

## Run 2, proof, new marker

Same command with the marker `478dfe09-35a1-4349-8f48-9543ef5a4e10`, folder `wp5-live-proof-DGefD9`. Exit 0, cost usd 0.00421635. The four verdicts are the same as in run 1:

```
(1) library server connected at init: PASSED
    init mcp_servers entry: {"name":"plugin:super-rex:library","status":"connected","source":"plugin"}
(2) the mod's $.mcp.call of head: PASSED
    probe tool input: {"marker":"478dfe09-35a1-4349-8f48-9543ef5a4e10"}
    probe tool result: WP5-LIVE-MARKER marker=478dfe09-35a1-4349-8f48-9543ef5a4e10 caller=main server=plugin:super-rex:library route=call isError=false answer=[{"protocol":"skills/1","library":"super-rex","version":"sha256:ed1fae5f8d82e14527263fd17a0ca9f0a1767c93ced20c6e213b4f2884aa4a56","skills":25,"vocabulary":"1"}] WP5-LIVE-MARKER-END
(3) the connected server is this checkout's: PASSED
    server of the session: pid 41178 ppid 41123 binary /Users/shaun/Code/DEVELOPMENT/super-rex.worktrees/0h0f8vnww4/target/debug/rex marker in its environment true command rex skills serve --name super-rex --library /Users/shaun/Code/DEVELOPMENT/super-rex.worktrees/0h0f8vnww4/plugin/library
(4) the mod's own call is not refused inside a job: NOT EXERCISED: …
```

A separate read of that transcript, not through the runner's parser:

```
init cwd /private/var/folders/t5/jttpy31x0rq99fnq9rrj1jfr0000gn/T/wp5-live-proof-DGefD9 | claude 2.1.332 | model claude-haiku-5-5 | permissionMode default
init tools with library or wp5: ['mcp__wp5-live-probe__head_probe']
init library server: [{'name': 'plugin:super-rex:library', 'status': 'connected', 'source': 'plugin'}]
tool_use mcp__wp5-live-probe__head_probe {"marker": "478dfe09-35a1-4349-8f48-9543ef5a4e10"}
tool_result WP5-LIVE-MARKER marker=478dfe09-35a1-4349-8f48-9543ef5a4e10 caller=main server=plugin:super-rex:library route=call isErr ...
result success turns 2 cost 0.00421635
```

## Run 3, control: this build left off PATH

`node tests/fixtures/wp5-live/run.ts control` is the same session, except that the runner does not put `target/debug` first on PATH. `rex` then resolves to `/Users/shaun/.cargo/bin/rex`, an older install with no `skills serve`. The control shows that each of lines 1 to 3 can fail, and that the live server really is the `rex` that PATH names. Exit 1, as expected:

```
rex on the session PATH: /Users/shaun/.cargo/bin/rex
debug: … [ERROR] "MCP server \"plugin:super-rex:library\" Server stderr: error: unrecognized subcommand 'serve'\n\nUsage: rex skills <COMMAND>\n\nFor more information, try '--help'."
debug: … [ERROR] MCP server "plugin:super-rex:library" Connection failed (CONNECTION_CLOSED): Connection closed
(1) library server connected at init: FAILED: plugin:super-rex:library has status failed
    init mcp_servers entry: {"name":"plugin:super-rex:library","status":"failed","source":"plugin"}
(2) the mod's $.mcp.call of head: FAILED: no result of mcp__wp5-live-probe__head_probe holds a marker span
    probe tool input: {"marker":"d99d856e-93d9-4fab-846a-61c3f0fb9a9c"}
    probe tool result: <tool_use_error>head_probe failed (throw): $.mcp.call(plugin:super-rex:library, head) failed with HooksError: wp5-live-probe: $.mcp.call: no connected MCP tool "head" on a server named "plugin:super-rex:library", and $.mcp.connect(plugin:super-rex:library) refused with failed: The MCP server "plugin:super-rex:library" did not connect: Connection closed.</tool_use_error>
(3) the connected server is this checkout's: FAILED: version differs; no server process of the session runs this build on this library with the marker
```

In that run the model first called `head_probe` with `{}`, which the probe refused for a missing marker, and then called it again with the marker. That is model behaviour and does not change the result.

## Why no other super-rex could have answered

- The nested session runs in a new temp folder, so no plugin of project scope applies. Its init message listed one `super-rex` plugin, the one at `<worktree>/plugin`, loaded with `--plugin-dir` (`source: super-rex@inline`).
- None of the person's cached super-rex installs (0.8.2 to 0.14.0 under `~/.claude/plugins/cache/super-rex/super-rex/`) declares `mcpServers`, so none of them can start a library server.
- The main checkout's library gives another version: `target/debug/rex skills serve --name super-rex --library /Users/shaun/Code/DEVELOPMENT/super-rex/plugin/library` answers `sha256:dc3be932fd3d7f6ffbd2065b83a4b570bbfa4693f11c4e2d9ae7af39d9713d4a`. So the version hash `sha256:ed1fae5f…` points to this worktree's library and not to the main checkout's.
- The runner read the binary of the live server process with `lsof` (the file the kernel has open), not from PATH, and its parent process was the nested `claude` process of the run.

## Limits of this proof

- The probe tool ran in the main loop of the session. WP0 already showed `$.mcp.call` from a sub-agent and from an agent that a mod started; this run does not repeat those cases.
- The call came from a probe plugin, not from super-rex's own mod, because no super-rex mod tool calls `head` yet. The engine let one plugin call another plugin's server by its `/mcp` name with no refusal.
- No job was open, so the library guard ran only in its inert path. Line 4 stays with the unit tests.
