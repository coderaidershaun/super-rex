---
name: fx-test-doubles
description: When a unit test may replace a collaborator with a stand-in, and how.
when_to_use: Use when a unit test needs a stand-in for a clock, a network or a store.
metadata:
  kind: method
  category: testing
  claims:
    artefact: [unit-test]
  placement: open-when
  intents:
    - replace a clock in a unit test
    - stand in for a network call
    - fake a store for one test
---
# Test doubles

Use a stand-in only at the edge of the system.
