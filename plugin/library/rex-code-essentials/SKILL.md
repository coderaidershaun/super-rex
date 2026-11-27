---
name: rex-code-essentials
description: >
  One page of the coding rules that every code task obeys, in any language:
  simplicity, names, types, comments and errors. The four full rulebooks are
  its open-when items.
when_to_use: >
  Use when a task writes or changes code, in any language.
metadata:
  kind: rule
  category: coding-standards
  claims:
    facts: { writes_code: true }
  placement: full
  intents:
    - obey the coding rules of every task
    - write simple, readable code in any language
    - name things and handle errors well
  open_when:
    - { skill: rex-code-philosophy, when: "you weigh simplicity against abstraction, tests or speed, and this page is not enough" }
    - { skill: rex-code-commenting, when: "you write a module comment, doc comment or flag, and this page is not enough" }
    - { skill: rex-code-ergonomics, when: "you design a public API, names, types or signatures, and this page is not enough" }
    - { skill: rex-code-error-writing, when: "you design error types or error messages, and this page is not enough" }
---

# Code essentials

Four rulebooks in short form. IF this page does not settle a case, THEN open its rulebook with `rex_skill`.

Each rule states intent. Names in brackets are Rust examples. Other language: apply same intent in its idiom. IF it lacks a Rust feature that a rule relies on, THEN obey `rex-gap-<language>`.

## Simplicity: `rex-code-philosophy`

- [ ] Simplest version that works. Plain beats clever. IF you can delete instead of add, THEN delete.
- [ ] No abstraction before shape is clear. Repetition costs less than wrong abstraction.
- [ ] Behavior on thing that does it: method on type, not `utils`.
- [ ] Named intermediate values, not long one-line chains.
- [ ] IF task asks for fix, THEN fix only that.
- [ ] Know why code exists before you remove it.
- [ ] Bug found: regression test first, then fix. Prefer integration tests. Never mock own code.
- [ ] Profile before you optimize.

## Names and types: `rex-code-ergonomics`

- [ ] Full words, no abbreviations: `user_count`, not `usr_cnt`. Boolean = predicate: `is_active`.
- [ ] Verb names function. Noun names type. Type name carries meaning: `UserId`, not `Id`.
- [ ] Own type for each domain primitive. Invariant in type, not runtime check.
- [ ] No boolean flag in public function: use enum. More than 3 arguments: use struct.
- [ ] Take most general input type (`&str`, `&[T]`). Return concrete type. Smallest visibility that works.

## Comments: `rex-code-commenting`

- [ ] Default: no comment. First rename, extract, or name constant.
- [ ] Comment only non-obvious WHY: hidden constraint, trap, outside reference. One or two plain sentences.
- [ ] Never: restated code, commented-out code, history, vague TODO, plan or task reference, path of other file, jargon.
- [ ] Change comment with its code. Stale comment is worse than none.
- [ ] Each source file starts with 1 to 2 line comment: why file exists.
- [ ] Smell that you cannot fix now: flag it, `SMELL:` plus reason.

CAUTION: Never delete a `SMELL:`, warning, hazard or `SAFETY:` comment while its problem stays in code. A deleted flag hides a known problem.

## Errors: `rex-code-error-writing`

- [ ] Library: one typed error for each concept, so caller can match it (`thiserror`).
- [ ] Program entry point: add context each time error passes up (`anyhow`, `.context(...)`).
- [ ] Public API of library: no untyped or text-only error (`Box<dyn Error>`, `Result<T, String>`).
- [ ] Message names what code tried and value seen: `failed to read /etc/foo.toml`.
- [ ] Bug = crash. Bad input, missing file, network failure = returned error (`Result`).
- [ ] Outside tests, no unchecked crash on error (`.unwrap()`). At boundary, say why it cannot fail (`.expect("why")`).
- [ ] Never swallow error (`let _ =`). Return or handle it. Log alone is not handling.
- [ ] No text-only catch-all variant (`Other(String)`). No match on error text.
