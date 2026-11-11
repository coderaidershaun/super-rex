---
name: rex-code-folder-management
description: >
  Hard rules for folders, files, and module layout in a Rust crate. The tree
  tells how the crate works. Covers placement, `mod.rs`, 500-line limit,
  imports, safe moves.
when_to_use: >
  Use when you create, split, or move a module, file, or folder, review a tree,
  or the user says "tidy the modules", "group modules", "folder structure",
  "where does this file go", "subfolders", "mod.rs".
disable-model-invocation: false
user-invocable: true
---

# Folder Management — Hard Rules

Other language: apply intent of each rule in idiom of that language. IF language lacks Rust feature that a rule relies on, THEN obey gap skill of that language: skill for what Rust gives by default and that language does not.

Tree = first document. Reader sees tree before code.
Tree must tell: jobs of crate, order of work, home of each concept.

Sister skills:

- `rex-code-improve-codebase-architecture` — finds what to move. This skill tells where it goes. Holds deletion test.
- `rex-code-smells` — finds smells inside files.
- `rex-code-commenting` — law for module comment: comment that opens each file.
- `rex-code-philosophy` — complexity rules.

## Files of this skill

| Read | When |
| --- | --- |
| `.claude/skills/rex-code-folder-management/audit.md` | IF you review a tree or report defects, THEN read it. AUDIT block + report format. |
| `.claude/skills/rex-code-folder-management/move.md` | IF you move files or plan a move, THEN read it before first move. Hidden behavior, procedure, proof, scope. |

## Words

| Term | Meaning |
| --- | --- |
| Module | One `.rs` file, or one folder with `mod.rs`. |
| Root file | `mod.rs` of folder. |
| Child | File or folder directly in folder. `mod.rs` is not child. |
| Top module | Module that `lib.rs` or `main.rs` declares. |
| Job | One thing crate does for its caller. Example: convert, read. |
| Shared data | Types that 2+ jobs use. Example: format saved on disk. |
| Leaf | Module that imports no other module of crate. |
| Interface | Names that root file exports. All that caller must know. |
| Seam | Place where module's interface lives: behavior changes there with no edit there. Real seam = trait with 2+ adapters. Example: live adapter + test adapter. Full words: `.claude/skills/rex-code-improve-codebase-architecture/LANGUAGE.md`. |
| Joint | Place where file divides clean: one unit, one caller, small interface. |
| Move | Change of path with no change of code. |
| Hidden behavior | Behavior that depends on path or comment. Compiler does not see change. |

## RULES: tree

- [ ] One top module for each job. Shared data = one more top module. Target: 2 to 5 top modules.
- [ ] Top module name = job name. Reader lists `src/` and knows what crate does.
- [ ] Module with children = folder with `mod.rs`. Never `name.rs` beside `name/`.
- [ ] Module with no children = one file `name.rs`. Never folder with only `mod.rs`.
- [ ] Folder holds `mod.rs` + min 2 children. Folder with 1 child = defect. Apply ONE-CHILD FIX.
- [ ] Max 3 folder levels below `src/`.
- [ ] Max 12 children in one folder. IF more, THEN group children that have one caller.
- [ ] Max 500 lines in one code file. No exceptions. IF file > 400 lines, THEN find joint now.
- [ ] Root file holds: module comment, `mod` lines, re-exports, entry items of module. Nothing else.
- [ ] Asset (prompt, fixture, schema) lives beside file that loads it. 2+ assets = own subfolder.
- [ ] List of children reads as steps of job.
- [ ] One order for `mod` lines + `use` lines in whole crate.

## RULES: placement

| Fact about module | Home |
| --- | --- |
| One module uses it | Folder of that module |
| 2+ children of one folder use it | That folder |
| Type goes to disk, wire, or other crate | Shared data module, with all other saved types |
| 2+ top modules use it + it is not data | Lowest common parent |
| Code is behind one seam | Folder of seam. All of it. |
| Wrapper for outside tool or service | Folder of one job that uses it |
| Rule or number that one job uses | Folder of that job |

- [ ] IF rule home makes cycle, THEN put rule with its type + add `SMELL:` comment.
- [ ] IF two table rows apply, THEN first row that applies wins.
- [ ] IF no row applies, THEN module has no clear owner. Use `rex-code-improve-codebase-architecture`.

## RULES: names

- [ ] File name = one domain word for concept that file holds.
- [ ] `mod foo;` per concept, not per type. `auth/`, not `auth_user.rs` + `auth_token.rs` + `auth_session.rs`.
- [ ] Take word from code: type name, function name, error name. Do not invent new word.
- [ ] One concept = one name in whole tree.
- [ ] Types file = noun (`reply`). Calls file = verb (`transcribe`).
- [ ] Forbidden names: `utils`, `helpers`, `common`, `misc`, `shared`, `stuff`, `manager`.
- [ ] Name must not promise more than file holds.
- [ ] IF file holds part of decision, THEN module comment tells which part, in plain words. No path to other file: tree shows where rest lives (`rex-code-commenting`).
- [ ] Each file starts with module comment, 1 to 2 lines: why file exists. Form for each language: `.claude/skills/rex-code-commenting/module-comments.md` (`//!` in Rust). Root file of package or program can add priority block after it (`rex-code-priorities`).
- [ ] After each move, make sure each module comment is true.

## RULES: imports + visibility

- [ ] Imports go one direction: job to shared data, parent to child.
- [ ] Shared data module = leaf. It never imports job module.
- [ ] No cycle between siblings.
- [ ] Children of folder = private. Root file re-exports interface.
- [ ] Code outside folder imports from root file only. It never names child.
- [ ] Path style: `super::` for parent or sibling in same folder. `crate::` for all else.
- [ ] Visibility: narrowest that compiles. Order: private, `pub(super)`, `pub(crate)`, `pub`.
- [ ] Crate root re-exports all that caller needs for main jobs.
- [ ] Stand-in for seam imports from named sub-path. Tell this in module comment of crate root.
- [ ] No re-export that has no user. No `#[allow(unused)]` to hide one.

## ONE-CHILD FIX

```
IF parent + child <= 500 lines AND one topic, THEN merge child into parent. Parent = one file.
ELSE IF parent holds second joint, THEN extract that joint as second child.
ELSE move child up as sibling of parent. Parent = one file.
```

- [ ] Never invent child with no joint to obey rule.
- [ ] Apply deletion test to new child. IF complexity vanishes, THEN child is pass-through. Merge it.
- [ ] Merge, extract, or move up = move. Before you start, read `.claude/skills/rex-code-folder-management/move.md`.

## SPLIT

File near limit. Joints, best first:

1. Types + calls that make them. Types go one file, calls go other file.
2. Decision table + its test.
3. Unit with one caller behind one function.
4. I/O + pure compute.

- [ ] Inline test moves with code that it tests. Same name, same body.
- [ ] IF file has no joint, THEN problem = design, not size. Use `rex-code-improve-codebase-architecture`.
- [ ] Split = move. Before you start, read `.claude/skills/rex-code-folder-management/move.md`.

## Ergonomics check

- [ ] Can reader name jobs of crate from `ls src/`?
- [ ] How many files must reader open to trace one job? Fewer = better.
- [ ] Can new person place new file with placement table and no question?
- [ ] Does file name match word that `grep` user types?
- [ ] Does each folder have one reason to change?

IF one answer is no, THEN tree is not done.
