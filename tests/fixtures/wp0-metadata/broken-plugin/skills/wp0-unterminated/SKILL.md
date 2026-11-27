---
name: wp0-unterminated
description: "Test skill whose header is broken YAML, with a quote that is never closed.
when_to_use: Use when a test needs a header that Claude Code cannot read.
metadata:
  kind: method
---

# WP0 unterminated fixture

This skill exists only to be loaded. Its header opens a quote and never
closes it, so a loader must warn about it.
