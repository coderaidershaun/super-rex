---
name: rex-code-commenting
description: >
  Hard rules for code comments. Best comment is no comment. Comment only a
  non-obvious WHY, in short plain English. Flags for smells, hazards, and
  SAFETY invariants are permanent.
when_to_use: >
  Use when you write, review, or tidy comments, JSDoc, docstrings, or TODO
  markers, or commit AI-generated code, or the user asks "should I add a
  comment", "document this", "explain this code".
disable-model-invocation: false
user-invocable: true
---

# Code Commenting — Hard Rules

These rules HARD. No compromise. No "but my case different".

Sister skill: `rex-code-philosophy`. Same complexity-averse, ergonomics-first frame.

Sources: "shocking truth about comments", refine.dev.

## Files of this skill

| Read | When |
| --- | --- |
| `.claude/skills/rex-code-commenting/module-comments.md` | IF you write or check comment at top of file. Place for each language. |
| `.claude/skills/rex-code-commenting/examples.md` | IF you want bad + good comments, or quotables. |

## Iron law #1 — best comment is no comment

Default: **no comment**. Code self-document. Comment = design smell. Try fix design first.

Test before you write anything: *can competent programmer who never saw this code understand it from code alone?* IF yes, THEN **write no comment**. Silence = goal, not fallback.

Self-evident code gets nothing. No preamble above obvious fn. No summary of block that reads fine. No comment added because file "looks bare" or because comment feels expected. Zero comments in file = healthy file.

