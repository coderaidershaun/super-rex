# Lens 2 — Smell catalog (Rust translation)

Method + severity: `.rex/skills/rex-code-smells/SKILL.md`.

| Smell | Rust form | Detect |
|---|---|---|
| Long fn | >50 production lines, does N jobs | `awk` sweep in `.rex/skills/rex-code-smells/sweep.md`. Confirm it is N jobs, not 1 long match |
| Long param list | >3 args, or 2+ same-typed | signature read. Fix = struct/builder |
| Bool flags | `f(true, false)` unreadable at call site | Lens 1 sweep. Fix = enum |
| Data clumps | same 3 params travel together across fns | they = struct that does not exist yet |
| Divergent change / shotgun surgery | same enum matched exhaustively in 3+ files | `grep -rln 'match .*<EnumName>'` — change = N edits |
| Speculative generality | trait w/ 1 impl, generic w/ 1 instantiation, `pub` w/ no external caller | deletion test: remove it. IF only ceremony vanishes, THEN it failed rent |
| Dead code | uncalled item hidden by `pub` | downgrade declaration to private → compiler `dead_code` audits it for free |
| Feature envy | method body mostly reads ANOTHER type's fields | count foreign vs own field touches. Method wants to move |
| Message chains | `a.b().c().d()` crossing module seams | 3+ hops = any refactor of middle breaks every caller |
| Middle man | type whose methods all one-line delegate | delete hop |
| Temporary field | `Option<T>` field that is `Some` only in one phase | phases = enum states, not nullable fields |
| Comment deodorant | comment explaining WHAT a block does | block = unextracted fn → `rex-code-commenting` |
| Duplicate code | same block twice w/ cosmetic diffs | second copy already drifted? That drift is the bug |

Clippy does part of this sweep mechanically. Run on scoped path. Findings are candidates:

```bash
cargo clippy --all-targets -- -W clippy::too_many_arguments -W clippy::too_many_lines \
  -W clippy::cognitive_complexity -W clippy::fn_params_excessive_bools \
  -W clippy::struct_excessive_bools -W clippy::option_option \
  -W clippy::module_name_repetitions 2>&1 | grep -A2 'warning'
```
