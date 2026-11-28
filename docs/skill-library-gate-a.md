# Skill library gate A: the evidence

This is the evidence for gate A of the skill library spec (section 12). It was measured at base commit `75566e3` (`75566e35d9aa83c2ae166ae1906a83a3b33b8dae`) with the binary built from that commit. Neither bar is met. In the primary blind run, the right skill is in the top five for 54 of 89 needs (60.7%), and the bar is 70%. In the answers of that run, lookup B made 5 assignments that the judge rules wrong, and the bar allows none.

You decide whether the mod is changed now, or whether the design, or the defects below, are revisited first. This document gives the evidence and no verdict.

Four findings explain most of the shortfall. For each, this document names the request (a form file under `tests/fixtures/gate-a/`) that shows it, and nothing was repaired:

1. The coding digest `rex-code-essentials` is never in the top five of a code task. In run 1 it is at rank 13 to 17 on all 13 code items that need it. That is 13 of the 35 misses.
2. Keyword search (lookup C) put a right skill in the top five once among 89 needs. Needs that expect lookup C were met mostly by lookup B.
3. The wrong assignments come from vocabulary terms that are broader than the skills that claim them, and from code skills that have no `writes_code` scope.
4. The page budget of 1,200 tokens holds 7 or 8 items, not 10, and on every 10-item page it leaves out all files and open-when items to get there.

## How the numbers were made

| Input | Value |
| --- | --- |
| Binary | `target/debug/rex` built from `75566e3`. This job changes no file under `src/` or `plugin/`: after `main` was merged into the job branch, `git diff --stat origin/main -- src plugin` is empty |
| Library | `plugin/library`, 25 skills, vocabulary version 1 |
| Filler pool | TerminalSkills/skills at commit `42289059901710441bfa8081c08c3a9f36b93f9c`, 1,146 skills, outside the repo at `/Users/shaun/Code/DEVELOPMENT/gate-a-pool/terminalskills/skills` |
| Index version of every answer | `sha256:2a06b957e9467fddf488f6cd0e816e7f345909b49017ec8a8f21a2066bc7a13c` |
| Items | 23, in `tests/fixtures/gate-a/tasks.json`: every WORK BREAKDOWN row of 7 recorded jobs (17 task rows and 6 reviewer rows). Job `0ganyb6d6v`, whose tasks the earlier tests of spec 11.1 used, is left out. |
| Forms | `tests/fixtures/gate-a/forms/run-1/` (the primary blind run), `forms/run-2/` and `reference/`, 23 each, copied unchanged from the sealed folders |
| Labels | `tests/fixtures/gate-a/labels.json`, by the judge |
| Verdicts | `tests/fixtures/gate-a/judgments.json`, by the judge |
| Results | `tests/fixtures/gate-a/results/`: one name-only answer per item and run, and the output of every script named below |
| Order of commits on `rex/0h0vht64cg` | skill labels `c1fd0ae`, forms `afed734`, need labels `d3b8b79`, search results `5c1b22a`, verdicts `de50e8d`, scores `c3cc2d0`, then this document |

All commands run from the root of the job worktree. In this document `results/` means `tests/fixtures/gate-a/results/`, and `<run>` is one of `run-1`, `run-2` or `reference`. Each script prints numbered lines, such as `k1.2`, and a source given beside a number names the file and the line that holds it. Full answers hold pool text, so they stay outside the repo, in `/Users/shaun/Code/DEVELOPMENT/gate-a-pool/raw/<run>/`.

| Step | Command | Output |
| --- | --- | --- |
| Fill repo facts | `node tests/fixtures/gate-a/fill-facts.ts --forms <form folder> --out /Users/shaun/Code/DEVELOPMENT/gate-a-pool/filled/<run>` | 784 tracked paths and 14 dependencies per form |
| Search | `node tests/fixtures/gate-a/run-search.ts --forms /Users/shaun/Code/DEVELOPMENT/gate-a-pool/filled/<run> --out tests/fixtures/gate-a/results/<run> --label <run> --all-pages yes` | `results/<run>/`, `results/run-search-<run>.txt` |
| Per-run labels | `jq --arg r run1 '.items \|= map(.needs \|= map(select(.run == $r)))' tests/fixtures/gate-a/labels.json` (and `run2`); the reference gets the skill labels only | `results/labels/` |
| Score | `node tests/fixtures/gate-a/score-hits.ts --labels tests/fixtures/gate-a/results/labels/labels-run1.json --results tests/fixtures/gate-a/results/run-1 --judgments tests/fixtures/gate-a/judgments.json` | `results/score-run-1.txt` (and `-run-2`, `-reference`) |
| Forms | `node tests/fixtures/gate-a/compare-forms.ts --run tests/fixtures/gate-a/forms/run-1 --run tests/fixtures/gate-a/forms/run-2 --reference tests/fixtures/gate-a/reference` | `results/compare-forms.txt` |
| Repeat, pool, size | `node tests/fixtures/gate-a/repeat-check.ts --forms <filled folder>`, then `pool-invariance.ts` and `answer-size.ts` with the same flag | `results/repeat-<run>.txt`, `results/pool-invariance-<run>.txt`, `results/answer-size-<run>.txt` |
| Parts 3, 4 and 5a | `node tests/fixtures/gate-a/steps-e-h.ts` | `tests/fixtures/gate-a/steps-e-h-output.txt` |