Comment earns its place only IF answer is truly no. Then only under [Iron law #2](#iron-law-2--comment-only-why-non-obvious-never-what) and [Iron law #3](#iron-law-3--plain-english-anyone-can-read).

## Iron law #2 — comment only WHY non-obvious, NEVER WHAT

Code show what. Name show what. Structure show what. Comment job = capture what code *cannot* say:
- Why this approach over obvious alternative
- Hidden constraint (rate limit, legacy bug, vendor quirk)
- Trap future reader will hit
- External context (ticket ref, regulatory rule, benchmark result)

IF WHY obvious from name + structure, THEN delete comment.

## Iron law #3 — plain English anyone can read

**Every comment that survives = short, simple, plain English.**

Write for competent programmer who joined yesterday. Reader knows nothing about plan that produced this code. Reader reads under pressure at 2am. Reads it once and understands.

- Full, grammatical sentences. Short, but never cryptic.
- Simple everyday words. IF plainer word exists, THEN use it.
- One or two sentences. IF comment needs paragraph, THEN code needs refactor, not essay.
- No telegraphic fragments, no shorthand like "b/c" or "w/", no caveman style.
- **No jargon.** No internal codenames, no unexplained acronyms, no architecture vocabulary ("seam", "adapter", "deepening", "port") in code comments. That vocabulary is for design discussion. In comment, say plain thing instead.
- **No references to plans, tasks, or working process.** No "as per the plan", no "phase 2", no "step 3 of the refactor", no "TODO from the review". Reader never saw plan and never will.
- **No pointers to other source files.** No "see src/foo.rs", no "moved from bar.rs", no line numbers. Paths rot. Comment lies within one month. Describe constraint, not location.

This skill document uses compressed words to save tokens. Comments in code do NOT. Write them plain. Bad + good: `.claude/skills/rex-code-commenting/examples.md`.

## Forbidden — delete on sight

- **Restate-what comments.** `// increment counter` above `counter++`. Insult reader.
- **Stale comments.** Lie silently for years. Worse than nothing. Wrong > missing.
- **Commented-out code.** Git remember. Delete. No "backup" blocks. No "old version below".
- **History comments.** `// 2024-03: added PayPal`. Use `git log` + commit message.
- **TODO/FIXME drift.** Vague promise = abandoned. Fix now, delete it, OR make it SMELL flag with reason (whitelist #7). Repo with issue tracker: ticket ref also OK (whitelist #4). No middle.
- **Design band-aid.** IF comment explain why code weird, THEN refactor code, not annotate. Exception: IF refactor truly not possible now, THEN flag it as code smell (whitelist #7). Flag REQUIRED then, not forbidden.
- **Section dividers in long fns.** `// --- validation ---`. Function too long. Extract instead.
- **AI-generated noise.** LLM produce verbose what-comments. Strip ruthlessly on commit.
- **Redundant docstring.** `@param userId — the user id`. Type say it. Delete.
- **Plan / process references.** `// per the plan`, `// phase 2`, `// step 3 of the refactor`, `// as agreed in review`. Plan dead once code shipped. Reader never saw it. Delete.
- **Cross-file pointers.** `// see src/loader/mod.rs`, `// moved from parser.rs`, `// mirrors line 42 in engine.rs`. Paths + line numbers rot fast. Say constraint plain, or say nothing.
- **Jargon + private shorthand.** Internal codenames, unexplained acronyms, architecture vocabulary ("seam", "adapter", "deepening"). Rewrite plain or delete.

## Module comment: every file, every language

Each module starts with comment of 1-2 lines. Comment says why module exists. Plain English, why not what.

Place for each language: `.claude/skills/rex-code-commenting/module-comments.md`.

Rules for every language:

- [ ] First sentence stands alone, ends with full stop, says why file exists. It does not restate file name.
- [ ] IF file has license header, THEN write license first, then blank line, then module comment.
- [ ] Nothing else comes before comment. Only lines in column "Allowed before comment" of `module-comments.md` + directive lines.
- [ ] No tag (example: `@file`, `@brief`) comes before first sentence.
- [ ] Exception to 1-2 lines: crate root can hold priority block after first sentence, max 8 lines (`rex-code-priorities`).

## Permitted — narrow whitelist

1. **WHY non-obvious.** "Manual sort because the stdlib is O(n²) on near-sorted input; benchmarks show this is 4x faster."
2. **Warning.** "Do not lower this timeout below 30 seconds — it breaches the vendor SLA."
3. **Edge case trap.** "The API returns null instead of an empty list when there are no results; handled below."
4. **Durable external ref.** Permanent record only — ticket ID, spec section, vendor doc. "The vendor caps this at 10 requests a second. See ticket PROJ-1234." NOT working plan, NOT another source file.
5. **Public API doc.** JSDoc/docstring on exported surface. Generated docs consume it.
6. **Algorithm conceptual frame.** "Boyer-Moore search — the skip table is built once and reused for every search."
7. **Code-smell / tech-debt flag.** "SMELL: this reads the viewer's private fields directly, which breaks every time the viewer changes. Needs a proper interface." Known smell that you cannot fix now: you MUST flag it. Same for hazards, known limitations, `SAFETY:` invariants.

That is all. Nothing else.

**Whitelist entries 2 and 7 are permanent.** ALWAYS include + keep warnings, code-smell flags, hazard notes, `SAFETY:` blocks, known-limitation notes. Cleanup passes, comment sweeps, and AI hygiene passes must never delete them. Tighten wording. Never remove flag while its problem is in code. Removal of one = new silent trap. Remove flag only in commit that fixes problem it names.

## Self-document first (try in order)

Before write any comment, attempt these. In order:

1. **Rename.** `calc(p, t)` -> `calculatePriceWithTax(price, taxRate)`. Often kill comment need.
2. **Extract fn.** Inline block w/ explanation -> named fn. Name = comment.
3. **Replace magic number.** `if x > 3` -> `if x > AccessLevel.ADMIN`. Constant = comment.
4. **Replace bool flag.** `process(true, false)` -> `process({skipCache: true, retry: false})`.
5. **Decompose.** Big fn w/ section comments -> small fns. Section = fn name.
6. **Pattern consistency.** Every controller `validate -> process -> respond`. Pattern = doc.

IF still need comment, THEN write minimal one.

## Decision flow

```
Want add comment?
  ├─ Code self-evident to a stranger? -> No comment. Best comment = no comment.
  ├─ Restating what code do? -> DELETE. No comment.
  ├─ Can rename/extract/refactor remove need? -> DO THAT. No comment.
  ├─ History? -> git. No comment.
  ├─ TODO? -> fix now, delete, or SMELL flag with reason.
  ├─ About the plan, the task, or another file? -> DELETE, or rewrite as the plain constraint.
  ├─ Code smell / hazard you cannot fix now? -> FLAG IT. Required. Permanent.
  ├─ WHY non-obvious / warning / edge / ref / public API? -> WRITE. One or two plain sentences.
  └─ Unsure? -> No comment. Reader read code.
```

Then reread what you wrote. IF it has jargon, acronym, plan reference, or file path, THEN rewrite in plain words or delete.

## Maintenance discipline

- **Comment = code.** Edit comment when edit code. Stale = bug.
- **Review comments in PR same as code.** Catch drift early.
- **AI commit hygiene.** LLM-generated PR -> scan for what-comments. Strip before merge.
- **Never strip flags.** Code-smell, warning, hazard, `SAFETY:` comments survive every sweep. Tighten wording only.
- **Rewrite survivors to plain English.** Sweep converts cryptic, telegraphic, or jargon-laden comments into short plain English. Sweep removes plan references + file pointers from comments that stay.
- **Comment older than function = suspect.** Re-read. Make sure still true. Update or delete.

## Ergonomics check

Before commit, ask:

- **Code as clear with this comment deleted?** IF yes, THEN delete it.
- **Necessary context, or excuse for unclear code?** IF latter, THEN refactor instead.
- **New joiner with no project knowledge understands every word?** IF no, THEN rewrite plain.
- **Mentions plan, task, phase, or another file?** IF yes, THEN remove that part.
- **Will this comment lie in 6 months?** IF yes, THEN delete or pin to constant.
- **Reader read code first or comment first?** Code. Comment supplement only.
- **Can name carry this?** IF yes, THEN rename.

IF any fail, THEN reconsider.
