---
name: fx-proptest
description: How to write property tests with the proptest crate in Rust.
when_to_use: Use when a Rust task writes a property test with proptest.
metadata:
  kind: tool
  category: testing
  claims:
    facts: { writes_code: true, languages: [rust], repo_dependencies: [proptest] }
  placement: full
  intents:
    - write a property test with proptest
    - shrink a failing proptest case
    - generate random inputs for a test
---
# Proptest

Write a strategy for each input. Let proptest shrink a failure.