A need is either an item paired with one library skill that its label names, or one free-text need of a form that has a right skill. A need is hit when its skill is at rank 1 to 5 of the item's first page, counted over all tiers. A free-text need with no right skill is counted apart and never scored.

## 1. The blind run

**Result.** In run 1, the right skill is in the top five for 54 of 89 needs (60.7%). By the lookup that the label expects, the rates are 20.0% for lookup A, 90.2% for lookup B and 46.4% for lookup C.

### The labels

The judge wrote the skill labels first, from `tasks.json`, the spec, the gate A README and the library headers only. They were committed alone, as `c1fd0ae`, before any form or result was read. They hold 68 library skill labels on 18 items: 20 expect lookup A, 41 lookup B and 7 lookup C. Five items have no library skill. Three are reviewers whose method is `rex-pr-review`, a listed skill outside `plugin/library`. Two write no code: a findings file, and the vocabulary files with their mapping document.

The free-text needs were labelled after the forms were read and before any search was run on them (commit `d3b8b79`). Run 1 has 56 needs, and 21 of them have a right skill. Run 2 has 50 needs, and 25 have a right skill. A pool skill is named only when its folder name is the technology that the need names, such as `sqlite` or `claude-code`, because only the scripts may read the pool's text.

Two rules of the labels change the numbers, so they are stated here:

- `rex-code-essentials` is the one label for the coding rules of a code task. The four full rulebooks (`rex-code-philosophy`, `rex-code-commenting`, `rex-code-ergonomics` and `rex-code-error-writing`) are its open-when items under D18, so they are not labelled again. With the four rulebooks counted as labels too, run 1 falls to 67 of 141 (47.5%), because only `rex-code-commenting` of the four reaches the top five (`results/score-run-1-with-rulebooks.txt`).
- `rex-code-tests-integration-testing` was labelled only for tests that reach an outside program or database. Its header says "the real outside world"; its body also counts a test of the binary against the real file system. So the labels leave it out on 6 items where the judge later rules it right.

### Hit rates

| Measure (top five) | Run 1 | Run 2 | Reference forms |
| --- | --- | --- | --- |
| All needs | 54 of 89 (60.7%) | 53 of 93 (57.0%) | 41 of 68 (60.3%) |
| Library skill labels | 43 of 68 (63.2%) | 39 of 68 (57.4%) | 41 of 68 (60.3%) |
| Free-text needs | 11 of 21 (52.4%) | 14 of 25 (56.0%) | not labelled |
| Expected lookup A | 4 of 20 (20.0%) | 5 of 20 (25.0%) | 5 of 20 (25.0%) |
| Expected lookup B | 37 of 41 (90.2%) | 32 of 41 (78.0%) | 33 of 41 (80.5%) |
| Expected lookup C | 13 of 28 (46.4%) | 16 of 32 (50.0%) | 3 of 7 (42.9%) |
| Hits found by tier A, B, C | 5, 48, 1 | 6, 46, 1 | 5, 35, 1 |
| Without the 3 items whose text quotes a library header | 45 of 71 (63.4%) | 44 of 75 (58.7%) | 33 of 56 (58.9%) |

Source: lines `k1.2` to `k1.5` and `k2.2` of `results/score-<run>.txt`. The reference forms are scored on the 68 skill labels only, because their free-text needs were not labelled. The three items that quote a header, listed in `tests/fixtures/gate-a/header-quotes.json`, are `0h0962klnd-T2` and `0h0f8vnww4-T2` (both quote `sub-agent`) and `0h0d1109wo-T1` (it quotes `coding-standards`).

Lookup C did little. In run 1, 13 of the 28 needs that expect lookup C were hit, but 11 of those hits came through a tier B item, 1 through a tier A item and 1 through a tier C item. The one tier C hit is `rex-skill-language` for the reviewer `0h0d1109wo-reviewer`. No pool skill that a label names was in any top five.

### Needs with no hit in run 1

There are 35 (line `k1.6` of `results/score-run-1.txt`). Run 2 has 40: the same 13 misses of the digest, 2 rulebooks of `0h0d1109wo-T1`, 9 skills that expect lookup B, the same 5 reviewer skills and 11 free-text needs. The reference forms have 27: the 13 digest misses, 2 rulebooks, 8 skills that expect lookup B and 4 reviewer skills. Their lists are in `results/score-run-2.txt` and `results/score-reference.txt`.

