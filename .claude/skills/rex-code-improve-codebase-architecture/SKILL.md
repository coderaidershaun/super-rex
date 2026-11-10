---
name: rex-code-improve-codebase-architecture
description: >
  Finds architecture friction in a codebase, ranks the candidates, and designs
  the best one. It makes shallow modules deep and makes the folder tree tell
  how the code works. It uses the domain language in CONTEXT.md, the decisions
  in docs/adr/, and the tree rules in rex-code-folder-management. It runs with
  no questions to the user: it picks the target, builds nothing, and ends with
  a design brief that tells the builder how to prove that behavior did not
  change. Use this skill when
  the user wants to improve architecture, find refactor candidates, group or
  tidy modules, join coupled modules, or make code easier to test and to
  navigate. Use it also when the user says "improve architecture", "refactor
  opportunities", "deepen modules", "tidy up modules", or "get ready for the
  next feature".
disable-model-invocation: false
user-invocable: true
---

# Improve Codebase Architecture

Find friction. Rank candidates. Design best candidate. Give brief to builder. Brief tells how to prove behavior did not change.
This skill builds nothing. It edits no code.
Aim: code that is easy to test, easy to navigate, easy to call.

## Words

Use these exact terms in each suggestion. Full definitions: [LANGUAGE.md](LANGUAGE.md).

| Term | Meaning |
| --- | --- |
| Module | Interface + implementation. Function, type, file, folder, or crate. |
| Interface | All that caller must know: types, invariants, error modes, order, config. |
| Implementation | Code inside module. |
| Depth | Leverage at interface. Deep = much behavior behind small interface. |
| Shallow | Interface is almost as complex as implementation. |
| Seam | Place where behavior changes with no edit in that place. Home of interface. |
| Adapter | Concrete thing that satisfies interface at seam. |
| Leverage | What caller gets from depth. |
| Locality | What maintainer gets from depth: one place for change, bug, knowledge. |
| Friction | Cost that reader or caller pays because of module shape. |
| Candidate | One change that removes friction. |
| Proof | Evidence that behavior did not change. |
| Lens mode | Other skill loads this file to review only. |
| Brief | Design for builder. Output of step 4. |
| Builder | Caller, agent, or person that builds design from brief. |

Forbidden words: "component", "service", "API", "boundary". Say module, interface, seam.

Three tests:

- [ ] Deletion test: delete module. IF complexity vanishes, THEN module = pass-through. IF complexity comes back in N callers, THEN module earns its place.
- [ ] Interface = test surface. IF test must go past interface, THEN module has wrong shape.
- [ ] One adapter = hypothetical seam. Two adapters = real seam. No new seam for one adapter.

## Skills + files that this skill loads

| When | Load |
| --- | --- |
| Each run, before step 1 | `.claude/skills/rex-code-folder-management/SKILL.md` — tree rules, placement table, AUDIT, MOVE |
| Step 2, to judge names + call sites | `.claude/skills/rex-code-ergonomics/SKILL.md` |
| Step 3, dependencies of candidate | [DEEPENING.md](DEEPENING.md) |
| Step 3, IF candidate changes interface | [INTERFACE-DESIGN.md](INTERFACE-DESIGN.md) |

Domain language names good seams. `CONTEXT.md` has domain terms. `docs/adr/` has decisions that this skill must not open again without cause.

## RULES: no questions

Run alone from start to end. Invocation = permission.

- [ ] Ask user nothing. Not before start, not during work, not before brief.
- [ ] No scope question. IF invocation names path, THEN scope = that path. ELSE scope = whole repo, minus vendored + generated code.
- [ ] No permission question. No "shall I proceed".
- [ ] No choice question. Break ties yourself.
- [ ] IF fact is unknown, THEN pick sane assumption, write it in report, continue.
- [ ] Findings, decisions, risks go in final report. Never in question.
- [ ] IF user sends message during run, THEN message = hard constraint. Add it to brief. Continue. Do not ask user to confirm.
- [ ] IF decision is hard, or tie, or load-bearing, THEN get second opinion.

Second opinion: dispatch second subagent. Give it file paths, short list, trade-off. It gives recommendation + reasons. You decide. Second opinion = always subagent, never user.

## Lens mode

IF other skill or agent loads this file as review lens, THEN:

- [ ] Do step 1 + step 2 only.
- [ ] Report findings. Change nothing. Write no brief.

Only direct invocation runs step 3 + step 4.

## 1. Explore

Facts first. No advice in this step.

### 1.1 Read

- [ ] `CONTEXT.md` + `docs/adr/` for scope. IF file does not exist, THEN continue.
- [ ] Document that invocation names: spec, plan, goal. List what next work imports from this code.
- [ ] `CODEBASE.md`, IF it exists. ELSE IF `rex` is on PATH, THEN run `rex codebase --rust-only --with-context`.
- [ ] Entry files: `lib.rs`, `main.rs`, `Cargo.toml`.

