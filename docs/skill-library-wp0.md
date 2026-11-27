# Skill library, WP0 findings

This file records the result of work package WP0 of the skill-library spec (`superrex-skill-library-spec.md`, sections 7.1, 8.1, 8.3 and 12). WP0 tests three technical assumptions and one manifest key before later work depends on them.

Two provers wrote the proofs. This file comes from a third run, by an integrator who wrote no proof. The integrator reran every proof on the merged job branch and checked each prover's verdict against the rerun.

- Branch: `rex/0h0962klnd`, commit `1bd4e96` (SuperRex 0.14.0 plus both provers' work).
- Run on 2026-11-27 with Claude Code 2.1.332, node v24.4.0, tsc 5.7.3, SQLite 3.54.1 (bundled).
- Nested `claude -p` sessions used for the reruns: 8 (5 for assumption 2, 3 for assumption 3).

## Verdicts

| Item | Verdict | Run or read | Later work must change anything? |
| --- | --- | --- | --- |
| 1. FTS5 is in the bundled SQLite of `rusqlite` | PASSED | Run | No. |
| 2. Claude Code loads nested `metadata` with no warning; `rex skills check` passes it | PASSED | Run | No for the assumption. Four side findings for WP1, WP4 to WP7 (see below). |
| 3a. `$.mcp.call` from the main session | PASSED | Run | No. |
| 3b. `$.mcp.call` from a sub-agent's tool call | PASSED | Run | No. |
| 3c. proxy: `$.mcp.call` from a grid agent that a mod started with `$.agent.spawn` | PASSED (proxy only) | Run | See the open design point on hiding the server's tools. |
| 3c. real: a builder that a real master started | NOT EXERCISED | Not run | Cause: a master tree opens only from a person's own prompt. |
| Manifest key that declares an MCP server: `mcpServers` | PASSED | Run, and read from documentation and typings | No. |

"Run" means a command ran in this environment and its output is quoted below. "Read" means the claim comes from a document or source only. No item here is only read, except the statement that the typings never spell the manifest key.

## What I reran

Every command ran in `/Users/shaun/Code/DEVELOPMENT/super-rex.worktrees/0h0962klnd`. No proof, test, fixture or production file was changed.

| Command | Result |
| --- | --- |
| `cargo build` | exit 0, `Finished dev profile` |
| `cargo test` | exit 0, every `test result` line is `ok` (quoted below) |
| `tsc -p plugin` | exit 0 |
| `tsc -p tests/fixtures/wp0-mcp/probe-plugin/tsconfig.json` | exit 0 |
| `tsc -p tests/fixtures/wp0-mcp/tsconfig.json` | exit 0 |
| `claude plugin test plugin` | exit 0, `545 pass`, `0 fail`, `Ran 545 tests across 64 files` |
| `node tests/fixtures/wp0-metadata/run-live-load.mts` (all modes) | exit 0, 5 nested sessions |
| `node tests/fixtures/wp0-mcp/run.ts a b c` | exit 0, 3 nested sessions, all three `verdict: PASSED` |

`cargo test` result lines:

```
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out   (lib)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out    (bin rex)
test result: ok. 39 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out   (contract)
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out   (fitness)
test result: ok. 98 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out   (integration)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out    (doc tests)
```

The six wp0 test lines, from `cargo test --test integration wp0 -- --nocapture`:

```
test wp0::fts5::bundled_sqlite_is_compiled_with_fts5 ... ok
test wp0::fts5::bm25_column_weights_of_the_spec_rank_name_then_intents_then_card ... ok
test wp0::skill_metadata::metadata_fixture_nests_every_field_and_names_only_real_files_and_skills ... ok
test wp0::skill_metadata::skills_check_passes_the_metadata_fixture_and_its_baseline_in_a_plugin ... ok
test wp0::skill_metadata::skills_check_passes_the_metadata_fixture_in_a_project_claude_skills_folder ... ok
test wp0::skill_metadata::skills_check_refuses_each_broken_header_of_the_control_fixture ... ok
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 92 filtered out; finished in 0.41s
```

### Where my rerun differs from the provers' reports

1. **Case 3c, final answer.** In the first prover's run the main session waited and its final answer was the marker. In my run the main session's final text said the agent had started and its answer had not arrived yet. The marker is still in the transcript, and the stub logged the call by the spawned agent. The verdict does not change. Do not rely on the main session's final text in a headless run to carry a spawned agent's answer.
2. **Session count of the first prover.** The first prover reported 6 nested runs because it ran one `read` session before the third broken fixture existed. The runner's current `all` mode makes 5 sessions. I ran 5.
3. **`rex skills check` path.** The first prover's report shows `rex skills check` from the fixture folder. The command needs the `skills/` folder of a plugin as its path. `rex skills check metadata-plugin` fails with `is a folder with no SKILL.md and no skill folder in it`. `rex skills check metadata-plugin/skills` gives the `ok` line. The tests use the right paths. Only the report's shorthand was loose.

No verdict differs.

## Assumption 1: FTS5 is in the bundled SQLite of `rusqlite`

**Verdict: PASSED. Run.**

**What ran.** `tests/integration/wp0/fts5.rs` makes `CREATE VIRTUAL TABLE skills USING fts5(name, card, intents)` in memory with 8 rows. Every column of every row has three words. The word `latency` is in the name of one row, the card of a second and the intents of a third. Each match therefore scores the same per column, and only the column weights decide the order.

**Result, quoted:**

```
sqlite_version() = 3.54.1; sqlite_compileoption_used('ENABLE_FTS5') = 1
weights 4, 2, 3: ["latency budget rules", "error types guide", "testing first method"]
weights 1, 4, 2: ["testing first method", "error types guide", "latency budget rules"]
```

With the spec's `bm25(skills, 4.0, 2.0, 3.0)` the order is name, then intents, then card. With other weights the order reverses, so the weights take effect. Versions: `rusqlite 0.41.1`, `libsqlite3-sys 0.39.1`, feature `bundled`.

**If it had failed.** Lookup C of section 8.3 would need a build flag or a fallback to plain `LIKE` ranking. It did not fail.

**What later work does.** WP4 uses FTS5 and `bm25()` with weights 4, 2, 3 as written. No new feature or flag is needed.

## Assumption 2: nested `metadata` loads with no warning, and `rex skills check` passes it

**Verdict: PASSED. Run.**

**What ran.**

- The fixture skill `tests/fixtures/wp0-metadata/metadata-plugin/skills/wp0-metadata/SKILL.md` uses every field of section 7.1: `kind`, `category`, `claims` (a nested `facts` map plus lists), `placement`, `scope` (a nested map), `not_when`, `intents`, `files` (a list of maps, in flow and block style), `open_when` (a list of maps), `requires`, `replaces` and `max_topics`.
- A baseline copy has no `metadata`.
- A control plugin has three broken headers: tab-indented, an unclosed `[`, and an unclosed `"`.
- The same three fixtures run through `rex skills check`, `claude plugin validate`, `claude plugin details` and nested headless sessions.

**`rex skills check`, quoted (my run, built binary):**

```
$ rex skills check metadata-plugin/skills
ok metadata-plugin/skills/wp0-metadata                 (exit 0)
$ rex skills check baseline-plugin/skills
ok baseline-plugin/skills/wp0-metadata                 (exit 0)
$ rex skills check broken-plugin/skills
broken-plugin/skills/wp0-broken/SKILL.md: H2: the header is not valid YAML: tabs disallowed within this context (block indentation) at byte 175 line 5 column 2
broken-plugin/skills/wp0-unclosed/SKILL.md: H2: the header is not valid YAML: illegal placement of ':' indicator at byte 118 line 3 column 12
broken-plugin/skills/wp0-unterminated/SKILL.md: H2: the header is not valid YAML: invalid indentation in quoted scalar at byte 36 line 2 column 14
error: 3 findings in 3 of 3 skills                     (exit 1)
```

The test `skills_check_passes_the_metadata_fixture_in_a_project_claude_skills_folder` shows the same pass for a skill in a project's `.claude/skills/`.

**Claude Code static checks, quoted:**

```
claude plugin validate metadata-plugin            exit 0   Validation passed   (only warning: ICON_MISSING)
claude plugin validate --strict metadata-plugin   exit 0   Validation passed
claude plugin validate metadata-plugin/skills     exit 0   Validation passed
claude plugin validate broken-plugin              exit 1   ✘ frontmatter: YAML frontmatter failed to parse: YAML Parse error: Unexpected character. At runtime this skill loads with empty metadata (all frontmatter fields silently dropped).
claude --plugin-dir metadata-plugin plugin details wp0-metadata   -> wp0-metadata ~80 always-on, ~130 on-invoke
claude --plugin-dir baseline-plugin plugin details wp0-baseline   -> wp0-metadata ~80 always-on, ~130 on-invoke
```

The metadata skill costs the same tokens as the baseline.

**Live load, quoted.** Five headless sessions, each in a new temp folder outside the repo (`--model haiku`, `--debug-file`, `--max-turns 1`). The init events:

```
metadata skill in a plugin:     skills with wp0: ["wp0-metadata:wp0-metadata"]
baseline skill in a plugin:     skills with wp0: ["wp0-baseline:wp0-metadata"]
broken headers in a plugin:     skills with wp0: ["wp0-broken:wp0-broken","wp0-broken:wp0-unclosed"]
metadata skill in .claude/skills: skills with wp0: ["wp0-metadata"]
```

The fifth session loaded all fixtures and turned on the Skill tool, so the model could quote its listing:

```
- wp0-metadata => Test skill whose header holds a metadata map with nested maps and lists of maps. - Use when a test proves that Claude Code and rex skills check accept nested metadata.
- wp0-metadata:wp0-metadata => (same line)
- wp0-baseline:wp0-metadata => (same line)
- wp0-broken:wp0-broken => Test skill whose header is broken YAML, indented with tabs. - Use when ...
- wp0-broken:wp0-unclosed => [Test skill whose header is broken YAML, with a list that is never closed. - Use when ...
```

The only header warning in that session's debug log:

```
[WARN] Failed to parse YAML frontmatter in .../broken-plugin/skills/wp0-unterminated/SKILL.md: YAML Parse error: Unexpected character
```

**Why this counts as "no warning".** The positive control works. The unclosed-quote skill gets a validate error, a `[WARN]` debug line and a missing entry in the `skills` list. The metadata skill gets none of these. Its listing line is character for character the baseline's.

**Limit of this proof.** `rex skills check` passes the fixture today because the current checker does not read `metadata`. WP1 adds `metadata` parsing and new checks. The pass for the final checker is WP1's own test, not this one.

**If it had failed.** Section 7.1 would have to move the library fields out of the header into a sidecar file. It did not fail.

### Side findings for later work

| # | Finding | Who acts |
| --- | --- | --- |
| 1 | Claude Code's header parser is looser than `rex skills check`. A tab-indented header and an unclosed `[` load with no warning anywhere, and their `description` is still read. `rex skills check` refuses both with H2. Only an unclosed quote makes Claude Code warn. | WP1: keep H2 as the only guard. A header that Claude Code accepts can still be invalid YAML for the rex parser and the library index. Do not use tabs or an unclosed `[` as a positive control in later tests. |
| 2 | `claude plugin validate` does not police header keys. The prover saw no warning for an unknown top-level key or for a `name` that is not the folder name. | WP1: H3 and H8 are the only checks of those. |
| 3 | Claude Code shows the model only `description` and `when_to_use`. It passes no `metadata`. | WP4 to WP7: the library server and the mod read `metadata` from `SKILL.md` themselves. Everything an agent needs to choose a skill from the plain listing must be in `description` and `when_to_use`. |
| 4 | With `--tools ""` a headless model sees no skill listing. | Any later eval that asks a headless model about skills must turn on the Skill tool. |
| 5 | `claude plugin details` takes an installed plugin name, not a path. For a folder use `claude --plugin-dir <dir> plugin details <name>`. `rex skills check` takes the `skills/` folder, not the plugin root. | Anyone who scripts these commands. |

## Assumption 3: a mod calls its own plugin's MCP server with `$.mcp.call`

The three cases share one proof, so I describe it once. Each case has its own verdict below.

**The proof.** The fixture plugin `wp0-mcp-probe` (`tests/fixtures/wp0-mcp/probe-plugin/`) lists one stdio server in its manifest under the key `mcpServers`: `{"stub": {"command": "node", "args": ["${CLAUDE_PLUGIN_ROOT}/server/stub.ts"]}}`. The stub is a hand-written MCP server (JSON-RPC over stdio, node built-ins only). Its tool `echo` returns the nonce it gets and writes one line to `$WP0_MCP_LOG`. The fixture mod registers two tools:

- `mcp_probe` calls `$.mcp.call("plugin:wp0-mcp-probe:stub", "echo", {nonce, caller})` and wraps the answer in `WP0-MCP-MARKER ... WP0-MCP-MARKER-END`.
- `spawn_probe` starts a `super-rex:model-low-effort-low` agent with `$.agent.spawn` in another folder.

Each session denies the model every stub tool (`--disallowedTools mcp__plugin_wp0-mcp-probe_stub 'mcp__plugin_wp0-mcp-probe_stub__*'`). In every run the init message offered the model no stub tool, and the marker text exists only in the mod's source. A marked answer with the run's nonce therefore came through the mod's `$.mcp.call`, and the stub's own log shows who called.

**Type checks.** All three exit 0, as quoted in the table at the top: `tsc -p plugin`, the probe plugin's tsconfig, and the runner's tsconfig.

### 3a. Main session

**Verdict: PASSED. Run.**

```
nonce: d2c94044-0895-43ca-b979-339ee41f22e9
stub server at init: plugin:wp0-mcp-probe:stub connected
stub tools offered to the model: none
tool calls in the transcript: mcp__wp0-mcp-probe__mcp_probe in main
marker: WP0-MCP-MARKER caller=main server=plugin:wp0-mcp-probe:stub route=call isError=false answer=[stub-echo nonce=d2c94044-0895-43ca-b979-339ee41f22e9 caller=main pid=28700] WP0-MCP-MARKER-END
log: {"event":"tools/call","tool":"echo","nonce":"d2c94044-0895-43ca-b979-339ee41f22e9","caller":"main"}
```

`route=call` means the direct `$.mcp.call` worked. The fallback `$.mcp.connect` was not needed.

**If it had failed.** WP6 would call `$.mcp.connect("<key>")` first, or the mod would read the library without the server. It did not fail.

### 3b. A sub-agent's tool call

**Verdict: PASSED. Run.**

The main session started one `general-purpose` sub-agent and told it to call the probe.

```
tool calls in the transcript: Agent in main, mcp__wp0-mcp-probe__mcp_probe in toolu_01GbJ1ktzjXSaG9J12it9uSJ
marker: WP0-MCP-MARKER caller=ad8adcb7134e9074c server=plugin:wp0-mcp-probe:stub route=call isError=false answer=[stub-echo nonce=42f82dfe-972f-421a-8e61-bef171889c64 caller=ad8adcb7134e9074c pid=28715] WP0-MCP-MARKER-END
log: {"event":"tools/call","tool":"echo","nonce":"42f82dfe-972f-421a-8e61-bef171889c64","caller":"ad8adcb7134e9074c"}
```

The caller is the sub-agent's id, not `main`. Nothing for later work to change.

### 3c. A builder that a master started

This case has two lines, because only a proxy could run.

**3c, line 1. Proxy: a grid agent that the mod started with `$.agent.spawn`. Verdict: PASSED (proxy only). Run.**

```
tool calls in the transcript: mcp__wp0-mcp-probe__spawn_probe in main, mcp__wp0-mcp-probe__mcp_probe in toolu_plugin_31a63969151349f0945a793dc8447b1e
spawned agent: a6a2f46f9ce3a7933
marker: WP0-MCP-MARKER caller=a6a2f46f9ce3a7933 server=plugin:wp0-mcp-probe:stub route=call isError=false answer=[stub-echo nonce=ac6208fb-1f05-4b64-b9b1-b944fcfa2c38 caller=a6a2f46f9ce3a7933 pid=28734] WP0-MCP-MARKER-END
log: {"event":"tools/call","tool":"echo","nonce":"ac6208fb-1f05-4b64-b9b1-b944fcfa2c38","caller":"a6a2f46f9ce3a7933"}
```

The agent that `$.agent.spawn` started is the agent that called the probe. The probe's `$.mcp.call` reached the stub from a mod-spawned agent.

**3c, line 2. A builder that a real master started. Verdict: NOT EXERCISED.**

Cause: a master tree opens only from a person's own prompt of the master skill. This environment has no person, so no master tree, no `rex_tree_open` and no `rex_build` ran. Nothing here shows that this case passes.

**What the proxy leaves out:**

- No master tree and no charter.
- No `rex_tree_open` or `rex_build` gate.
- No job, no job worktree, no builder brief and no `pendingSpawns` entry.
- No `linkBuilder` and no `permissionRefusal` check. Those run only when the spawn's origin is `super-rex`.
- No `bypassPermissions` run. The proxy ran in permission mode `default`.
- No interactive master session.
- The builder's working folder was not checked from inside the builder, because it had no tool that could print it.
- The installed super-rex mod stayed inert in the nested session. The repo's own `plugin/` was loaded only for its grid agent types.

**Residual risk, as the second prover judged it.** The real path adds super-rex's own `agent.spawn` hook. It records the spawn and may deny it, and it never changes the tool pool or MCP connections of the builder it lets through. I read this in `plugin/hooks/master/build.ts` and `spawn.ts` as the prover reported, and did not re-check it. It is a judgement, not a proof.

**What later work must do about 3c.**

- WP6 (or WP7, where the mod builds the brief) must run a first live check of the library call from a real builder. Do it in the first job that has a master tree and a person present, before any gate relies on it.
- If it fails there, the builder gets the library text another way: the mod fetches the skill with `$.mcp.call` at spawn time and puts it in the builder's brief, so the builder never calls the server itself.

### Findings for WP5 and WP6 (carried over from the second prover)

1. **Manifest entry.** Declare the server in `plugin/.claude-plugin/plugin.json` under `mcpServers`, as an inline map keyed by name. A `.mcp.json` at the plugin root also works. Today `plugin.json` lists no servers and the repo has no `.mcp.json`. Example:

   ```json
   "mcpServers": { "library": { "command": "rex", "args": ["skills", "serve", "--library", "${CLAUDE_PLUGIN_ROOT}/library"] } }
   ```

   Claude Code starts the server with the session. In the runs the server's cwd was the session's folder and it inherited the parent's environment.
2. **Server name.** The mod calls the server as `$.mcp.call("plugin:super-rex:<key>", tool, args)`. The direct call worked in all three runs. WP6 can also call `$.mcp.connect("<key>")` once, to get the name and to see a refusal reason (`unlisted`, `disabled`, `failed`) for the `library_stale` or dead-library flag.
3. **The library server's own tools will be visible to agents.** They appear to every agent as `mcp__plugin_super-rex_<key>__<tool>`, behind ToolSearch. `--disallowedTools` hid them from the model and did not block the mod's `$.mcp.call`.
   - **Open design point for the owner.** If agents must reach the library only through the mod's tools (`rex_skill_search` and the others), WP5 or WP6 must hide the raw tools. Options are a deny rule, or a mod `tool.call` hook that refuses `mcp__plugin_super-rex_<key>__*` from any agent.
   - Neither option was tested. The deny rule was shown to hide tools from the model while leaving the mod's call working, in the fixture only.
4. **Hook time limit.** A `tool.call` hook has 10 s of its own time (`HookBudget.ms`, `typings/claude-code/index.d.ts` line 5463). The time of a `$` call such as `$.mcp.call` does not count against it. Any parsing the hook does after the call does count.
5. **Spawned agents run in the background.** `$.agent.spawn` resolves when the agent starts. A hook must not wait in its own code for the agent's answer. My rerun of 3c shows the same thing: the main session's final text came before the agent's answer.

## The manifest key

**Verdict: `mcpServers`. PASSED. Run, and read from documentation and typings.**

**Run.** `claude plugin validate tests/fixtures/wp0-mcp/probe-plugin`, quoted:

```
  ❯ mcpServers: stub stdio node ["${CLAUDE_PLUGIN_ROOT}/server/stub.ts"]
✔ Validation passed
```

The only warning was `ICON_MISSING`. In all three live runs the init message listed `plugin:wp0-mcp-probe:stub` with status `connected`, so Claude Code read the key and started the server.

**Documentation, read.** I fetched https://code.claude.com/docs/en/plugins-reference on 2026-11-27. The Fields table:

> | `mcpServers` | Path, object, or array of either | `.json` MCP config files, `.mcpb` or `.dxt` bundles, or inline server configs keyed by name. Loaded together with `.mcp.json`; a server name declared later replaces an earlier one |

The `mcpServers` section: "Claude Code loads `.mcp.json` at the plugin root first, then each declared shape in order. A server name declared later replaces an earlier one." Its Inline map row: `{ "deploy-api": { "command": "node", "args": ["${CLAUDE_PLUGIN_ROOT}/server.js"] } }`. The Standard layout table lists `MCP servers | .mcp.json | MCP server definitions`. The page also says a top-level key that Claude Code does not recognise is stripped, so a misspelt key would load the plugin with no server and a validate warning.

**Typings, read.** `typings/claude-code/index.d.ts`:

- Lines 2894-2895, on `$.mcp.connect`: "Connects an MCP server if the session has not already: one this plugin's own manifest lists, by its key, or any server /mcp lists."
- Lines 2911-2912: "@param server the server's key in this plugin's manifest, or its name as /mcp lists it".
- Lines 6319-6320: "The name /mcp lists it under and `$.mcp.call` takes: usually `plugin:<plugin>:<server>`."
- Lines 2873-2880, on `$.mcp.call`: "with the engine's own connection and credentials ... No permission prompt: the plugin's call, seen by the hooks above it, is the grant."

The typings never spell the manifest key. The only `mcpServers` in them (line 530) is a field of an agent spec. The key therefore rests on the documentation and the live run, not on the typings. The spec's wording in section 8.1 ("confirm the exact manifest key") is now closed: it is `mcpServers`, plural, camel case.

**If it had failed.** The server would go in a `.mcp.json` at the plugin root. The documentation says that file is read too. No prover ran that form, so it stays read only.

## What later work packages do

| Package | Action |
| --- | --- |
| WP1 | Parse `metadata` in `header.rs`. Keep H2 as the YAML guard (Claude Code is looser). Keep H3 and H8, since `claude plugin validate` does not check keys. Do not use tab indents or an unclosed `[` as a negative control. |
| WP4 | Use FTS5 with `bm25(skills, 4.0, 2.0, 3.0)` as written. Read `metadata` from `SKILL.md` yourself. Claude Code never passes it on. |
| WP5 | Declare the server under `mcpServers` in `plugin/.claude-plugin/plugin.json`. Decide, with the owner, whether and how to hide the server's raw tools from agents. |
| WP6 | Call `plugin:super-rex:<key>` with `$.mcp.call`. Use `$.mcp.connect` once for the refusal reason. Never wait in a hook for a spawned agent. Run the first live check from a real builder (case 3c, line 2) and keep the brief-injection fallback ready. |
| Owner | Decide the open design point: raw library tools visible to agents or hidden. |
