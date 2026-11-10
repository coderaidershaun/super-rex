---
name: rex-code-smells
description: >
  This skill is a detection playbook for bad naming, code smells, brittle code,
  and code rot in Rust. It has grep sweeps, a smell catalog translated to Rust,
  brittleness signals, rot signals with the guard that stops each one, and a
  format for the findings report. It only detects problems. The fixes are in
  sister skills.
when_to_use: >
  Use when you are about to refactor unfamiliar code or change code that nobody
  touched for a long time, or when a pipeline orchestrator dispatches a smell,
  brittleness, or rot audit. Use it also when the user says "find code smells",
  "smell sweep", "audit naming", "what's brittle here", "code rot", "bit rot",
  "stale code", "what has gone stale", "old TODOs", "code quality audit",
  "review for smells", "is this maintainable", "refactor opportunities in X".
disable-model-invocation: false
user-invocable: true
---

# Smell, Brittleness + Rot Detection

This skill FINDS problems. Never fixes them. Output = findings report, file:line + evidence.
Fix rules live in sisters:

- `rex-code-ergonomics` — naming/type/signature law (fix for most Lens 1 findings)
- `rex-code-commenting` — comment law (fix for most Lens 4 comment findings)
- `rex-code-error-writing` — unwrap/Result law
- `rex-code-external-libraries` — dependency law (fix for Lens 4 dependency findings)
- `rex-code-philosophy` — deletion test, complexity budget

## Method

1. **Scope.** One file → read it whole, no sweep needed. Module/crate → mechanical sweep first, read hits after.
2. **Sweep finds CANDIDATES, reading confirms.** Never report from grep output alone. Grep hit that you did not read in context = not a finding. One false positive poisons whole report.
3. **Clean is a valid verdict.** "No findings above NIT" = success, not failure. Never invent findings to look useful. Fabricated nitpick costs more trust than empty report.
4. **Every finding = evidence + cost.** file:line, what it is, concrete way it bites (bug it invites, edit it taxes), one-line fix direction. No "consider maybe possibly".

Severity, three only:

| Level | Meaning |
|---|---|
| BLOCK | bug factory — invites wrong-value/wrong-order/panic defect |
| REFACTOR | taxes every future edit or reader |
| NIT | real but cheap — batch-fix or ignore |

## Lens 1 — Naming

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

## Lens 2 — Smell catalog (Rust translation)

| Smell | Rust form | Detect |
|---|---|---|
| Long fn | >50 production lines, does N jobs | `awk` sweep below. Confirm it is N jobs, not 1 long match |
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

## Lens 3 — Brittleness signals

Brittle = looks fine, shatters on small perturbation. Signals:

| Signal | Detect | Why it bites |
|---|---|---|
| `.unwrap()`/`.expect` on external input | `grep -rn '\.unwrap()\|\.expect(' src/` — anything downstream of network/file/env/args | first unusual input = panic in prod |
| unchecked index after split/parse | `[0]`/`[1]` on `split`/`parse` results | input shape assumption nobody wrote down |
| duplicated knowledge | same magic literal 2+ places (`grep -rn '86400'` style), same rule re-encoded | change updates N−1 copies |
| ordering dependency | `init` flags, `assert!(ready)`, "call X before Y" in docs | types do not enforce sequence — one reorder away from breakage |
| data w/o control flow | field with no writer, method with no caller — both compile green | seam half-wired. Grep BOTH sides of every seam |
| stringly-typed seam | `&str` params matched against literals deep inside | typo compiles. Enum/newtype at boundary does not |
| float in state logic | `f64` key, `==` on floats, float money accumulator | rounding error drifts state — use integers or decimal type for money and counts |
| wall clock in state transition | `now()`/`Instant::now` decides state, not only stamps time | replay or test gives different result — give time as parameter |
| brittle tests | assert on `Debug` format, exact multi-line strings, HashMap iteration order, `sleep` | harmless refactor turns suite red → people ignore red |

## Lens 4 — Rot

Rot = code that was right when written and is wrong now. Nobody edited it. World around it moved: callers, deps, compiler, plans.

- [ ] Old ≠ rotten. Stable code untouched for years = fine. Never report age alone.
- [ ] Rot = MISMATCH. Code disagrees with what it describes, depends on, or promises.
- [ ] Every rot finding names its GUARD in fix direction. Guard = mechanical check that stops same rot again.
- [ ] IF no guard is possible, THEN write `guard: none, review only`.

