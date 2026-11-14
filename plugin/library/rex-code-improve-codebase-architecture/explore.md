# 1. Explore

Words, rules + stop conditions: `rex-code-improve-codebase-architecture/SKILL.md`.

Facts first. No advice in this step.

## 1.1 Read

- [ ] Priority block: obey READ + USE in `rex-code-priorities`. IF block does not exist, THEN continue. Stop conditions in `SKILL.md` say when to stop.
- [ ] Document that invocation names: spec, plan, goal. List what next work imports from this code.
- [ ] `CODEBASE.md`, IF it exists. ELSE IF `rex` is on PATH, THEN run `rex codebase --only code --with-context`.
- [ ] Entry files: Rust `lib.rs`, `main.rs`, `Cargo.toml`. Other language: its entry file + manifest (`go.mod`, `pyproject.toml`, `package.json`).

## 1.2 Baseline

- [ ] Record `git rev-parse HEAD` + `git status --short`.
- [ ] Run build, clippy, fmt check, doc, tests. Record each result.
- [ ] IF build is red, THEN stop. Report. Change nothing.
- [ ] Never run ignored tests or tests that spend money.

## 1.3 Map

Run AUDIT block of `rex-code-folder-management` (`rex-code-folder-management/audit.md`). It gives: file sizes, children of each folder, module graph, forbidden names, `pub` items that no caller reaches.

Then list each item declaration for each file:

```bash
grep -rnE '^\s*(pub(\([a-z ]+\))? )?(async )?(fn|struct|enum|trait|type|const|static|impl) ' src
```

## 1.4 Explore agent

Dispatch one agent (`subagent_type=Explore`). Ask for facts with file:line, in tables. Do not ask for advice. Do not give your hypothesis.

Questions to ask:

1. Which public items does code outside the library use, and through which paths? Which public items have no outside user?
2. Which items does a module other than the definer use? Give definer and users. List each cycle.
3. Which types are saved to disk or sent out, and where do they live?
4. Which files hold two topics? Give line ranges for each topic.
5. What is dead: template code, items with no caller, items that only their own test uses?
6. What hidden behavior exists? See the table in `rex-code-folder-management`, section MOVE (`rex-code-folder-management/move.md`).
7. Which inline tests exist, and which private items does each one use?
8. Which state lives longer than one call? Give holder + each module that writes it.
9. Where does code wait, lock, spawn, or cross thread, process, or network?
10. Which files did the commit that added the last unit of growth change? (Skip when growth = "none".)

CAUTION: Do not edit files while the Explore agent reads them. Wait for its report.

## 1.5 Shape

Facts only. Use answers 8-10 of 1.4.

- [ ] Pattern: name arrangement that code uses now, one line (term in Words, `SKILL.md`). ELSE "none".
- [ ] Growth: copy from priority line or 1.1. ELSE "none".
- [ ] Growth test: count modules that need an edit for one more unit of growth. 1 = pass.
- [ ] Main path: list its modules.

## 1.6 Friction signals

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
- [ ] No pattern name fits, or module goes against pattern of the rest.
- [ ] Growth test fails.
- [ ] State has writers in 2+ modules that are not children of one folder.
- [ ] Bandage, and no warning comment tells why.
- [ ] Wait, lock, or crossing that interface + name do not tell.
