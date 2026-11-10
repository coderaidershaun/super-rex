---
name: rex-code-commenting
description: Hard rules for code comments. The best comment is no comment — self-evident code gets none. Comment only WHY non-obvious, never WHAT. Every comment that survives is short plain English anyone can read — no jargon, no acronyms, no shorthand, no references to plans, tasks, or other files. Comments flagging code smells, hazards, or SAFETY invariants are permanent — no cleanup pass may delete them. Distilled from "shocking truth about comments" + refine.dev. Use when writing/reviewing comments, JSDoc, docstrings, TODO markers, cleaning up comments, or when user asks "should I add a comment", "document this", "explain this code", or AI-generated code is being committed.
disable-model-invocation: false
user-invocable: true
---

# Code Commenting — Hard Rules

These rules HARD. No compromise. No "but my case different".

Sister skill: `rex-code-philosophy`. Same complexity-averse, ergonomics-first frame.

## Iron law #1 — the best comment is no comment

Default: **no comment**. Code self-document. Comment = design smell. Try fix design first.

Test before writing anything: *could a competent programmer who has never seen this code work it out from the code alone?* Yes -> **write no comment**. Silence = the goal, not the fallback.

Self-evident code gets nothing. No preamble above obvious fn. No summary of block that reads fine. No comment added because the file "looks bare" or because a comment feels expected. Zero comments in a file = healthy file.

