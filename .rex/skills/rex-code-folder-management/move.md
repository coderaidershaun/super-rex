# MOVE

Words (Move, Hidden behavior, Joint) and tree rules are in `.rex/skills/rex-code-folder-management/SKILL.md`. AUDIT block is in `.rex/skills/rex-code-folder-management/audit.md`.

CAUTION: Do not type code again during a move. Cut and paste the exact text. A changed attribute or doc comment can change behavior, and the compiler does not report it.

CAUTION: Find hidden behavior before the first move. A move that breaks hidden behavior passes the build and the tests.

Hidden behavior to find:

| Item | Why move can break it |
| --- | --- |
| `include_str!`, `include_bytes!` | Path is relative to file that holds macro. |
| `env!("CARGO_MANIFEST_DIR")` joins | Path names folder. |
| `///` on type with derive macro (`JsonSchema`, `Parser`) | Macro reads doc comment as data. |
| `#[path]`, `build.rs`, `mod` in macro | Path names file. |
| Module path in string | Test filter, log target, `module_path!`, `type_name`. |
| Path in docs, hooks, CI config | Text goes stale. |

Procedure:

1. Record baseline: `git rev-parse HEAD`, `git status --short`, build, clippy, fmt, doc, tests.
2. Snapshot each output that must not change. Example: generated schema, prompt file, saved file.
3. Write move map. One row for each file: old path, new path, what changes inside.
4. Write closed list: each edit that is not move. Kinds: visibility, body, deleted, added.
5. Write public path table: path before, path after. Each public item stays public.
6. Order steps so build is green after each step.
7. Each step includes `mod`, `use`, and test-import edits that it forces.
8. Use `git mv` for each file, so that history stays with the file.
9. `git mv` stages the change. Use `git diff HEAD -M` to see the work. Do not commit unless the user asks.
10. DO build + test UNTIL green, after each step. IF 3 rounds fail, THEN stop + report.

Proof of no behavior change. All must hold:

- [ ] Snapshots = byte-identical to baseline.
- [ ] Code with comments removed differs only by closed list + `mod` lines + `use` lines.
- [ ] Each warning comment + `SMELL:` comment is present, word for word.
- [ ] Tests: same names, same bodies. Only import lines differ.
- [ ] Build, clippy, fmt, doc, tests = green. Zero new warnings.
- [ ] AUDIT block (`audit.md`) gives zero defects.

Scope:

- [ ] One run = one kind of change. Move OR redesign. Never both.
- [ ] IF move shows design smell, THEN add `SMELL:` comment + report it. Do not correct it in move run.
- [ ] Parallel agents: one file group for each agent. No formatter until all agents are done.

CAUTION: Do not let a parallel agent run `git stash`, `git checkout`, `git restore`, or `git reset`. These commands delete the work of the other agents.
