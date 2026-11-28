---
name: fx-ts-errors
description: How a TypeScript function reports a failure with a typed result instead of a throw.
when_to_use: Use when a task writes TypeScript code that can fail.
metadata:
  kind: rule
  category: coding-standards
  claims:
    facts: { writes_code: true, languages: [ts] }
  placement: full
  intents:
    - return a typed failure from typescript
    - replace a throw with a result type
    - name the cases of a failure union
---
# TypeScript errors

Return a result that names each failure. Throw only on a bug.
