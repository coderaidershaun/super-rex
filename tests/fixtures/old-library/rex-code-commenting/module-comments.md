# Module comment: place for each language

Rules for every language are in `.rex/skills/rex-code-commenting/SKILL.md`, section "Module comment". This file says where comment goes and which lines can come before it.

| Language | Top of file | Allowed before comment | Example |
| --- | --- | --- | --- |
| Rust | `//!` lines, before any item. | Any line. Comment starts by line 5. | `//! Reads the price feed and reconnects when it drops.` |
| Go | `//` comment above `package`. IF file documents package, THEN comment = package comment and touches `package`. ELSE put blank line between comment and `package`, so Go does not add it to package documentation. | Blank lines, license header, `//go:build` lines. | `// Reads the price feed and reconnects when it drops.` |
| C, C++ | `//` or `/* */` comment above first `#include`, before or directly after include guard. Header + source file each have one. | Blank lines, license header, include guard lines: `#pragma`, `#if…`, `#define`. | `// Declares the feed reader and its errors.` |
| Python | `"""` docstring as first statement. | Blank lines, shebang, coding line, license header. | `"""Loads the run config from disk."""` |
| JavaScript, TypeScript | `//` or `/** */` comment above first `import`. | Blank lines, license header, `"use strict"`, `"use client"`. | `// Draws the order book chart.` |
| Shell | `#` comment on line after shebang. | Blank lines, shebang, license header. | `# Builds the release archive.` |
| Java, Scala | `//`, `/* */` or `/** */` comment above `package`. | Blank lines, license header. | `/** Reads the price feed. */` |
| Kotlin | `//`, `/* */` or `/** */` comment above `package`. | Blank lines, license header, `@file:` annotations. | `// Reads the price feed.` |
| C# | `//` or `/* */` comment above first `using`. | Blank lines, license header, `#define`, `#undef`, `#nullable`, `#if…`, `#pragma`. | `// Reads the price feed.` |
| Swift | `//`, `///` or `/* */` comment above first `import`. | Blank lines, license header, `// swift-tools-version:` line. | `/// Reads the price feed.` |
| Objective-C | `//` or `/* */` comment above first `#import`. Header + source file each have one. | Same as C. | `// Declares the feed reader.` |
| Dart | `///` or `/* */` comment above `library;`, else above first `import`. | Blank lines, license header, `// @dart=` line. | `/// Reads the price feed.` |
| PHP | `/** */` or `//` comment after `<?php`, above `namespace`. | `<?php`, `declare(strict_types=1);`, blank lines, license header. | `/** Reads the price feed. */` |
| Ruby | `#` comment above first `module`, `class` or `require`. `=begin`/`=end` also read. | Blank lines, shebang, magic comments (`# frozen_string_literal: true`, `# encoding:`), license header. | `# Reads the price feed.` |
| Lua | `--`, `---` or `--[[ ]]` comment above first code. | Blank lines, shebang, license header. | `-- Reads the price feed.` |
| Haskell | `-- \|`, `--` or `{-\| -}` comment above `module`. Prose first, no `Module :` field header. | Blank lines, license header, pragmas `{-# LANGUAGE … #-}`, `{-# OPTIONS_GHC … #-}`. | `-- \| Reads the price feed.` |
| Elixir | `@moduledoc """` text, first line inside `defmodule`. `#` comment above `defmodule` also read. | Blank lines, shebang, license header, `defmodule` line. | `@moduledoc """` then `Reads the price feed.` |
| R | `#` or `#'` comment above first code. | Blank lines, shebang, license header. | `# Reads the price feed.` |
| SQL | `--` or `/* */` comment above first statement. | Blank lines, license header, `-- +goose`, `-- migrate:` lines. | `-- Stores the price feed.` |
| Zig | `//!` lines, before any declaration. | Any line. Comment starts by line 5. | `//! Reads the price feed.` |
| Perl | `#` comment on line after shebang, above `package` or `use`. POD is not read. | Blank lines, shebang, license header. | `# Reads the price feed.` |

Directive lines can also come before comment: `#!` shebang, `//go:`, `// +build`, editor lines with `-*-` or `vim:`, `coding:`, `encoding:`, `frozen_string_literal:`, `typed:`, `shellcheck`, `eslint-`, `@ts-`, `@dart=`, `swift-tools-version`, `+goose`, `migrate:`, `SPDX-License-Identifier`.

Extension clash: `.h` = C, C++ + Objective-C. `.m` = Objective-C, not MATLAB. `.pl` = Perl, not Prolog.

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