| Group | Needs |
| --- | --- |
| The digest, expected A (13) | `rex-code-essentials` on every code item: `0h0962klnd-T1` rank 16, `0h0962klnd-T2` 17, `0h096dyeq9-T2` 15, `0h0b8x725g-T1` 17, `0h0dnqm24i-T1` 15, `-T2` 16, `-T3` 17, `-T4` 17, `-T5` 14, `-T6` 14, `0h0f8vnww4-T1` 16, `-T2` 15, `-T3` 13 |
| Rulebooks as digest sources, expected A (3) | `0h0d1109wo-T1`: `rex-code-ergonomics` rank 6, `rex-code-error-writing` 9, `rex-code-philosophy` 15 |
| Expected B (4) | `0h096dyeq9-T2` `rex-code-folder-management` rank 6; `0h0dnqm24i-T1` `rex-code-validation-plan` 9; `0h0f8vnww4-T1` `rex-code-failure-policy` 6 and `rex-code-folder-management` 7 |
| Reviewer skills, expected C (5) | `0h0d1109wo-reviewer`: the four rulebooks, on no page; `0h0fitv57s-reviewer`: `rex-skill-language`, on no page |
| Free-text needs (10) | "SQLite FTS5 and bm25 ranking in bundled rusqlite" (`sqlite` at rank 14); "stub stdio MCP server" (`mcp-server-builder` at 11); "plugin manifest key for MCP servers", "headless claude session with allowed and disallowed tools", "hand-written MCP JSON-RPC server over stdio", "SQLite FTS5 query with weighted bm25", "escape caller text in FTS5 queries", "drive a headless claude session and quote its init line" and "check each rule against the code" (each on no page); "refusal text that names the tool to use" (`rex-code-error-writing` at 12) |

### The digest defect

The request `tests/fixtures/gate-a/forms/run-1/0h0dnqm24i-T2.json` shows it. Its answer starts with `rex-code-commenting` at rank 1, then `rex-code-tests-fitness-functions`, `rex-code-tests-contract-seams`, `rex-code-tdd` and `rex-code-failure-policy`. `rex-code-essentials` is at rank 16, the last item of the second page. Three causes act together:

- The four rulebooks still claim `facts: { writes_code: true }`, the same claim as the digest. So lookup A returns five rule skills with placement `full` (each to be pasted whole into the brief) for every code task, although D18 makes the four rulebooks open-when items of the digest.
- The five are tied, and a tie is broken by name. So the order is commenting, ergonomics, error-writing, essentials, philosophy, and the digest comes fourth.
- The page is built in rounds, and each round gives each need of the form its next item. `writes_code` is one need with five skills, so its five rule skills come one per round. In this answer, round 1 holds ranks 1 to 7: one rule skill for `writes_code`, the five tier B items and one tier C item. Each later round holds 4 items: the next rule skill and one tier C item for each of the three free-text needs, mostly pool skills. So the rule skills are at ranks 1, 8, 12, 16 and 20, and the digest, fourth by name, is at rank 16.

### Assignments of lookups A and B judged wrong or doubtful

The three answers hold 153 distinct pairs of item and skill at tier A or B, over all pages. The judge ruled 72 right, 70 doubtful and 11 wrong. By tier, A has 20 right and 53 doubtful; B has 52 right, 17 doubtful and 11 wrong. In each answer separately (line `k3.2`): run 1 has 5 wrong B items, run 2 has 9 and the reference forms have 5. A pair can be wrong in more than one answer, so these make 11 distinct pairs.

The verdicts mean this. Right: the skill covers work that the item asks for. Doubtful: it covers that work only in part, or it comes in the wrong form, such as a duplicate or placement `full` where D18 says open-when. Wrong: neither its header nor its body covers any work that the item asks for.

**Wrong (11).**

| Item | Skill | In | Reason |
| --- | --- | --- | --- |
| `0h096dyeq9-T1` | `rex-code-design-sketch` | all three | All three forms ticked activity `design` for a vocabulary and a mapping document. The sketch is a module table and a flow diagram of code, and the item has no module. |
| `0h096dyeq9-T1` | `rex-code-validation-plan` | all three | Expected traces of flows and failures; a vocabulary has no flow. |
| `0h0962klnd-T3` | `rex-code-spike` | run 1, reference | The item writes the findings file; nothing is left unknown, and the spike's own rule says no spike then. |
| `0h096dyeq9-T2` | `rex-code-smells` | run 2, reference | Found by `legacy-code`. The skill only audits and writes a findings report; the item builds new rules. |
| `0h0dnqm24i-T6` | `rex-code-smells` | run 2, reference | The same, for the intent check. |
| `0h0d1109wo-T1` | `rex-code-folder-management` | runs 1 and 2 | Found by `file-layout` for the new skill folder, which holds skill text, not code. |
| `0h0f8vnww4-T2` | `rex-agent-prompting` | run 1 | Run 1 ticked `sub-agent`; the hook refuses tools for agents but starts none and writes no brief. |
| `0h0962klnd-T1`, `0h0962klnd-T2` | `rex-code-design-sketch` and `rex-code-validation-plan` (4 pairs) | run 2 | Run 2 ticked activity `design` for two proof tasks, which design no module and have no flow to trace. |

