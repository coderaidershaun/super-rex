---
name: fx-debugging
description: Find the cause of a bug before you change the code.
when_to_use: Use when a task fixes a bug in code.
metadata:
  kind: method
  category: testing
  claims:
    activity: [fix]
  placement: full
  scope: { writes_code: [true] }
  not_when: "A change that fixes no fault, such as a new feature."
  intents:
    - find the cause of a bug
    - reproduce a fault before a fix
    - read a stack trace
---
# Debugging

Reproduce the fault. Find the first wrong value. Then fix it.
