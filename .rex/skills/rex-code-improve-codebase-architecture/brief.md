# 4. Brief

Direct invocation only. This step edits no code. It ends with brief for builder.

Words, rules + stop conditions: `.rex/skills/rex-code-improve-codebase-architecture/SKILL.md`. Answers come from step 3 (`.rex/skills/rex-code-improve-codebase-architecture/grill-self.md`).

## 4.1 Proof before brief

Hold Proof outside repo, in scratchpad. Put its path in brief.

- [ ] Snapshot each output that must not change. Example: generated schema, prompt, saved file.
- [ ] Copy of source with comments removed, for code-only diff.
- [ ] List of each warning comment + `SMELL:` comment, word for word. IF `rex` is on PATH, THEN `rex smells` lists each `SMELL:` comment.
- [ ] Baseline from 1.2.

## 4.2 Write brief

Give brief to builder. No confirm step.

Brief is for agents with no context. Write it in full sentences. It must hold:

- [ ] What code does, in 3 lines. What task is. Owner's words, quoted.
- [ ] Files in scope. Current friction.
- [ ] Target tree + target interface. Seam placement. Dependency category.
- [ ] Hard constraints, numbered. Include each rule from `rex-code-folder-management` that applies.
- [ ] Parallel builders: each builder gets one file group. No file is in two groups. CAUTION: No builder runs `git stash`, `git checkout`, `git restore`, or `git reset`. These commands delete the work of other builders.
- [ ] Hidden behavior, each item named, with rule: "move by cut and paste, never type again".
- [ ] Proof: path + checks from 4.3, as hard constraints. Builder runs checks after each stage.
- [ ] Tests that must stay. Tests that must never run.
- [ ] "Done means" list that a person can confirm line by line.

Brief must be literal. Builder must be able to compile it in head.

- [ ] Each re-export that moved item needs is written out.
- [ ] Each edit that is not move is on closed list.
- [ ] No step leaves build red.
- [ ] "Done" line is true as written. Say which public paths change.

## 4.3 Checks for builder

Put these checks in brief. Builder runs Proof after each stage.

- [ ] Build, clippy, fmt, doc, tests = green. Zero new warnings.
- [ ] Snapshots = byte-identical.
- [ ] Code-only diff = closed list + `mod` lines + `use` lines. Nothing else.
- [ ] Each flag comment is present, word for word.
- [ ] AUDIT block (`.rex/skills/rex-code-folder-management/audit.md`) gives zero defects. No file > 500 lines.

IF check fails, THEN stage is not done.