| Signal | Detect | Confirm by | Guard |
|---|---|---|---|
| stale comment | comment names fn/type/field → `grep -rn '<name>'` finds no item | read comment against code below it. Says X, code does Y? | edit comment in same commit as code (`rex-code-commenting`). doc links like ``[`Name`]`` + `-D rustdoc::broken_intra_doc_links` |
| stale file comment | IF `rex` on PATH, THEN `rex codebase --only code --with-context` | sentence still says what file does? No sentence = missing comment | read tree in review of each change |
| lying SMELL flag | IF `rex` on PATH, THEN `rex smells`. ELSE `grep -rn 'SMELL'` | problem it names still in code? Valid flag = known debt, NOT finding | remove flag in same commit as fix |
| aged TODO | `grep -rnE '\b(TODO\|FIXME\|XXX\|HACK)\b'` + `git blame -L <n>,<n> --date=short` | still true? still wanted? No reason = nobody knows when it is done | do it, delete it, or make it a SMELL flag with reason |
| provisional code that stayed | `grep -rniE 'temporary\|for now\|workaround\|remove (after\|once\|when)'` | exit condition already happened? Upstream bug fixed in current dep version? | comment names exit condition that a reader can test |
| commented-out code | `grep -rnE '^\s*//\s*(let\|fn\|use\|if\|match\|pub\|impl\|return)\b'` | real code, not prose example | delete. Git keeps history |
| stub that shipped | `grep -rnE '\b(todo\|unimplemented)!\('` outside tests | reachable from prod path? | `-D clippy::todo -D clippy::unimplemented` |
| suppression outlived cause | `grep -rnE '#!?\[allow\('` | remove attribute, build. No warning = dead weight. Warning = read what it hid | `#[expect(lint, reason = "…")]` — compiler warns when expectation is no longer met. `clippy::allow_attributes` bans bare `allow` |
| deprecated API | build warnings with `deprecated`, `grep -rn 'allow(deprecated)'` | replacement exists? removal in next major? | `-D deprecated` in hook |
| test that does not run | `grep -rn '#\[ignore'` | reason still holds? `cargo test -- --ignored` still passes? | `#[ignore = "reason"]`. Ignored test with no reason = finding |
| half migration | names with `_old`/`_new`/`legacy`/`v2`, two impls of one job | both paths still have callers? | finish or revert. Two live paths = two places for every fix |
| unused or stale dep | `cargo machete`, `cargo tree -d`, `cargo outdated` IF installed | dep still used? major versions behind? | dep check in pre-push hook (`rex-code-external-libraries`) |
| dead cfg | `#[cfg(feature = "x")]`, x not in `[features]` | rustc `unexpected_cfgs` warning | keep `unexpected_cfgs` at warn or deny |
| stale doc example or path | commands, flags, paths in README + doc comments | `cargo test --doc` passes? command in `--help`? each path → `test -e` | doc tests run in hook. Prefer doc test to prose example |

Rot severity:

- [ ] BLOCK: comment or doc states wrong invariant, unit, order, or `SAFETY:` rule. Reader trusts it → bug. Reachable `todo!()`. Ignored test of invariant that now fails.
- [ ] REFACTOR: half migration, provisional code past its exit, suppression that hides real warning, deprecated API, stale file comment.
- [ ] NIT: commented-out code, aged TODO, suppression that hides nothing, unused dep.

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
# rot
grep -rnE '\b(TODO|FIXME|XXX|HACK)\b' $P --include='*.rs'
grep -rniE 'temporary|for now|workaround|remove (after|once|when)' $P --include='*.rs'
grep -rnE '^\s*//\s*(let|fn|use|if|match|pub|impl|return)\b' $P --include='*.rs'   # commented-out code
grep -rnE '\b(todo|unimplemented)!\(' $P --include='*.rs'
grep -rnE '#!?\[allow\(' $P --include='*.rs'
grep -rn '#\[ignore' $P tests/ --include='*.rs'
grep -rn 'SMELL' $P --include='*.rs'
```

Rot sweep at crate scope. Run one time per crate, not one time per module:

```bash
cargo build --all-targets 2>&1 | grep -E 'deprecated|unexpected_cfgs|unfulfilled_lint_expectations'
cargo test --doc                    # lib crates only
cargo tree -d                       # duplicate versions of one dep
cargo machete                       # IF installed. ELSE write it in Not checked
rex codebase --only code --with-context   # IF rex on PATH
```

Test files: unwrap/expect are LEGAL there. Exclude `tests/` from Lens 3 unwrap sweep. Include it only for brittle-test signal. Rot applies to tests too: ignored tests, stale TODOs, dead `allow`.

## Report format

ALWAYS this shape:

```markdown
## Smell audit: <scope>

| # | Where | Lens | Smell | Sev | Evidence | Fix direction |
|---|-------|------|-------|-----|----------|---------------|
| 1 | src/foo.rs:42 | naming | bool param pair | BLOCK | `copy(a, b, true, false)` at 3 call sites | CopyOpts enum |
| 2 | src/bar.rs:10 | rot | suppression outlived cause | NIT | `#[allow(dead_code)]` on fn with 2 callers | remove attribute. Guard: `clippy::allow_attributes` |

**Clean:** <what was checked and passed — one line, so a clean verdict is visible work>
**Not checked:** <anything out of scope, e.g. tests/, generated code>
```

Sort BLOCK first. Counts in closing line: `N findings: X BLOCK / Y REFACTOR / Z NIT`.

## Scaling

- One file → read + report inline.
- One module → sweep block + read hits + report.
- Whole crate → one subagent per top-level module that runs this skill, merge tables, dedup by file:line.
- Crate-scope rot sweep → orchestrator runs it one time. Subagents skip it.
