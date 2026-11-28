---
name: fx-failure-policy
description: What a long-running process does when input is malformed or a retry is due.
when_to_use: Use when code reads input it does not control, runs for a long time, or retries.
metadata:
  kind: rule
  category: coding-standards
  claims:
    touches: [outside-input, long-running, retry]
  placement: full
  scope: { writes_code: [true] }
  not_when: "A task that writes no code, such as a plan, a guide or a list of findings."
  intents:
    - reject malformed input
    - retry with backoff
    - keep a long-running process alive
---
# Failure policy

Name the failure class of each error, and let the supervisor act on it.
