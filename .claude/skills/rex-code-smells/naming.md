# Lens 1 — Naming

Method + severity: `.claude/skills/rex-code-smells/SKILL.md`.

| Check | Candidate sweep | Confirm by |
|---|---|---|
| bool not a predicate | `grep -n ': bool' src/` minus `is_\|has_\|can_\|should_` | field/param read as yes/no question? |
| abbreviations | `grep -rnE '\b(mgr|cnt|idx2?|tmp|val|buf|proc|calc|usr|obj)\b'` | `cfg`/`ctx`/loop `i` idiomatic — NOT findings |
| noise words | type/fn names containing `data\|info\|manager\|helper\|util\|item\|thing` | does name say what it IS? |
| `get_` prefix | `grep -rn 'fn get_'` | plain getter → field name. `get` only for indexed access |
| stuttering | `module::ModuleThing` — grep module name inside its own files | path already carries context |
| unit-less quantity | `timeout\|delay\|size\|interval\|duration` params w/o unit suffix | unit belongs in name: `_ms`, `_us`, `_bytes` |
| conversion prefix lies | `fn as_` that allocates, `fn to_` that is free, `fn into_` taking `&self` | `as_`=free view, `to_`=work, `into_`=consumes. Callers infer COST from prefix |
| synonym drift | same concept named `order`/`record`/`entry` across fns | one concept one name, greppability |
| primitive obsession | pub fns taking 2+ bare `u64`/`i64`/`String` | swap-safe? `transfer(from: u64, to: u64)` compiles wrong — newtype |
