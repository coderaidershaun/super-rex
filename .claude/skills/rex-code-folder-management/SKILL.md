---
name: rex-code-folder-management
description: >
  Hard rules for folders, files, and module layout in a Rust crate. The tree
  must tell how the crate works before a reader opens a file. The skill covers
  top modules, where a module lives, `mod.rs`, the minimum of two children for
  each folder, the 500-line limit, import direction, visibility, and how to
  move files with no behavior change. Use this skill when you create a module
  or a folder, split or move a file, or review a tree. Use it also when the
  user says "tidy the modules", "group modules", "folder structure", "where
  does this file go", "subfolders", or "mod.rs". The
  rex-code-improve-codebase-architecture skill loads this skill in every run.
disable-model-invocation: false
user-invocable: true
---

# Folder Management — Hard Rules

Tree = first document. Reader sees tree before code.
Tree must tell: jobs of crate, order of work, home of each concept.

Sister skills:

- `rex-code-improve-codebase-architecture` — finds what to move. This skill tells where it goes.
- `rex-code-smells` — finds smells inside files.
- `rex-code-commenting` — law for `//!` lines.
- `rex-code-philosophy` — deletion test, complexity budget.

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
| Seam | Trait with 2+ adapters. Example: live adapter + test adapter. |
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
- [ ] Root file holds: `//!` line, `mod` lines, re-exports, entry items of module. Nothing else.
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
- [ ] Take word from code: type name, function name, error name. Do not invent new word.
- [ ] One concept = one name in whole tree.
- [ ] Types file = noun (`reply`). Calls file = verb (`transcribe`).
- [ ] Forbidden names: `utils`, `helpers`, `common`, `misc`, `shared`, `stuff`, `manager`.
- [ ] Name must not promise more than file holds.
- [ ] IF file holds part of decision, THEN `//!` tells which part + where rest lives in plain words.
- [ ] Each module starts with `//!`, 1 to 2 lines: why module exists.
- [ ] After each move, make sure each `//!` is true.

## RULES: imports + visibility

- [ ] Imports go one direction: job to shared data, parent to child.
- [ ] Shared data module = leaf. It never imports job module.
- [ ] No cycle between siblings.
- [ ] Children of folder = private. Root file re-exports interface.
- [ ] Code outside folder imports from root file only. It never names child.
- [ ] Path style: `super::` for parent or sibling in same folder. `crate::` for all else.
- [ ] Visibility: narrowest that compiles. Order: private, `pub(super)`, `pub(crate)`, `pub`.
- [ ] Crate root re-exports all that caller needs for main jobs.
- [ ] Stand-in for seam imports from named sub-path. Tell this in crate `//!`.
- [ ] No re-export that has no user. No `#[allow(unused)]` to hide one.

## ONE-CHILD FIX

```
IF parent + child <= 500 lines AND one topic, THEN merge child into parent. Parent = one file.
ELSE IF parent holds second joint, THEN extract that joint as second child.
ELSE move child up as sibling of parent. Parent = one file.
```

- [ ] Never invent child with no joint to obey rule.
- [ ] Apply deletion test to new child. IF complexity vanishes, THEN child is pass-through. Merge it.

## SPLIT

File near limit. Joints, best first:

1. Types + calls that make them. Types go one file, calls go other file.
2. Decision table + its test.
3. Unit with one caller behind one function.
4. I/O + pure compute.

- [ ] Inline test moves with code that it tests. Same name, same body.
- [ ] IF file has no joint, THEN problem = design, not size. Use `rex-code-improve-codebase-architecture`.

## AUDIT

Run from crate folder. Each hit = candidate. Read hit before you report it.

