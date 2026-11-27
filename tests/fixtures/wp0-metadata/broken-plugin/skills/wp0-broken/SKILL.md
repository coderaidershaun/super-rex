---
name: wp0-broken
description: Test skill whose header is broken YAML, indented with tabs.
when_to_use: Use when a test needs a header that Claude Code cannot read.
metadata:
	kind: method
	claims:
		facts:
			writes_code: true
---

# WP0 broken fixture

This skill exists only to be loaded. Its header is indented with tabs, which
YAML does not allow, so a loader must warn about it.
