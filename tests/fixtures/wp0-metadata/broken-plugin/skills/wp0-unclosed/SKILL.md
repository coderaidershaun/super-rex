---
name: wp0-unclosed
description: [Test skill whose header is broken YAML, with a list that is never closed.
when_to_use: Use when a test needs a header that Claude Code cannot read.
metadata:
  kind: method
---

# WP0 unclosed fixture

This skill exists only to be loaded. Its header opens a YAML list and never
closes it, so a loader must warn about it.
