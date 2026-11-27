---
name: full
description: A skill whose header carries every field of the metadata map.
when_to_use: Use when a test needs a skill with a correct metadata map.
metadata:
  author: a test
  kind: rule
  category: coding-standards
  claims:
    facts: { writes_code: true, languages: [rust], role: plan-reviewer }
    activity: [build, fix]
    touches: [file-layout]
    quality: [readability]
    artefact: [unit-test]
  placement: full
  scope: { writes_code: [true], languages: [rust], latency_scale: [ns_us] }
  not_when: "Generated code that nobody reads."
  intents:
    - name a public function
    - choose a function signature
    - write a doc comment
  files:
    - { path: rust.md, role: language, language: rust }
    - { path: measure.md, role: topic, when: "you have no measurement yet", intents: [measure a hot path] }
    - { path: measure-code.md, role: code, language: rust }
  open_when:
    - { skill: rex-code-error-writing, when: "you design an error type" }
    - { skill: rex-code-tdd, file: mocking.md, when: "a test crosses the file system" }
    - { file: measure.md, when: "you measure the hot path" }
  requires: [rex-code-philosophy]
  replaces: [rex-code-ergonomics]
  max_topics: 2
---
Body.
