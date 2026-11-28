---
name: fx-unit-tests
description: Rules for small tests that run one function with no outside world.
when_to_use: Use when a task writes a unit test.
metadata:
  kind: rule
  category: testing
  claims:
    artefact: [unit-test]
  placement: full
  scope: { writes_code: [true] }
  not_when: "The task writes no code."
  intents:
    - write a unit test for one function
    - keep a unit test fast
    - name a unit test for its behaviour
  files:
    - { path: shared-setup.md, role: topic, when: "you share setup between tests" }
  open_when:
    - { file: shared-setup.md, when: "you share setup between tests" }
---
# Unit tests

Test one behaviour in each test. Open `shared-setup.md` when you share setup between tests.
