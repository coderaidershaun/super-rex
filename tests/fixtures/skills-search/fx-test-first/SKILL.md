---
name: fx-test-first
description: Test-driven development with a red-green-refactor loop.
when_to_use: Use when a task builds a feature or fixes a bug in code.
metadata:
  kind: method
  category: testing
  claims:
    activity: [build, fix]
  placement: full
  scope: { writes_code: [true] }
  not_when: "Work that writes no code, such as a change to a README."
  intents:
    - build a feature test first
    - fix a bug with a failing test first
    - red green refactor loop
---
# Test first

Write one failing test. Make it pass. Then clean up.
