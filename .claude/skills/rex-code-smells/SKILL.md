---
name: rex-code-smells
description: Detection playbook for bad naming, code smells, and brittle code in Rust. Grep sweeps + Rust-translated smell catalog + brittleness signals + findings report format. Use whenever user says "find code smells", "smell sweep", "audit naming", "what's brittle here", "code quality audit", "review for smells", "is this maintainable", "refactor opportunities in X", or before refactoring unfamiliar code, or pipeline orchestrator dispatches a smell/brittleness audit. Detection only — fixes live in sister skills.
disable-model-invocation: false
user-invocable: true
---

# Smell + Brittleness Detection

This skill FINDS problems. Never fixes them. Output = findings report, file:line + evidence.
Fix rules live in sisters:

- `rex-code-ergonomics` — naming/type/signature law (the fix for most Lens 1 findings)
- `rex-code-commenting` — comment law
- `rex-code-error-writing` — unwrap/Result law
- `rex-code-philosophy` — deletion test, complexity budget

## Method

1. **Scope.** One file → read it whole, no sweep needed. Module/crate → mechanical sweep first, read hits after.
2. **Sweep finds CANDIDATES, reading confirms.** Never report from grep output alone — a grep hit you didn't read in context is not a finding. One false positive poisons the whole report.
3. **Clean is a valid verdict.** "No findings above NIT" = success, not failure. Never invent findings to look useful — a fabricated nitpick costs more trust than an empty report.
4. **Every finding = evidence + cost.** file:line, what it is, the concrete way it bites (the bug it invites, the edit it taxes), one-line fix direction. No "consider maybe possibly".

Severity, three only:

| Level | Meaning |
|---|---|
| BLOCK | bug factory — invites a wrong-value/wrong-order/panic defect |
| REFACTOR | taxes every future edit or reader |
| NIT | real but cheap — batch-fix or ignore |

## Lens 1 — Naming

| Check | Candidate sweep | Confirm by |
|---|---|---|
| bool not a predicate | `grep -n ': bool' src/` minus `is_\|has_\|can_\|should_` | field/param read as yes/no question? |
| abbreviations | `grep -rnE '\b(mgr|cnt|idx2?|tmp|val|buf|proc|calc|usr|obj)\b'` | `cfg`/`ctx`/loop `i` idiomatic — NOT findings |
| noise words | type/fn names containing `data\|info\|manager\|helper\|util\|item\|thing` | does name say what it IS? |
| `get_` prefix | `grep -rn 'fn get_'` | plain getter → field name; `get` only for indexed access |
| stuttering | `module::ModuleThing` — grep module name inside its own files | path already carries context |
| unit-less quantity | `timeout\|delay\|size\|interval\|duration` params w/o unit suffix | the unit belongs in the name: `_ms`, `_us`, `_bytes` |
| conversion prefix lies | `fn as_` that allocates, `fn to_` that's free, `fn into_` taking `&self` | `as_`=free view, `to_`=work, `into_`=consumes. Callers infer COST from prefix |
| synonym drift | same concept named `order`/`record`/`entry` across fns | one concept one name, greppability |
| primitive obsession | pub fns taking 2+ bare `u64`/`i64`/`String` | swap-safe? `transfer(from: u64, to: u64)` compiles wrong — newtype |

## Lens 2 — Smell catalog (Rust translation)

| Smell | Rust form | Detect |
|---|---|---|
| Long fn | >50 production lines, does N jobs | `awk` sweep below; confirm it's N jobs, not 1 long match |
| Long param list | >3 args, or 2+ same-typed | signature read; fix = struct/builder |
| Bool flags | `f(true, false)` unreadable at call site | Lens 1 sweep; fix = enum |
| Data clumps | same 3 params travel together across fns | they're a struct that doesn't exist yet |
| Divergent change / shotgun surgery | same enum matched exhaustively in 3+ files | `grep -rln 'match .*<EnumName>'` — change = N edits |
| Speculative generality | trait w/ 1 impl, generic w/ 1 instantiation, `pub` w/ no external caller | deletion test: remove it — if only ceremony vanishes, it failed rent |
| Dead code | uncalled item hidden by `pub` | downgrade declaration to private → compiler `dead_code` audits it for free |
| Feature envy | method body mostly reads ANOTHER type's fields | count foreign vs own field touches; method wants to move |
| Message chains | `a.b().c().d()` crossing module seams | 3+ hops = any refactor of the middle breaks every caller |
| Middle man | type whose methods all one-line delegate | delete the hop |
| Temporary field | `Option<T>` field that's `Some` only in one phase | phases = enum states, not nullable fields |
| Comment deodorant | comment explaining WHAT a block does | the block is an unextracted fn → `rex-code-commenting` |
| Duplicate code | same block twice w/ cosmetic diffs | second copy already drifted? that's the bug arriving |

