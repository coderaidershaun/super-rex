---
name: holder
description: >
  A library skill whose body sends an agent to files of another skill.
metadata:
  kind: method
  category: testing
  files:
    - { path: notes.md, role: topic, when: "you read the notes" }
  open_when:
    - { skill: other, when: "you need the other skill" }
---

# Holder

Declared: for the loop, obey `other/SKILL.md`.
Undeclared: for the table, read `other/extra.md`.
Own file, never flagged: `holder/notes.md`.
Not a skill path, never flagged: `src/missing/file.md`.
