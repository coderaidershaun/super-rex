---
name: rex-gap-ts
description: What TypeScript lacks that Rust gives by default, and the rule for each gap.
when_to_use: Use when a task writes TypeScript code.
metadata:
  kind: rule
  category: coding-standards
  claims:
    facts: { writes_code: true, languages: [ts] }
  placement: full
  intents:
    - write typescript with checked errors
    - avoid the any type in typescript
    - narrow a union type in typescript
---
# Gap: TypeScript

Turn on strict mode. Never use `any`.