The causes are of three kinds, and each is reported, not repaired:

- **Vocabulary terms broader than their skills.** `design` ("Designs a system, feature or module before its code") brings the code design sketch to a vocabulary task. `legacy-code` ("Changes code that is old or that the agent does not know") fits any edit of existing code, but `rex-code-smells` is an audit. `file-layout` ("Creates, splits or moves a file, module or folder") fits a skill folder, but `rex-code-folder-management` covers the module tree of code. Requests: `0h096dyeq9-T1.json` in `forms/run-1/`, `forms/run-2/` and `reference/`; `0h096dyeq9-T2.json` in `forms/run-2/` and `reference/`; `forms/run-1/0h0d1109wo-T1.json` (all under `tests/fixtures/gate-a/`).
- **Headers with no `writes_code` scope.** `rex-code-spike`, `rex-code-design-sketch` and `rex-code-validation-plan` have no scope, so tasks that write no code get them. Requests: `forms/run-1/0h0962klnd-T3.json` and the three forms of `0h096dyeq9-T1`, all with `writes_code: false`.
- **Form choices.** Run 2 chose activity `design` for the two WP0 proof tasks, and run 1 ticked `sub-agent` for a hook. These are errors of the form, not of search.

**Doubtful, tier A (53).** The four rulebooks on each of the 13 code items (52 pairs): each is the right rule for code, but it repeats the digest and comes with placement `full` against D18. Also `rex-code-essentials` on `0h0d1109wo-T1`, the item that writes it: it did not exist when that job ran.

**Doubtful, tier B (17).**

| Item | Skill | Reason |
| --- | --- | --- |
| `0h0962klnd-T1`, `0h0962klnd-T2`, `0h0f8vnww4-T3` | `rex-code-tdd` | Found by `system-boundary`; a proof builds no feature to drive test first. |
| `0h0962klnd-T2` | `rex-agent-prompting` | The runner's nested sessions start a sub-agent, but with probe prompts, not briefs for a grid agent. |
| `0h0962klnd-T2`, `0h0f8vnww4-T3` | `rex-code-folder-management` | A new TypeScript fixture folder; the layout rules apply only in part. |
| `0h0dnqm24i-T2`, `0h0dnqm24i-T6` | `rex-code-folder-management` | The contract places the files; the item fills them. |
| `0h096dyeq9-T2` | `rex-code-tests-unit-testing` | The rule parsers suit inline tests, but the item asks for test files under `tests/`. |
| `0h0f8vnww4-T2` | `rex-code-tests-unit-testing` | The hook tests must stay, and the skill makes unit tests scaffolds to delete. |
| `0h0dnqm24i-T1` | `rex-code-improve-codebase-architecture` | Its words for depth and seams help a contract; its method finds friction in old code. |
| `0h0dnqm24i-T1` | `rex-code-tests-contract-seams`, `rex-code-tests-fitness-functions`, `rex-code-tests-integration-testing` | The item only registers empty test files; later tasks write the tests. |
| `0h0dnqm24i-reviewer` | `rex-code-spike`, `rex-code-validation-plan` | The reviewer checks spike answers and traces, but runs no spike and writes no plan. |
| `0h0fitv57s-T1` | `rex-agent-prompting` | The reference ticked `sub-agent` for the step after the skill writer reports; the item starts no agent. |

Every verdict, with its reason and the answers that hold it, is in `tests/fixtures/gate-a/judgments.json`.

## 2. The form test

**Result.** The two blind runs agree on 11 of 23 forms on every field except the free-text needs and the evidence quotes (line `m1.1` of `results/compare-forms.txt`). Against the judge's rulings, run 1 and run 2 are each right on every field of 18 of the 23 forms. The 5 forms with a field ruled wrong are `0h0962klnd-T3`, `0h096dyeq9-T2`, `0h0d1109wo-T1`, `0h0dnqm24i-T2` and `0h0f8vnww4-T2` in run 1, and `0h0962klnd-T1`, `0h0962klnd-T2`, `0h096dyeq9-T2`, `0h0d1109wo-T1` and `0h0dnqm24i-T1` in run 2.

Runs 1 and 2 were filled by grid agent `super-rex:model-high-effort-high`: two fresh agents with one brief, role form-writer. The reference forms were filled by `super-rex:model-high-effort-xhigh`, a third agent, role reference-writer. Both agent files, `plugin/agents/model-high-effort-high.md` and `model-high-effort-xhigh.md`, set `model: opus`, with effort `high` and `xhigh`. The table of model tiers in `rex-agent-prompting` names that model Opus 5.5; the agent files name only the alias `opus`. All 69 forms pass `validate-form.ts`.

"Raw" counts the items where a field equals the reference. "Right" counts the items where the judge rules the run right. A field that equals the reference counts right, unless a ruling on the other run found that shared value wrong. A field that differs counts right when the ruling is "run right" or "both defensible".

