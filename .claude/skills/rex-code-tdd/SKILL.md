---
name: rex-code-tdd
description: >
  This skill gives test-driven development with a red-green-refactor loop.
when_to_use: >
  Use when the user wants to build features or fix bugs with TDD, mentions
  "red-green-refactor", wants integration tests, or asks for test-first
  development.
disable-model-invocation: false
user-invocable: true
---

# Test-Driven Development

## Philosophy

**Core principle**: Tests must test behavior through public interfaces, not implementation details. Code can change entirely. Tests must not change.

**Good tests** = integration-style. They exercise real code paths through public APIs. They describe _what_ system does, not _how_ it does it. Good test reads like specification: "user can checkout with valid cart" tells you exactly what capability exists. These tests survive refactors because they do not care about internal structure.

**Bad tests** = coupled to implementation. They mock internal collaborators, test private methods, or assert through external means (example: direct database query instead of interface). Warning sign: test breaks when you refactor, but behavior did not change. IF you rename internal function and tests fail, THEN those tests test implementation, not behavior.

## Files of this skill

| Read | When |
| --- | --- |
| `.claude/skills/rex-code-tdd/tests.md` | IF you are not sure that a test checks behavior and not implementation. Good + bad tests. |
| `.claude/skills/rex-code-tdd/mocking.md` | IF a test crosses a system boundary: external API, database, time, randomness, file system. |
| `.claude/skills/rex-code-tdd/deep-modules.md` | Step 1 Planning, IF you look for deep modules. |
| `.claude/skills/rex-code-tdd/interface-design.md` | Step 1 Planning, IF you design an interface for testability. |
| `.claude/skills/rex-code-tdd/refactoring.md` | Step 4 Refactor, after all tests pass. |

## Anti-Pattern: Horizontal Slices

**DO NOT write all tests first, then all implementation.** This = "horizontal slicing": RED = "write all tests" and GREEN = "write all code."

This produces **crap tests**:

- Tests that you write in bulk test _imagined_ behavior, not _actual_ behavior
- You test _shape_ of things (data structures, function signatures), not user-facing behavior
- Tests become insensitive to real changes: they pass when behavior breaks, and fail when behavior is fine
- You outrun your headlights: you commit to test structure before you understand implementation

**Correct approach**: Vertical slices via tracer bullets. FOR EACH listed behavior: one test → one implementation. Each test responds to what you learned in previous cycle. You wrote code in last cycle, so you know exactly what behavior matters and how to test it.

```
WRONG (horizontal):
  RED:   test1, test2, test3, test4, test5
  GREEN: impl1, impl2, impl3, impl4, impl5

RIGHT (vertical):
  RED→GREEN: test1→impl1
  RED→GREEN: test2→impl2
  RED→GREEN: test3→impl3
  ...
```

## Workflow

### 1. Planning

When you explore codebase, use domain words that code uses. Purpose: test names and interface vocabulary match project's language. Obey priority block (`rex-code-priorities`) + warning comments in area where you work.

Before you write any code:

IF you are sub-agent, THEN never ask person direct. Take interface + behaviors from plan or brief. IF one is missing, THEN send `STATUS: QUESTIONS` to caller. Below, "user" = caller.


- [ ] Confirm with user what interface changes to make
- [ ] Confirm with user which behaviors to test (prioritize)
- [ ] Identify opportunities for deep modules (small interface, deep implementation)
- [ ] Design interfaces for testability
- [ ] List behaviors to test (not implementation steps)
- [ ] Get user approval on plan

Ask: "What should the public interface look like? Which behaviors are most important to test?"

**You cannot test everything.** Confirm with user exactly which behaviors matter most. Focus test effort on critical paths and complex logic, not every possible edge case.

### 2. Tracer Bullet

Write ONE test that confirms ONE thing about system:

```
RED:   Write test for first behavior → test fails
GREEN: Write minimal code to pass → test passes
```

This = your tracer bullet: it proves path works end-to-end.

### 3. Incremental Loop

For each remaining behavior:

```
RED:   Write next test → fails
GREEN: Minimal code to pass → passes
```

Rules:

- One test at a time
- Only enough code to pass current test
- Do not anticipate future tests
- Keep tests focused on observable behavior

### 4. Refactor

After all tests pass, look for refactor candidates:

- [ ] Extract duplication
- [ ] Deepen modules (move complexity behind simple interfaces)
- [ ] Apply SOLID principles where natural
- [ ] Consider what new code reveals about existing code
- [ ] Run tests after each refactor step

**Never refactor while RED.** Get to GREEN first.

## Checklist Per Cycle

```
[ ] Test describes behavior, not implementation
[ ] Test uses public interface only
[ ] Test would survive internal refactor
[ ] Code is minimal for this test
[ ] No speculative features added
```
