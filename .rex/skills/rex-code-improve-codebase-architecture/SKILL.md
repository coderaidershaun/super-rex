---
name: rex-code-improve-codebase-architecture
description: >
  Finds architecture friction, ranks candidates, designs the best: deep
  modules, a tree that tells how code works. No questions, builds nothing,
  ends with a design brief.
when_to_use: >
  Use when user wants refactor candidates, module grouping, or code easier to
  test and navigate, or says "improve architecture", "refactor opportunities",
  "deepen modules", "tidy up modules", "get ready for the next feature".
---

# Improve Codebase Architecture

Find friction. Rank candidates. Design best candidate. Give brief to builder. Brief tells how to prove behavior did not change.
This skill builds nothing. It edits no code.
Aim: code that is easy to test, easy to navigate, easy to call, easy to change. Never at cost of a higher quality in priority line.

Steps: 1 Explore, 2 Rank, 3 Grill self, 4 Brief. Then Report.

## Words

Use these exact terms in each suggestion.

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
| Pattern | Named arrangement of modules. Example: pipeline, one owner of state, state machine. Full rules: `.rex/skills/rex-code-design-sketch/SKILL.md`. |
| Growth | What there will be more of. Source: brackets in priority line, or next work. ELSE "none". |
| Main path | Modules that first quality of priority line depends on. Latency = hot path. |
| Bandage | Branch or module that only hides fault in other module. |

Forbidden words: "component", "service", "API", "boundary". Say module, interface, seam.

Three tests:

- [ ] Deletion test: delete module. IF complexity vanishes, THEN module = pass-through. IF complexity comes back in N callers, THEN module earns its place.
- [ ] Interface = test surface. IF test must go past interface, THEN module has wrong shape.
- [ ] One adapter = hypothetical seam. Two adapters = real seam. No new seam for one adapter.

## Skills + files that this skill loads

| When | Load |
| --- | --- |
| Each run, before step 1 | `.rex/skills/rex-code-folder-management/SKILL.md` — tree rules, placement table |
| Each run, before step 1 | `.rex/skills/rex-code-priorities/SKILL.md` — priority block, READ, USE |
| Step 1, each run | `.rex/skills/rex-code-improve-codebase-architecture/explore.md` |
| Step 1.3, each run | `.rex/skills/rex-code-folder-management/audit.md` — AUDIT |
| Step 1.4, each run | `.rex/skills/rex-code-folder-management/move.md` — MOVE, hidden behavior |
| Step 2, each run | `.rex/skills/rex-code-improve-codebase-architecture/rank.md` |
| Step 2, to judge names + call sites | `.rex/skills/rex-code-ergonomics/SKILL.md` |
| Step 3, direct invocation only | `.rex/skills/rex-code-improve-codebase-architecture/grill-self.md` |
| Step 3, dependencies of candidate | `.rex/skills/rex-code-improve-codebase-architecture/DEEPENING.md` |
| Step 3, IF candidate changes interface | `.rex/skills/rex-code-improve-codebase-architecture/INTERFACE-DESIGN.md` |
| Step 4, direct invocation only | `.rex/skills/rex-code-improve-codebase-architecture/brief.md` |
| IF you need full definition of a term in Words | `.rex/skills/rex-code-improve-codebase-architecture/LANGUAGE.md` |

Domain language names good seams. Names in code hold domain terms. Priority block holds ranking + Never lines. Warning comments + fitness functions hold decisions that this skill must not open again without cause.

## RULES: no questions

Run alone from start to end. Invocation = permission.

- [ ] Ask user nothing. Not before start, not during work, not before brief.
- [ ] No scope question. IF invocation names path, THEN scope = that path. ELSE scope = whole repo, minus vendored + generated code.
- [ ] No permission question. No "shall I proceed".
- [ ] No choice question. Break ties yourself.
- [ ] IF fact is unknown, THEN pick sane assumption, write it in report, continue.
- [ ] Never assume growth or ranking. IF no source names growth, THEN growth = "none".
- [ ] Findings, decisions, risks go in final report. Never in question.
- [ ] IF user sends message during run, THEN message = hard constraint. Add it to brief. Continue. Do not ask user to confirm.
- [ ] IF decision is hard, or tie, or load-bearing, THEN get second opinion.

Second opinion: dispatch second subagent. Give it file paths, short list, trade-off. It gives recommendation + reasons. You decide. Second opinion = always subagent, never user.

## Lens mode

IF other skill or agent loads this file as review lens, THEN:

- [ ] Do step 1 + step 2 only.
- [ ] Report findings. Change nothing. Write no brief.

Only direct invocation runs step 3 + step 4.

## Stop conditions

Stop before brief only IF:

- [ ] Build is red at baseline, OR
- [ ] No priority block AND choice of candidate depends on ranking (READ in `rex-code-priorities`), OR
- [ ] Chosen candidate conflicts with warning comment or fitness function that is not worth opening again.

Then report + leave code as it was.

## Report

Final report to user. Lead with what user gets. Full sentences, plain words.

- [ ] Brief: full text, or its path.
- [ ] Target: tree before + target tree.
- [ ] What was found: ranked candidates, top 5.
- [ ] What was chosen + why. Assumptions, under own heading.
- [ ] Proof: path + checks that builder must run. Say what no check covers.
- [ ] What AUDIT flagged. Each `SMELL:` comment in scope.
- [ ] Candidates not taken + reason for each.
- [ ] Public paths that brief changes.

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