Clippy does part of this sweep mechanically — run on the scoped path, findings are candidates:

```bash
cargo clippy --all-targets -- -W clippy::too_many_arguments -W clippy::too_many_lines \
  -W clippy::cognitive_complexity -W clippy::fn_params_excessive_bools \
  -W clippy::struct_excessive_bools -W clippy::option_option \
  -W clippy::module_name_repetitions 2>&1 | grep -A2 'warning'
```

## Lens 3 — Brittleness signals

Brittle = looks fine, shatters on small perturbation. The signals:

| Signal | Detect | Why it bites |
|---|---|---|
| `.unwrap()`/`.expect` on external input | `grep -rn '\.unwrap()\|\.expect(' src/` — anything downstream of network/file/env/args | first unusual input = panic in prod |
| unchecked index after split/parse | `[0]`/`[1]` on `split`/`parse` results | input shape assumption nobody wrote down |
| duplicated knowledge | same magic literal 2+ places (`grep -rn '86400'` style), same rule re-encoded | change updates N−1 copies |
| ordering dependency | `init` flags, `assert!(ready)`, "call X before Y" in docs | sequence not enforced by types — one reorder away from breakage |
| data w/o control flow | field with no writer, method with no caller — both compile green | seam half-wired; grep BOTH sides of every seam |
| stringly-typed seam | `&str` params matched against literals deep inside | typo compiles; enum/newtype at boundary doesn't |
| float in state logic | `f64` key, `==` on floats, float money accumulator | rounding error drifts the state — use integers or a decimal type for money and counts |
| wall clock in state transition | `now()`/`Instant::now` deciding state, not just stamping | a replay or a test gives a different result — pass the time in |
| brittle tests | assert on `Debug` format, exact multi-line strings, HashMap iteration order, `sleep` | harmless refactor turns suite red → red gets ignored |

## Mechanical sweep block

Copy-paste for module/crate scope (adjust path):

```bash
P=src/target/module
# long fns: lines from `fn` to the closing brace with the same indent (tests included) — confirm by reading
find "$P" -name '*.rs' -exec awk 'FNR==1{n=0} /^ *(pub(\([a-z]+\))? )?(const )?(async )?(unsafe )?fn / && $0 !~ /[;}] *$/ {n=FNR; f=$0; ind=match($0,/[^ ]/)} n && /^ *}$/ && match($0,/[^ ]/)==ind {if (FNR-n>50) print FILENAME":"n" ("FNR-n" lines) "f; n=0}' {} +
grep -rn '\.unwrap()\|\.expect(' $P --include='*.rs'
grep -rnE '\b(mgr|cnt|tmp|usr|obj|proc)\b' $P --include='*.rs'
grep -rn 'fn get_' $P --include='*.rs'
grep -rnE 'fn [a-z_]+\(.*bool.*\)' $P --include='*.rs'
grep -rnE '\)\.[a-z_]+\(\)\.[a-z_]+\(\)\.[a-z_]+\(' $P --include='*.rs'   # 3-hop chains
```

Test files: unwrap/expect are LEGAL there. Exclude `tests/` from Lens 3 unwrap sweep; include it only for the brittle-test signal.

## Report format

ALWAYS this shape:

```markdown
## Smell audit: <scope>

| # | Where | Lens | Smell | Sev | Evidence | Fix direction |
|---|-------|------|-------|-----|----------|---------------|
| 1 | src/foo.rs:42 | naming | bool param pair | BLOCK | `copy(a, b, true, false)` at 3 call sites | CopyOpts enum |

**Clean:** <what was checked and passed — one line, so a clean verdict is visible work>
**Not checked:** <anything out of scope, e.g. tests/, generated code>
```

Sort BLOCK first. Counts in a closing line: `N findings: X BLOCK / Y REFACTOR / Z NIT`.

## Scaling

- One file → read + report inline.
- One module → sweep block + read hits + report.
- Whole crate → one subagent per top-level module running this skill, merge tables, dedup by file:line.