| Field (of 23 items) | Run 1 = run 2 | Run 1 raw | Run 1 right | Run 2 raw | Run 2 right |
| --- | --- | --- | --- | --- | --- |
| `facts.writes_code` | 23 | 23 | 23 | 23 | 23 |
| `facts.languages` | 23 | 23 | 23 | 23 | 23 |
| `facts.role`, `facts.latency_scale` (none set) | 23 | 23 | 23 | 23 | 23 |
| `activity` | 21 | 23 | 23 | 21 | 21 |
| `touches` | 18 | 15 | 20 | 17 | 22 |
| `must_have` | 23 | 22 | 23 | 22 | 23 |
| `artefacts` | 20 | 18 | 21 | 18 | 20 |
| number of needs | 17 | 19 | 23 | 13 | 23 |
| needs text | 0 | 0 | not ruled | 0 | not ruled |
| evidence terms | 14 | 0 | not ruled | 0 | not ruled |

Source: lines `m1.1`, `m2.1` and `m2.2` of `results/compare-forms.txt` for the raw counts, and the `forms` rulings of `judgments.json` (42 rulings, one per field that differs from the reference). How the right counts are made:

- `touches`, run 1: 15 equal, less 1 shared value that is wrong (`0h0962klnd-T3`, `unproven-approach`), plus 6 of 8 differences ruled run right or both defensible, gives 20.
- `touches`, run 2: 17 equal, plus 5 of 6 differences, gives 22.
- `artefacts`, run 1: 18 equal, plus 3 of 5 differences, gives 21.
- `artefacts`, run 2: 18 equal, plus 2 of 5 differences, gives 20. On `0h0dnqm24i-T1` run 2 is right on its one difference, but it shares `contract-test` and `fitness-test` with the reference, which the ruling on run 1 finds wrong, so the field counts wrong.
- `activity`, run 2: the 2 differences are run 2's `design` for the two proof tasks, ruled reference right.

The reference itself is wrong on 3 fields by these rulings: `touches` of `0h0962klnd-T3` and of `0h0fitv57s-T1`, and `artefacts` of `0h0dnqm24i-T1`. Most rulings are "both defensible" because the vocabulary leaves room. For example, `system-boundary` for a test that reads fixture folders, and `legacy-code` for any change to existing code, both fit the words of the term. The free-text needs agree on no form, so keyword search gets different words from each run for the same task.

## 3. The intent check of the 25 library skills

**Result.** All 78 intent phrases of the 25 library skills find their own skill in the top five among the library and the 1,146 pool skills. No phrase misses.

- `target/debug/rex skills check --pool /Users/shaun/Code/DEVELOPMENT/gate-a-pool/terminalskills/skills plugin/library` exits 0 and prints 25 `ok` lines and no intent finding (M14). Line `e0` of `steps-e-h-output.txt`.
- `intent-check.ts` sends each phrase as one free-text need. 74 phrases put their skill at rank 1 of the tier C items and 4 at rank 2 (line `e3`). The four are "design a public API" (`rex-code-ergonomics`, after `rex-code-design-sketch`), "design an error type" (`rex-code-error-writing`), "define a new module interface" (`rex-code-tests-contract-seams`) and "model a database or a class design" (`rex-mermaid-diagrams`).

This proves less than it seems. The pool skills have no intents. The keyword index weighs a match in the intents column 3, in the name 4 and in the card 2 (spec 8.3). So each phrase matches its own skill's intent text word for word, and no pool skill has such a text. The check shows that each phrase finds its skill. It does not show that a task's own words do: in part 1, keyword search found a right skill once in 89 needs.

## 4. Search on the real library

**Result.** The latency example of spec 8.3 behaves as described for both tasks, and every answer is byte-identical on a second run. Adding the pool changes no tier A or B item and not their order among themselves. But pool items of tier C come between them on the page, so their ranks move, and the pool can push an A or B item out of the top five.

**The latency example.** `latency-check.ts` runs the two hand-written forms of `tests/fixtures/gate-a/latency/` (lines `l1` and `l2` of `steps-e-h-output.txt`):

- The Rust matching loop (`latency_scale: ns_us`, `must_have: latency`) gets `rex-code-latency` at rank 2, tier B, slot `quality:latency`.
- The Go service in milliseconds does not get it on any page. It is under `gaps` with the reason "scope: languages go not in rust; latency_scale ms not in ns_us" and its `not_when` line.

Time of one search, as the wall time of one process from start to exit, median of 10 runs on the debug build (least to most in brackets):

| Form | Library and pool (`latency-check.ts --time 10`) | Library alone (`results/time-library-only.ts`) |
| --- | --- | --- |
| Rust matching loop (`latency-rust-ns`) | 907 ms (902 to 915) | 38 ms (38 to 39) |
| Go service in milliseconds (`latency-go-ms`) | 903 ms (898 to 906) | 36 ms (35 to 36) |