### 1.2 Baseline

- [ ] Record `git rev-parse HEAD` + `git status --short`.
- [ ] Run build, clippy, fmt check, doc, tests. Record each result.
- [ ] IF build is red, THEN stop. Report. Change nothing.
- [ ] Never run ignored tests or tests that spend money.

### 1.3 Map

Run AUDIT block of `rex-code-folder-management`. It gives: file sizes, children of each folder, module graph, forbidden names, `pub` items that no caller reaches.

Then list each item declaration for each file:

```bash
grep -rnE '^\s*(pub(\([a-z ]+\))? )?(async )?(fn|struct|enum|trait|type|const|static|impl) ' src
```

### 1.4 Explore agent

Dispatch one agent (`subagent_type=Explore`). Ask for facts with file:line, in tables. Do not ask for advice. Do not give your hypothesis.

Questions to ask:

1. Which public items does code outside the library use, and through which paths? Which public items have no outside user?
2. Which items does a module other than the definer use? Give definer and users. List each cycle.
3. Which types are saved to disk or sent out, and where do they live?
4. Which files hold two topics? Give line ranges for each topic.
5. What is dead: template code, items with no caller, items that only their own test uses?
6. What hidden behavior exists? See the table in `rex-code-folder-management`, section MOVE.
7. Which inline tests exist, and which private items does each one use?

CAUTION: Do not edit files while the Explore agent reads them. Wait for its report.

### 1.5 Friction signals

Mark each signal that is true. Give file:line for each.

- [ ] To understand one concept, reader opens many small modules.
- [ ] Tree does not tell jobs of crate. Folder rule is broken.
- [ ] Module is shallow. Apply deletion test.
- [ ] Cycle between modules. Type lives in module that does not own it.
- [ ] One file holds two topics: types + calls, rule + I/O.
- [ ] Shared data module imports job module.
- [ ] Knowledge is in N places: same constant, same list, same rule.
- [ ] Pure functions exist for tests, but bug hides in how caller joins them. No locality.
- [ ] Test goes past interface, or code is hard to test through interface.
- [ ] Interface is wider than caller needs: `pub` module that only tests use.
- [ ] Next work (from 1.1) must import item that is private, split, or has wrong home.
- [ ] File > 400 lines.

## 2. Rank

Write numbered list. Sort by leverage x locality gain. Each candidate:

| Field | Content |
| --- | --- |
| Files | Files + modules in candidate |
| Problem | Friction, with file:line evidence |
| Solution | Plain words: what changes |
| Kind | Move (no code change) OR redesign (interface or logic changes) |
| Benefits | Locality + leverage + test gain |
| Ergonomics | Result of ERGONOMICS CHECK |
| Risk | Hidden behavior, public path change, size of diff |

Rules:

- [ ] Use `CONTEXT.md` terms for domain. Use Words table for architecture. Say "Order intake module", not "FooBarHandler".
- [ ] Each candidate must pass ERGONOMICS CHECK. IF it fails, THEN drop it or change it.
- [ ] Each candidate must obey each rule of `rex-code-folder-management`.
- [ ] IF candidate conflicts with ADR, THEN list it only when friction is large. Mark it: "contradicts ADR-0007, open again because …".
- [ ] One run = one kind. Join move candidates into one run. Never mix move + redesign.
- [ ] IF scope has move candidates + redesign candidates, THEN do move first. Clean tree makes redesign small.
- [ ] Pick top candidate yourself. IF tie between top 2 or 3, THEN get second opinion, then decide.
- [ ] Keep list of candidates not taken, each with one-line reason. It goes in report.

IF lens mode, THEN stop here + report.

## 3. Grill self

Walk design tree for chosen candidate alone. Write each answer. Answers become brief for step 4.

- [ ] Constraints: what must not change? Public paths, saved formats, messages, prompts, tests.
- [ ] Dependencies: category from [DEEPENING.md](DEEPENING.md) for each one.
- [ ] Shape: interface of each module after change. What is behind each seam?
- [ ] Tree: target tree, with one line of purpose for each file. Make sure AUDIT gives zero defects for it.
- [ ] Names: each new name is word that code or `CONTEXT.md` uses. No invented word.
- [ ] Tests: which tests stay, which move with code, which go. Default: add zero tests for move.
- [ ] Hidden behavior: each item + how Proof covers it.
- [ ] Next work: each import from 1.1 is at interface after change.
- [ ] Weakest part: which part is forced by rule and not by real joint? Say so in brief.

IF kind = move, THEN write these also. Format: `rex-code-folder-management`, section MOVE.

- [ ] Move map.
- [ ] Closed list of edits that are not moves.
- [ ] Public path table, before + after.
- [ ] Step order where build is green after each step.