Only when the answer is genuinely no does a comment earn its place — and then only under [Iron law #2](#iron-law-2--comment-only-why-non-obvious-never-what) and [Iron law #3](#iron-law-3--plain-english-anyone-can-read).

## Iron law #2 — comment only WHY non-obvious, NEVER WHAT

Code show what. Name show what. Structure show what. Comment job = capture what code *cannot* say:
- Why this approach over obvious alternative
- Hidden constraint (rate limit, legacy bug, vendor quirk)
- Trap future reader will hit
- External context (ticket ref, regulatory rule, benchmark result)

If WHY obvious from name + structure -> delete comment.

## Iron law #3 — plain English anyone can read

**Every comment that survives = short, simple, plain English.**

Write for a competent programmer who joined yesterday, knows nothing about the plan that produced this code, and is reading under pressure at 2am. They read it once and understand.

- Full, grammatical sentences. Short, but never cryptic.
- Simple everyday words. If a plainer word exists, use it.
- One or two sentences. Needs a paragraph -> the code needs a refactor, not an essay.
- No telegraphic fragments, no shorthand like "b/c" or "w/", no caveman style.
- **No jargon.** No internal codenames, no unexplained acronyms, no architecture vocabulary ("seam", "adapter", "deepening", "port") in code comments. That vocabulary is for design discussion. In a comment, say the plain thing instead.
- **No references to plans, tasks, or working process.** No "as per the plan", no "phase 2", no "step 3 of the refactor", no "TODO from the review". The reader has never seen the plan and never will.
- **No pointers to other source files.** No "see src/foo.rs", no "moved from bar.rs", no line numbers. Paths rot, and the comment lies within a month. Describe the constraint, not the location.

This skill document is written compressed to save tokens. Comments in code are NOT. Write them plain.

```rust
// bad — telegraphic
// mnl sort: stdlib O(n²) near-sorted. 4x per bench.

// bad — jargon
// Adapter at the ingest seam; deepens the module by hiding the transport.

// bad — plan reference
// Phase 2 of the refactor plan: temporary until the loader lands. See src/loader/mod.rs.

// good — short plain English
// Manual sort because the stdlib is O(n²) on near-sorted input; benchmarks show this is 4x faster.

// good — short plain English
// The exchange sends prices as strings, so parse before comparing.
```

## Forbidden — delete on sight

- **Restate-what comments.** `// increment counter` above `counter++`. Insult reader.
- **Stale comments.** Lie silently for years. Worse than nothing. Wrong > missing.
- **Commented-out code.** Git remember. Delete. No "backup" blocks. No "old version below".
- **History comments.** `// 2024-03: added PayPal`. Use `git log` + commit message.
- **TODO/FIXME drift.** Vague promise = abandoned. File ticket OR fix now. No middle.
- **Design band-aid.** Comment explain why code weird -> refactor code, not annotate. Exception: refactor genuinely not possible now -> flag it as a code smell (whitelist #7). Flag REQUIRED then, not forbidden.
- **Section dividers in long fns.** `// --- validation ---`. Function too long. Extract instead.
- **AI-generated noise.** LLM produce verbose what-comments. Strip ruthlessly on commit.
- **Redundant docstring.** `@param userId — the user id`. Type say it. Delete.
- **Plan / process references.** `// per the plan`, `// phase 2`, `// step 3 of the refactor`, `// as agreed in review`. Plan dead once code shipped. Reader never saw it. Delete.
- **Cross-file pointers.** `// see src/loader/mod.rs`, `// moved from parser.rs`, `// mirrors line 42 in engine.rs`. Paths + line numbers rot fast. Say the constraint plain, or say nothing.
- **Jargon + private shorthand.** Internal codenames, unexplained acronyms, architecture vocabulary ("seam", "adapter", "deepening"). Rewrite plain or delete.

## Module comment: every file, every language

Each module starts with comment of 1-2 lines. Comment says why module exists. Plain English, why not what.

| Language | Top of file | Allowed before comment | Example |
| --- | --- | --- | --- |
| Rust | `//!` lines, before any item. | Any line. Comment starts by line 5. | `//! Reads the price feed and reconnects when it drops.` |
| Go | `//` comment above `package`. IF file documents package, THEN comment = package comment and touches `package`. ELSE put blank line between comment and `package`, so Go does not add it to package documentation. | Blank lines, license header, `//go:build` lines. | `// Reads the price feed and reconnects when it drops.` |
| C, C++ | `//` or `/* */` comment above first `#include`, before or just after include guard. Header + source file each have one. | Blank lines, license header, include guard lines: `#pragma`, `#if…`, `#define`. | `// Declares the feed reader and its errors.` |
| Python | `"""` docstring as first statement. | Blank lines, shebang, coding line, license header. | `"""Loads the run config from disk."""` |
| JavaScript, TypeScript | `//` or `/** */` comment above first `import`. | Blank lines, license header, `"use strict"`, `"use client"`. | `// Draws the order book chart.` |
| Shell | `#` comment on line after shebang. | Blank lines, shebang, license header. | `# Builds the release archive.` |

Directive lines are also allowed before comment: `#!` shebang, `//go:`, `// +build`, editor lines with `-*-` or `vim:`, `shellcheck`, `eslint-`, `@ts-`, `SPDX-License-Identifier`.

Rules for every language:

- [ ] First sentence stands alone, ends with full stop, says why file exists. It does not restate file name.
- [ ] IF file has license header, THEN write license first, then blank line, then module comment.
- [ ] Nothing else comes before comment. Only lines in column "Allowed before comment" + directive lines.
- [ ] No tag (example: `@file`, `@brief`) comes before first sentence.

Not (Go): license + comment in one run of `//` lines. Both count as license. Module comment is lost.

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

## Allowed — narrow whitelist

1. **WHY non-obvious.** "Manual sort because the stdlib is O(n²) on near-sorted input; benchmarks show this is 4x faster."
2. **Warning.** "Do not lower this timeout below 30 seconds — it breaches the vendor SLA."
3. **Edge case trap.** "The API returns null instead of an empty list when there are no results; handled below."
4. **Durable external ref.** A permanent record only — ticket ID, ADR number, spec section, vendor doc. "The vendor caps this at 10 requests a second. See ticket PROJ-1234." NOT the working plan, NOT another source file.
5. **Public API doc.** JSDoc/docstring on exported surface. Generated docs consume it.
6. **Algorithm conceptual frame.** "Boyer-Moore search — the skip table is built once and reused for every search."
7. **Code-smell / tech-debt flag.** "SMELL: this reads the viewer's private fields directly, which breaks every time the viewer changes. Needs a proper interface." Known smell that cannot be fixed now MUST be flagged. Same for hazards, known limitations, `SAFETY:` invariants.

That's it. Nothing else.

**Whitelist entries 2 and 7 are permanent.** Warnings, code-smell flags, hazard notes, `SAFETY:` blocks, known-limitation notes must ALWAYS be included and kept. No cleanup pass, comment sweep, or AI hygiene pass may delete them — tighten the wording, never remove the flag. Removing one = introducing a silent trap.

## Self-document first (try in order)

Before write any comment, attempt these. In order:

1. **Rename.** `calc(p, t)` -> `calculatePriceWithTax(price, taxRate)`. Often kill comment need.
2. **Extract fn.** Inline block w/ explanation -> named fn. Name = comment.
3. **Replace magic number.** `if x > 3` -> `if x > AccessLevel.ADMIN`. Constant = comment.
4. **Replace bool flag.** `process(true, false)` -> `process({skipCache: true, retry: false})`.
5. **Decompose.** Big fn w/ section comments -> small fns. Section = fn name.
6. **Pattern consistency.** Every controller `validate -> process -> respond`. Pattern = doc.

Still need comment? OK. Now write minimal one.

## Decision flow

```
Want add comment?
  ├─ Code self-evident to a stranger? -> No comment. Best comment = no comment.
  ├─ Restating what code do? -> DELETE. No comment.
  ├─ Can rename/extract/refactor remove need? -> DO THAT. No comment.
  ├─ History/TODO? -> git/ticket. No comment.
  ├─ About the plan, the task, or another file? -> DELETE, or rewrite as the plain constraint.
  ├─ Code smell / hazard you cannot fix now? -> FLAG IT. Required. Permanent.
  ├─ WHY non-obvious / warning / edge / ref / public API? -> WRITE. One or two plain sentences.
  └─ Unsure? -> No comment. Reader read code.
```

Then reread what you wrote. Jargon, acronym, plan reference, or file path in it? -> rewrite in plain words or delete.

## Maintenance discipline

- **Comment = code.** Edit comment when edit code. Stale = bug.
- **Review comments in PR same as code.** Catch drift early.
- **AI commit hygiene.** LLM-generated PR -> scan for what-comments. Strip before merge.
- **Never strip flags.** Code-smell, warning, hazard, `SAFETY:` comments survive every sweep. Tighten wording only.
- **Rewrite survivors to plain English.** Sweep converts cryptic, telegraphic, or jargon-laden comments into short plain English, and strips plan references + file pointers out of the ones worth keeping.
- **Comment older than function = suspect.** Re-read. Verify still true. Update or delete.

## Ergonomics check

Before commit, ask:

- **Would the code be just as clear with this comment deleted?** Yes -> delete it.
- **Necessary context, or excusing unclear code?** Latter -> refactor instead.
- **Would a new joiner understand every word, with no project knowledge?** No -> rewrite plain.
- **Does it mention a plan, a task, a phase, or another file?** Yes -> strip that out.
- **Will this comment lie in 6 months?** Yes -> delete or pin to constant.
- **Reader read code first or comment first?** Code. Comment supplement only.
- **Could a name carry this?** Yes -> rename.

Fail any -> reconsider.

## Quotables (carry these)

- "The best comment is the one you didn't need to write."
- "Comments rot faster than code."
- "Most comments don't improve quality. They create illusion of clarity."
- "Comment = duct tape for poor design."
- "Write code as if comments didn't exist. Then add the few that must exist."
- "Stale comment > silence in damage. Wrong worse than missing."
