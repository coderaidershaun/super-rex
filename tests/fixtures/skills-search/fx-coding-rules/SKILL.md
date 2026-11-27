---
name: fx-coding-rules
description: Rules for names, types and function signatures in any language.
when_to_use: Use when a task writes or changes code.
metadata:
  kind: rule
  category: coding-standards
  claims:
    facts: { writes_code: true }
  placement: full
  intents:
    - name a public function
    - choose a function signature
    - write a clear error message
  files:
    - { path: rust.md, role: language, language: rust }
    - { path: go.md, role: language, language: go }
  open_when:
    - { skill: fx-test-first, when: "you write the first test of a change" }
---
# Coding rules

Name each thing for what it holds. Keep each function short.