```bash
# size: files near 500-line limit
find src tests -name '*.rs' | xargs wc -l | sort -rn | head -8

# children: folder must hold mod.rs + min 2 children (asset folders are skipped)
for d in $(find src -mindepth 1 -type d); do
  find "$d" -name '*.rs' | grep -q . || continue
  files=$(find "$d" -maxdepth 1 -name '*.rs' ! -name mod.rs | wc -l)
  dirs=$(find "$d" -mindepth 1 -maxdepth 1 -type d -exec test -e {}/mod.rs \; -print | wc -l)
  echo "$((files + dirs)) children  $(test -e "$d/mod.rs" && echo mod.rs || echo NO-mod.rs)  $d"
done

# name.rs beside name/ : forbidden
for d in $(find src -mindepth 1 -type d); do test -e "$d.rs" && echo "FORBIDDEN $d.rs + $d/"; done

# module graph: who imports who
grep -rnE '^\s*(pub(\([a-z]+\))? )?mod |^\s*(pub(\([a-z]+\))? )?use (crate|super)' src | sort

# forbidden names
find src -name 'utils*' -o -name 'helpers*' -o -name 'common*' -o -name 'misc*' -o -name 'shared*'

# pub item that no code outside crate can reach (0 = good)
cargo rustc --lib -- -W unreachable_pub 2>&1 | grep -c unreachable
```

From module graph, find:

- [ ] Cycle between siblings.
- [ ] Shared data module that imports job module.
- [ ] Code that names child of other folder.
- [ ] Module with one user that lives outside folder of that user.
- [ ] `crate::` path where `super::` is correct, or reverse.

## MOVE

CAUTION: Do not type code again during a move. Cut and paste the exact text. A changed attribute or doc comment can change behavior, and the compiler does not report it.

CAUTION: Find hidden behavior before the first move. A move that breaks hidden behavior passes the build and the tests.

Hidden behavior to find:

| Item | Why move can break it |
| --- | --- |
| `include_str!`, `include_bytes!` | Path is relative to file that holds macro. |
| `env!("CARGO_MANIFEST_DIR")` joins | Path names folder. |
| `///` on type with derive macro (`JsonSchema`, `Parser`) | Macro reads doc comment as data. |
| `#[path]`, `build.rs`, `mod` in macro | Path names file. |
| Module path in string | Test filter, log target, `module_path!`, `type_name`. |
| Path in docs, hooks, CI config | Text goes stale. |

Procedure:

1. Record baseline: `git rev-parse HEAD`, `git status --short`, build, clippy, fmt, doc, tests.
2. Snapshot each output that must not change. Example: generated schema, prompt file, saved file.
3. Write move map. One row for each file: old path, new path, what changes inside.
4. Write closed list: each edit that is not move. Kinds: visibility, body, deleted, added.
5. Write public path table: path before, path after. Each public item stays public.
6. Order steps so build is green after each step.
7. Each step includes `mod`, `use`, and test-import edits that it forces.
8. Use `git mv` for each file, so that history stays with the file.
9. `git mv` stages the change. Use `git diff HEAD -M` to see the work. Do not commit unless the user asks.
10. DO build + test UNTIL green, after each step. IF 3 rounds fail, THEN stop + report.

Proof of no behavior change. All must hold:

- [ ] Snapshots = byte-identical to baseline.
- [ ] Code with comments removed differs only by closed list + `mod` lines + `use` lines.
- [ ] Each warning comment + `SMELL:` comment is present, word for word.
- [ ] Tests: same names, same bodies. Only import lines differ.
- [ ] Build, clippy, fmt, doc, tests = green. Zero new warnings.
- [ ] AUDIT block gives zero defects.

Scope:

- [ ] One run = one kind of change. Move OR redesign. Never both.
- [ ] IF move shows design smell, THEN add `SMELL:` comment + report it. Do not correct it in move run.
- [ ] Parallel agents: one file group for each agent. No formatter until all agents are done.

CAUTION: Do not let a parallel agent run `git stash`, `git checkout`, `git restore`, or `git reset`. These commands delete the work of the other agents.

## Report

| Where | Rule broken | Evidence | Fix |
| --- | --- | --- | --- |
| `src/figure/` | 1 child | holds only `refine.rs` | extract second joint `sheet` |

- [ ] Give tree before + tree after.
- [ ] Give count of children for each folder.
- [ ] Give largest file + its line count.
- [ ] "No defects" = valid result. Do not invent defects.

## Ergonomics check

- [ ] Can reader name jobs of crate from `ls src/`?
- [ ] How many files must reader open to trace one job? Fewer = better.
- [ ] Can new person place new file with placement table and no question?
- [ ] Does file name match word that `grep` user types?
- [ ] Does each folder have one reason to change?

IF one answer is no, THEN tree is not done.
