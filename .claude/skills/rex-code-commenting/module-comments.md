# Module comment: place for each language

Rules for every language are in `.claude/skills/rex-code-commenting/SKILL.md`, section "Module comment". This file says where comment goes and which lines can come before it.

| Language | Top of file | Allowed before comment | Example |
| --- | --- | --- | --- |
| Rust | `//!` lines, before any item. | Any line. Comment starts by line 5. | `//! Reads the price feed and reconnects when it drops.` |
| Go | `//` comment above `package`. IF file documents package, THEN comment = package comment and touches `package`. ELSE put blank line between comment and `package`, so Go does not add it to package documentation. | Blank lines, license header, `//go:build` lines. | `// Reads the price feed and reconnects when it drops.` |
| C, C++ | `//` or `/* */` comment above first `#include`, before or directly after include guard. Header + source file each have one. | Blank lines, license header, include guard lines: `#pragma`, `#if…`, `#define`. | `// Declares the feed reader and its errors.` |
| Python | `"""` docstring as first statement. | Blank lines, shebang, coding line, license header. | `"""Loads the run config from disk."""` |
| JavaScript, TypeScript | `//` or `/** */` comment above first `import`. | Blank lines, license header, `"use strict"`, `"use client"`. | `// Draws the order book chart.` |
| Shell | `#` comment on line after shebang. | Blank lines, shebang, license header. | `# Builds the release archive.` |

Directive lines can also come before comment: `#!` shebang, `//go:`, `// +build`, editor lines with `-*-` or `vim:`, `shellcheck`, `eslint-`, `@ts-`, `SPDX-License-Identifier`.

Not (Go): license + comment in one run of `//` lines. Both count as license. Go loses module comment.

```go
// Copyright 2026 Example Ltd.
// Reads the price feed and reconnects when it drops.

package feed
```

Yes (file that does not document package):

```go
// Copyright 2026 Example Ltd.

// Reads the price feed and reconnects when it drops.

package feed
```
