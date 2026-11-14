# Lens 4 — Rot

Method + severity: `rex-code-smells/SKILL.md`.

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
| unused or stale dep | `cargo machete`, `cargo tree -d`, `cargo outdated` IF installed | dep still used? major versions behind? | remove or bump dep (`rex-code-external-libraries`) |
| dead cfg | `#[cfg(feature = "x")]`, x not in `[features]` | rustc `unexpected_cfgs` warning | keep `unexpected_cfgs` at warn or deny |
| stale doc example or path | commands, flags, paths in README + doc comments | `cargo test --doc` passes? command in `--help`? each path → `test -e` | doc tests run in hook. Prefer doc test to prose example |

Rot severity:

- [ ] BLOCK: comment or doc states wrong invariant, unit, order, or `SAFETY:` rule. Reader trusts it → bug. Reachable `todo!()`. Ignored test of invariant that now fails.
- [ ] REFACTOR: half migration, provisional code past its exit, suppression that hides real warning, deprecated API, stale file comment.
- [ ] NIT: commented-out code, aged TODO, suppression that hides nothing, unused dep.
