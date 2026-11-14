---
name: rex-code-smells
description: >
  Detection playbook: bad naming, smells, brittle code, rot in Rust.
when_to_use: >
  Use when you refactor unfamiliar or old code, run an audit, or user says
  "find code smells", "smell sweep", "audit naming", "what's brittle here",
  "code rot", "bit rot", "stale code", "what has gone stale", "old TODOs",
  "code quality audit", "review for smells", "is this maintainable",
  "refactor opportunities in X".
---

# Smell, Brittleness + Rot Detection

Other language: apply intent of each rule in idiom of that language. IF language lacks Rust feature that a rule relies on, THEN obey gap skill of that language: skill for what Rust gives by default and that language does not.

This skill FINDS problems. Never fixes them. Output = findings report, file:line + evidence.
Fix rules live in sisters:

- `rex-code-ergonomics` — naming/type/signature law (fix for most Lens 1 findings)
- `rex-code-commenting` — comment law (fix for most Lens 4 comment findings)
- `rex-code-error-writing` — unwrap/Result law
- `rex-code-external-libraries` — dependency law (fix for Lens 4 dependency findings)
- `rex-code-philosophy` — complexity rules
- `rex-code-improve-codebase-architecture` — deletion test (`rex-code-improve-codebase-architecture/SKILL.md`, Three tests)

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

## Lens files

IF request asks only for naming, only for brittleness, or only for rot, THEN read that one lens file. ELSE read all four lens files.

| Lens | Read | Holds |
|---|---|---|
| 1 Naming | `rex-code-smells/naming.md` | checks for names, with sweep + confirm |
| 2 Smell catalog | `rex-code-smells/smells.md` | smells in Rust form, clippy sweep |
| 3 Brittleness | `rex-code-smells/brittleness.md` | signals that break on small change |
| 4 Rot | `rex-code-smells/rot.md` | rot rules, signals with guard, rot severity |

IF scope = module or crate, THEN read `rex-code-smells/sweep.md`. Run its sweep before you read hits.

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