IF kind = redesign, THEN obey [INTERFACE-DESIGN.md](INTERFACE-DESIGN.md): 3+ different interfaces, compare, pick.

Side effects. Do each one now, not later:

| Condition | Action |
| --- | --- |
| New module name = concept not in `CONTEXT.md` | Add term to `CONTEXT.md`. IF file does not exist, THEN create it. |
| Vague term gets sharp during design | Update `CONTEXT.md`. |
| Candidate rejected for load-bearing reason that next explorer needs | Write ADR in `docs/adr/`. Skip reasons that are obvious or short-lived. |
| Stuck on trade-off | Second opinion. Not user. |

Format for `CONTEXT.md` term: `**Term** — one-line meaning. _Avoid_: words not to use.`
Format for ADR: title, context, decision, consequences. Max 20 lines.

## 4. Brief

Direct invocation only. This step edits no code. It ends with brief for builder.

### 4.1 Proof before brief

Hold Proof outside repo, in scratchpad. Put its path in brief.

- [ ] Snapshot each output that must not change. Example: generated schema, prompt, saved file.
- [ ] Copy of source with comments removed, for code-only diff.
- [ ] List of each warning comment + `SMELL:` comment, word for word. IF `rex` is on PATH, THEN `rex smells` lists each `SMELL:` comment.
- [ ] Baseline from 1.2.

### 4.2 Write brief

Give brief to builder. No confirm step.

Brief is for agents with no context. Write it in full sentences. It must hold:

- [ ] What code does, in 3 lines. What task is. Owner's words, quoted.
- [ ] Files in scope. Current friction.
- [ ] Target tree + target interface. Seam placement. Dependency category.
- [ ] Hard constraints, numbered. Include each rule from `rex-code-folder-management` that applies.
- [ ] Hidden behavior, each item named, with rule: "move by cut and paste, never type again".
- [ ] Proof: path + checks from 4.3, as hard constraints. Builder runs checks after each stage.
- [ ] Tests that must stay. Tests that must never run.
- [ ] "Done means" list that a person can check line by line.

Brief must be literal. Builder must be able to compile it in head.

- [ ] Each re-export that moved item needs is written out.
- [ ] Each edit that is not move is on closed list.
- [ ] No step leaves build red.
- [ ] "Done" line is true as written. Say which public paths change.

### 4.3 Checks for builder

Put these checks in brief. Builder runs Proof after each stage.

- [ ] Build, clippy, fmt, doc, tests = green. Zero new warnings.
- [ ] Snapshots = byte-identical.
- [ ] Code-only diff = closed list + `mod` lines + `use` lines. Nothing else.
- [ ] Each flag comment is present, word for word.
- [ ] AUDIT block gives zero defects. No file > 500 lines.

IF check fails, THEN stage is not done.

CAUTION: Do not commit or push unless the user asks. The `CONTEXT.md` and ADR edits stay in the working tree.

### 4.4 Stop conditions

Stop before brief only IF:

- [ ] Build is red at baseline, OR
- [ ] Chosen candidate conflicts with ADR that is not worth opening again.

Then report + leave code as it was.

## Report

Final report to user. Lead with what user gets. Full sentences, plain words.

- [ ] Brief: full text, or its path.
- [ ] Target: tree before + target tree.
- [ ] What was found: ranked candidates, top 5.
- [ ] What was chosen + why. Assumptions, under own heading.
- [ ] Proof: path + checks that builder must run. Say what was not checked.
- [ ] What AUDIT flagged. Each `SMELL:` comment in scope.
- [ ] Candidates not taken + reason for each.
- [ ] Public paths that brief changes.
- [ ] `CONTEXT.md` + ADR edits: files changed, committed or not.

## ERGONOMICS CHECK

Each candidate must answer all. IF one answer is bad, THEN think again before you propose it.

| Question | Good answer |
| --- | --- |
| Is caller code shorter after? | Shorter |
| How many files does reader open for one concept after? | Fewer |
| Does each name match `CONTEXT.md` or word in code? | Yes |
| Is autocomplete list at interface easier to read? | Yes |
| Does error reach caller with context? | Yes |
| Can reader name jobs of crate from tree alone? | Yes |

## SIZE CHECK

- [ ] No code file > 500 lines. No exceptions.
- [ ] No folder with 1 child.
- [ ] No candidate that makes either rule fail.

## Hard rules

- No questions to user. Second opinion = subagent.
- Facts before advice. Explore agent gives facts only.
- One run = one kind: move OR redesign.
- Proof is held by you, outside repo. Brief tells builder to run it after each stage.
- Flag comments + hidden behavior never change in move.
- Lens mode stops after step 2.
- Build nothing. Edit no code. Skill ends with brief. Builder builds.

<!-- Adapted from Matt Pocock [YouTube] -->
