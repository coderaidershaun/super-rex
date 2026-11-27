---
name: wp0-metadata
description: Test skill whose header holds a metadata map with nested maps and lists of maps.
when_to_use: Use when a test proves that Claude Code and rex skills check accept nested metadata.
metadata:
  kind: method
  category: testing
  claims:
    facts:
      writes_code: true
    activity: [builds_feature, fixes_bug]
    touches:
      - file_system
      - time_or_randomness
    quality: [invariant]
    artefact: [unit_test, property_test]
  placement: open-when
  scope:
    languages: [rust]
    latency_scale: ns_us
  not_when: "A task that writes no code."
  intents:
    - prove that a nested skill header loads
    - check a skill header that carries metadata
    - time one test run
  files:
    - { path: timing.md, role: topic, when: "you time one test run", intents: [time one test run] }
    - { path: timing-code.md, role: code, language: rust }
    - path: rust.md
      role: language
      language: rust
  open_when:
    - { file: timing.md, when: "you time one test run" }
    - skill: rex-code-tdd
      when: "you write the failing test first"
  requires: []
  replaces: []
  max_topics: 1
---

# WP0 metadata fixture

This skill exists only to be loaded. Its header carries every `metadata` field
of the proposed skill format, with nested maps and lists of maps.

- To time one test run, read [timing](timing.md).
- For the Rust form of the timing code, read [timing code](timing-code.md).
- For Rust notes, read [Rust](rust.md).