The same search over the library alone takes 36 to 38 ms, so most of the 0.9 seconds goes on indexing the 1,146 pool skills, which each process does again. The outputs are `results/latency-time.txt` and `results/latency-time-library-only.txt`.

**Same output on repeated runs.** `repeat-check.ts` searches each form twice. All answers were byte-identical: the 2 latency forms with and without the pool, and 23 of 23 forms of each of run 1, run 2 and the reference (`results/repeat-<run>.txt`, line `r2`).

**Adding the pool.** `pool-invariance.ts` compares the library alone with the library and the pool, over every page. The tier A and B items are the same and in the same order for the 2 latency forms and for 23 of 23 forms of each set (line `p2`). The first page is the same for 1 of 2 latency forms, and for 15, 20 and 22 of 23 forms of run 1, run 2 and the reference. The cause is the size limit: pool cards differ in size, so a different number of items moves to the next page.

The rank also moves, and that touches the bar. Run 1 searched over the library alone (`--library-only yes`, `results/run-1-library-only/`) scores 55 of 89 (61.8%), against 54 of 89 with the pool. The difference is `0h0d1109wo-T1`: `rex-code-ergonomics` is at rank 4 over the library alone, and at rank 6 with the pool, after the pool skills `idea-refine` and `review-swarm` at ranks 4 and 5. A tier C item can stand before an A or B item, because the page gives each need one item per round (part 1), and each free-text need is a need whose items are tier C.

## 5. Two open settings

### 5a. KEYWORD_SCORE_RATIO (Q14): not run

The ratio is the share of the best keyword score that a tier C candidate must reach to count as relevant (spec 8.2, Q14). It cannot be set or seen from outside the code, so the hit rate at three or more values was not measured. `ratio-check.ts` (lines `g0` to `g4` of `steps-e-h-output.txt`) shows what the code lacks:

1. An option or environment setting for `KEYWORD_SCORE_RATIO`. It is the constant `0.5` at `src/commands/skills/search/keyword.rs:23`, used at `keyword.rs:154` and `files.rs:125`. The skills code reads one environment value, `HOME`, and `--help` of `search`, `serve`, `check` and `index` names no ratio or score.
2. The score of each tier C item in the answer. The candidate holds its relevance, but the item that the answer prints does not.

No copy of the ranker was built. The one measured value is at the built-in ratio of 0.5:

| At ratio 0.5 | Run 1 | Run 2 | Reference forms |
| --- | --- | --- | --- |
| Needs that expect lookup C, hit in the top five | 13 of 28 | 16 of 32 | 3 of 7 |
| ... of them hit through a tier C item | 1 | 1 | 1 |
| Tier C candidates per free-text need, over all pages | 227 for 56 needs (4.05) | 202 for 50 (4.04) | 235 for 60 (3.92) |
| Needs with the cap of 5 candidates | 35 | 28 | 33 |
| Needs with no candidate | 2 | 2 | 2 |

The candidates come from the `why` field (`keyword: need <n>`) of each tier C item in the raw answers: `results/candidates-per-need-<run>.txt`, made by the command at the top of that file. The reference forms hold 60 free-text needs; they were searched but not labelled. The code stops a need's list at the first candidate below half the best score, or at the cap of five candidates (`keyword.rs:154`). Most needs reach the cap, so for them the cap, not the ratio, set the count. Of the 227 tier C items of run 1, 204 are pool skills. Only 3 are skills that a label names: the pool skills `sqlite` (rank 14) and `mcp-server-builder` (rank 11), and the library skill `rex-skill-language` (rank 2), the one in a top five.

### 5b. Finding G5: the size of a 10-item answer

**Result.** No real 10-item answer fits 1,200 tokens, even with every optional field left out. The binary keeps 7 or 8 items and drops all files and open-when items.

Sizes are the binary's own token estimate: 2 tokens for every 5 bytes of text and 5 tokens for every 8 bytes of a 64-digit hash. The budget is 120 tokens per item, for at least 10 items, so 1,200 tokens for a page of up to 10 items. When a page is over budget, the binary takes six steps in order: it cuts each long `why` to its head, then leaves out hashes, word counts, `placement`, `open_when` and `files`. If the page is still over budget, it moves items to the next page. `size-lib.ts` copies this, and `size-proof.ts` proves the copy against the binary on 71 answers (line `h2`). The sizes were measured on base `75566e3`, before a parallel job, which runs beside this one and is not judged here, adds open_when items to up to 12 library headers. So later answers will be larger.

| Real answers with 10 items on the full first page (least / median / most tokens) | Run 1 (18 answers) | Run 2 (17 answers) | Reference (18 answers) |
| --- | --- | --- | --- |
| Full page, every field | 1,990 / 2,732 / 3,148 | 1,972 / 2,672 / 3,190 | 1,920 / 2,798 / 3,220 |
| Without files and open_when | 1,915 / 1,990 / 2,081 | 1,936 / 1,981 / 2,061 | 1,914 / 2,003 / 2,116 |
| After all six steps of the binary | 1,350 / 1,425 / 1,554 | 1,386 / 1,412 / 1,492 | 1,349 / 1,433 / 1,580 |
| What the binary returns | 7 to 8 items, 1,049 to 1,194 tokens | 7 to 8 items, 1,059 to 1,190 tokens | 7 to 8 items, 1,054 to 1,190 tokens |

Source: `results/answer-size-<run>.txt`. Over all 23 answers, the full page is over its budget for 22 of run 1, 20 of run 2 and 23 of the reference. A full 10-item page in run 1 holds 0 to 12 files and 0 to 11 open-when items.

What each option of G5 costs, on the 18 ten-item answers of run 1 (`results/g5-options-run-1.txt`, made by `node tests/fixtures/gate-a/results/g5-options.ts --forms /Users/shaun/Code/DEVELOPMENT/gate-a-pool/filled/run-1`, which uses the proven copy of `size-lib.ts` and changes no setting of the binary):

| Option | What it costs |
| --- | --- |
| (a) A budget larger than 1,200 tokens | To show all 10 items: 1,990 to 3,148 tokens (median 2,732) with every field; 1,483 to 2,051 (median 1,897) with files and open_when kept and hashes, words, placement and the long `why` left out; 1,350 to 1,554 (median 1,425) with every optional field left out. Each search answer costs the caller that much context, once per task. |
| (b) Shorter cards | With files and open_when kept, no card length fits in 13 of 18 answers, not even an empty card. With every optional field left out, cards of at most 12 to 21 words (median 16) fit, against 30 words today. |
| (c) Files without hashes, items to the next page sooner | Each item keeps its `files` (without their hashes) and its `open_when`. When the page is over budget, items move to the next page before any file or open-when item is left out. Then 4 to 8 of the 10 items stay on the first page (median 5). The binary today keeps 7 or 8 (median 8) with no files or open-when items. So at the median, 3 more items than today (8 less 5) wait for a second call. |

One more size defect, which the agent that wrote the gate A scripts found, still holds: an answer can stay over the budget, because `gaps` are never shortened and the loop that moves items to the next page stops at one item. The request that shows it is `probe-nocode-all`, built by `size-proof.ts` (all terms, `writes_code: false`, library alone): 1 item at 1,242 tokens.

## 6. Weaknesses of this evidence, and the two bars

### The two bars

| Bar (spec 12) | Measured |
| --- | --- |
| The right skill is in the top five for at least 70% of needs | 60.7% (54 of 89) in run 1, the primary blind run. 57.0% (53 of 93) in run 2. 60.3% (41 of 68) for the reference forms, on skill labels only. Without the 3 header-quote items: 63.4%, 58.7% and 58.9%. |
| No lookup A or B assignment is judged wrong | 5 tier B items judged wrong in run 1's answers, 9 in run 2's and 5 in the reference's; 11 distinct pairs of item and skill in all. No tier A item is judged wrong; 53 are doubtful. |

### Weaknesses

| Weakness | What it means for the numbers |
| --- | --- |
| Blindness rests on a brief, sealed folders and one check of transcripts. | Each form writer got a folder with only `tasks.json`, `terms.json` and `aliases.json`. The orchestrator's list of their tool calls (`tests/fixtures/gate-a/transcripts.txt`, cut at 300 characters per call; the repo's whitespace hook removed one trailing space from the copy) shows that each read only those files. But the agents still had tools that could reach the library: `Read` and `Bash` on the whole disk, and `rex_skill`. Run 2 and the reference called `rex_skill` once with no name, which returns the brief assembly and no skill text. Run 1 listed the file names of this job's plan folder once; no file there is a skill header, and it opened none. Nothing but the transcript shows that no header was seen. |
| One party wrote the labels and the verdicts. | The judge, Claude Opus 5.5, wrote the skill labels before reading any form, and the need labels before any search. The verdicts were written after the answers were seen, by the same party. No second judge checked them; one changed verdict on a wrong item moves bar 2. |
| The labels rest on the headers only. | On reading the bodies, the judge found right assignments that the labels left out: 6 in run 1's answers, 7 in run 2's and 9 in the reference's. Most are `rex-code-tests-integration-testing` for tests of the binary, which its body counts as integration tests. One is `rex-code-failure-policy` on `0h0dnqm24i-T3`, which the label rules themselves called for ("never an error"). Such a skill takes a top-five place and is not counted as a hit. |
| The labels choose the digest over the rulebooks. | With the four rulebooks counted as labels, run 1 falls from 60.7% to 47.5% (67 of 141). |
| Pool labels rest on folder names. | 10 needs of run 1 and 14 of run 2 name a pool skill, and their text was never read. None of them was hit. |
| Some items quote the headers. | 3 items hold a header phrase of two or more words; the rates without them are given. 16 more hold one-word terms such as `build`, which every task text uses. |
| All 7 jobs are of the same work. | Every item comes from the jobs that built this skill library in SuperRex itself: a Rust CLI and a TypeScript mod. Their texts use the spec's own words, such as "spike", "fitness" and "contract". Work of another kind, or another repo, may score differently. |
| There is no real Go task. | The Go case is one hand-written form, `latency/go-service-ms.json`. No item writes Go, and only 3 write TypeScript. |
| One model at one effort per role. | Both blind runs are Opus at effort high with one brief, and the reference is the same model at effort xhigh. Agreement between the runs says nothing about another model, effort or brief. |
| The pool is unreviewed and has no intents. | Its skills compete only in lookup C, by name and card. It says nothing about a pool whose skills declare claims and intents. |
| The counts are small. | 23 items, and 89 needs in run 1. One need is 1.1 points of bar 1. Five items have no library skill at all. |
| The sizes are of one state of the library. | They were measured on `75566e3`, before a parallel job adds open_when items to up to 12 library headers. |
| The times are of a debug build on one machine. | Most of each call is the indexing of 1,146 pool skills in every process. |

### The old table, compared after the scores

`tests/fixtures/gate-a/old-table.json` holds, for each item, the skills that the old assignment table gave it, in full and as open-when items. The judge read it only after the scores were committed.

Count the digest as the four rulebooks that it replaces: the 68 skill labels, less the 13 digest labels, plus 4 rulebooks on each of those 13 items, make 107 pairs of item and library skill. Every one of them is also in the old table, in full or as an open-when item. The old table names 57 more such pairs, 56 of them as open-when items. The one more in full is `rex-code-tests-integration-testing` on `0h0b8x725g-T1`, which the judge rules right in part 1 and the labels left out. So the blind labels are a narrower set than the old table, not a different one. The old table also gives `rex-pr-review` to the 6 reviewers and `rex-simplify` to `0h0962klnd-T3`; both are listed skills outside the library, as the labels say.

### Repeat of the scores, and checks by hand

On the final branch, every score command of this document was run a second time into a scratch folder: `score-hits.ts` for each of the six pairs of labels and answers in `results/`, and `compare-forms.ts`. Each output is byte-identical to the committed file. `g5-options.ts` also gave the same numbers on a second run, and `steps-e-h.ts` gave the committed `steps-e-h-output.txt` byte for byte (sha256 `6b93af67464bb3f8f02dcc078c49e4e3c2c585dc2c9b742212764a1483db0ae0`).

**Repeat after the merge of main.** Before delivery, `main` was merged into the job branch. It had gained `ee3170d`, the parallel job, which adds 32 lines of open_when items to 13 library headers and changes nothing in `src/`. So every number above belongs to the library as of `75566e3`. To repeat them on a later tree, first put back that library with `git checkout 75566e3 -- plugin/library`, run the commands, and restore it with `git checkout HEAD -- plugin/library`. The scripts find the repo root from their own folder. The orchestrator did this on the merged branch. `steps-e-h.ts` then gave `steps-e-h-output.txt` byte for byte, and `run-search.ts` on run 1 gave `results/run-1/` byte for byte. On the merged library without the restore, run 1 changed only in the index version on each answer (24 lines). `score-hits.ts` on it still gave 54 of 89 (60.7%), and tier B still had 5 wrong. The sizes of part 5 grow: the latency answers go from 3,851 to 4,055 tokens (Rust) and from 2,532 to 2,618 (Go) on the full page. On the binary's page, the Rust answer still holds 8 items at 1,186 tokens and the Go answer 8 items at 1,200 tokens.

One item of each part was also checked by hand, from the raw answer, the forms and the labels, and each matches the script:

| Part | Item | Checked by hand | Script |
| --- | --- | --- | --- |
| 1 | `0h0dnqm24i-T2`, run 1 | In the raw answer, ranks 1 to 5 are `rex-code-commenting`, `rex-code-tests-fitness-functions`, `rex-code-tests-contract-seams`, `rex-code-tdd` and `rex-code-failure-policy`. Of its 5 skill labels and 2 need labels with a right skill, 6 are hit; `rex-code-essentials` is at rank 16, the last item of the second page. | `score-hits.ts` lists only that label as a miss of this item |
| 2 | `0h0962klnd-T1`, `activity` | The forms say `test` (run 1), `design` (run 2) and `test` (reference). | `compare-forms.ts` lists the same two differences |
| 3 | "design a public API" | Searched by hand as one need: the tier C items are `rex-code-design-sketch`, then `rex-code-ergonomics`. | `intent-check.ts`: rank 2 of 2 |
| 4 | `latency-go-ms` | Searched by hand: `rex-code-latency` is on no page, and is under `gaps` with the reason and the `not_when` line. | `latency-check.ts`: pass |
| 5 | `0h0fitv57s-T1`, run 1 | The raw first page is 1,151 bytes, with 2 hashes of 64 hex digits (128 bytes). The other 1,023 bytes at 2 tokens per 5 bytes give 410 tokens, and the hashes at 5 tokens per 8 bytes give 80: 490 tokens. | `answer-size.ts`: 2 items, 490 tokens |
