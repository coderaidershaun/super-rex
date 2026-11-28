# The skill library vocabulary, version 2

This file explains the shared vocabulary of SuperRex skill libraries and shows where each term comes from. The vocabulary is held in two files: `plugin/vocab/terms.json`, which lists the allowed terms, and `plugin/vocab/aliases.json`, which maps free words onto those terms. Part 1 maps every criterion, row and sibling file of `plugin/assign/table.json` onto the vocabulary, so that no row is lost when the table gives way to skill headers. Part 2 gives the value domain of each fact and scope key. Part 3 names the source of each term that does not come from the table. Part 4 lists the choices this work made where the specification left a gap. Part 5 gives the exact JSON form of both files and the rules that `rex skills check` applies with them. The specification is the change specification "SuperRex skill library: lookup, search and assignment" of 27 November 2026; section numbers below refer to it.

## Counts

The table at commit `e03e640` holds 23 rows. The rows hold 33 criteria and 7 `files` items. These counts match the brief. Three criteria are decided by code (`writes-code`, `language-not-rust`, `role:plan-reviewer`), so 30 criteria are questions to the decision agent today. Section 3.4 of the specification says "up to 31 yes/no criteria"; the table holds 30, and the difference does not change this contract.

`terms.json` holds 52 terms: 4 kinds, 8 categories, 6 facts, 3 scope keys, 8 activities, 13 touches, 2 qualities and 8 artefacts. No criterion was dropped: all 33 map to a term.

## How to read a term

A term belongs to one group. The group decides which field of a skill header and which field of a search request may use it.

| Group | Header field (section 7.1) | Request field (section 8.2) | Lookup |
| --- | --- | --- | --- |
| `kinds` | `kind` | `kinds` (filter) | none: a filter |
| `categories` | `category` | `categories` (filter) | none: a filter |
| `facts` | `claims.facts` | `facts` | A |
| `scope_keys` | `scope` | none: checked against `facts` | the scope check of B |
| `activities` | `claims.activity` | `activity` | B |
| `touches` | `claims.touches` | `touches` | B |
| `qualities` | `claims.quality` | `must_have` | B |
| `artefacts` | `claims.artefact` | `artefacts` | B |

A term is unique inside its group. The same word may be a term of two groups, and the group decides what it means: `design` and `review` are both activities and categories, because sections 7.1 and 7.3 use them in both places. Below, a term is written `group:term`, for example `touches:file-layout`.

---

## Part 1. The mapping of `table.json`

### 1.1 Criteria

Each line maps one criterion to the term it becomes. The last column names the skill whose header claims that term in WP2. The criterion `each-step` is not in this list: it is not a criterion of any row, but the question that decides placement today. Its answer is carried by the header field `placement` and by the assigner's choice of `full` or `open-when` (section 1.2).

| ID | Row | Criterion | Maps to | Claimed in WP2 by |
| --- | --- | --- | --- | --- |
| C01 | `code-rulebooks` | `writes-code` (rule) | `facts:writes_code` = `true` | `rex-code-essentials`. The four rulebooks claimed it in WP2 and lost the claim after gate A: they are open-when items of the digest |
| C02 | `gap-skill` | `language-not-rust` (rule) | `facts:languages`: each gap skill claims its own language and `writes_code` = `true` | `rex-gap-ts` claims `languages: [ts]`; each later `rex-gap-<language>` claims its language |
| C03 | `tdd` | `builds-feature` | `activities:build` | `rex-code-tdd` |
| C04 | `tdd` | `fixes-bug` | `activities:fix` | `rex-code-tdd` |
| C05 | `unit-tests` | `unit-test` | `artefacts:unit-test` | `rex-code-tests-unit-testing` |
| C06 | `contract-tests` | `contract-test` | `artefacts:contract-test` | `rex-code-tests-contract-seams` |
| C07 | `fitness-tests` | `invariant` | `artefacts:fitness-test` | `rex-code-tests-fitness-functions` |
| C08 | `fitness-tests` | `property-test` | `artefacts:fitness-test` | `rex-code-tests-fitness-functions` |
| C09 | `integration-tests` | `real-world` | `artefacts:integration-test` | `rex-code-tests-integration-testing` |
| C10 | `boundary-tests` | `outside-system` | `touches:system-boundary` | no skill since gate A. WP2 gave it to `rex-code-tdd` for its file `mocking.md` (line F2); the four test-rule skills still reach that file as an `open_when` item |
| C11 | `boundary-tests` | `time-or-randomness` | `touches:system-boundary` | no skill since gate A, as C10 |
| C12 | `boundary-tests` | `file-system` | `touches:system-boundary` | no skill since gate A, as C10 |
| C13 | `folders` | `creates-or-splits` | `touches:file-layout` | `rex-code-folder-management` |
| C14 | `folders` | `moves` | `touches:file-layout` | `rex-code-folder-management`, whose file `move.md` keeps the case of moving (line F3) |
| C15 | `dependencies` | `adds-or-upgrades` | `touches:dependencies` | `rex-code-external-libraries` |
| C16 | `failure-policy` | `outside-input` | `touches:outside-input` | `rex-code-failure-policy` |
| C17 | `failure-policy` | `long-running-state` | `touches:long-running` | `rex-code-failure-policy` |
| C18 | `failure-policy` | `retry-or-reconnect` | `touches:retry` | `rex-code-failure-policy` |
| C19 | `failure-policy` | `main-loop` | `touches:long-running` | `rex-code-failure-policy` |
| C20 | `design` | `design-first` | `activities:design` | `rex-code-design-sketch`, `rex-code-validation-plan` |
| C21 | `spike` | `unproven` | `touches:unproven-approach` | `rex-code-spike` |
| C22 | `smells` | `refactor-unfamiliar` | `touches:legacy-code` | no skill since gate A. WP2 gave it to `rex-code-smells`, which only audits |
| C23 | `smells` | `audit` | `activities:audit` | `rex-code-smells` |
| C24 | `architecture` | `architecture-friction` | `touches:architecture` | `rex-code-improve-codebase-architecture` |
| C25 | `priorities` | `ranked-decision` | `touches:trade-offs` | `rex-code-priorities` |
| C26 | `latency` | `hot-path` | `qualities:latency`, with scope `languages: [rust]` and `latency_scale: [ns_us]` | `rex-code-latency` |
| C27 | `pr-review` | `reviews-others` | `activities:review` | `rex-pr-review`, a listed skill that opts in to search (Q7) |
| C28 | `diagrams` | `draws-diagram` | `artefacts:diagram` | `rex-mermaid-diagrams` |
| C29 | `skill-text` | `writes-instructions` | `artefacts:instructions` | `rex-skill-language` |
| C30 | `briefs` | `starts-agent` | `touches:sub-agent` | `rex-agent-prompting` |
| C31 | `briefs` | `writes-brief` | `artefacts:brief` | `rex-agent-prompting` |
| C32 | `plan-review` | `plan-reviewer` (rule `role:plan-reviewer`) | `facts:role` = `"plan-reviewer"` | `rex-codex-adversarial-review` |
| C33 | `plan-writing` | `writes-code-plan` | `artefacts:plan` | `rex-code-design-sketch`, `rex-code-validation-plan` |

Several criteria share a term where they select the same skill and mean nearly the same thing: C07 and C08, C10 to C12, C13 and C14, and C17 and C19. Each pair or group stays findable because the aliases file maps each original wording ("property test", "invariant", "main loop", "file system" and the rest) onto the shared term.

The criterion C26 also names "a Linux host it controls". No fact can carry that, so code could never check it and the scope result would always be unknown. That condition goes into the `not_when` line of `rex-code-latency` instead, for example "A hot path measured in milliseconds, or one on a host you do not control."

The criterion C02 is a rule over a template: the row names `rex-gap-{language}`, and code fills in each language of the task other than Rust. In the new format each gap skill claims its own language. The server still needs to know which language lacks a gap skill, to report it under `gaps` (section 8.2); WP4 can find gap skills by the name pattern `rex-gap-<language>`.

### 1.2 Rows

Each line gives one row's `priority`, `placement` and `scope`, and what each becomes in the header of the row's skills.

- **`priority`** has no home in the header (Q13). It is recorded here and dropped. Today a lower number keeps a skill in full when a task has more read-first skills than the cap.
- **`placement`** becomes the header field `placement`. `read-first` becomes `full`. `open-when` becomes `open-when`. `ask` becomes `full`: today the `each-step` question decides between the two, and in the new design the assigner makes that choice per task in `rex_assign` (section 9.2), so the header only gives the default.
- **`scope`** `code` means the row applies only to a task that writes code, which is the fact `writes_code`. It becomes `scope: { writes_code: [true] }` in the header. A row whose own criterion is the fact `writes_code` (C01, C02) needs no scope key, because its claim already requires that fact. `any` means no `writes_code` key.

| Row | Skills | `priority` | `placement` → header | `scope` → header |
| --- | --- | --- | --- | --- |
| `code-rulebooks` | the four rulebooks | 1 | `read-first` → `full` | `code` = `writes_code`, held by the claim of `rex-code-essentials`; the four rulebooks have no claim since gate A, and no scope key |
| `gap-skill` | `rex-gap-{language}` | 2 | `read-first` → `full` | `code` = `writes_code`, held by the claim; no scope key |
| `tdd` | `rex-code-tdd` | 3 | `ask` → `full` | `code` = `writes_code: [true]` |
| `unit-tests` | `rex-code-tests-unit-testing` | 40 | `ask` → `full` | `code` = `writes_code: [true]` |
| `contract-tests` | `rex-code-tests-contract-seams` | 40 | `ask` → `full` | `code` = `writes_code: [true]` |
| `fitness-tests` | `rex-code-tests-fitness-functions` | 40 | `ask` → `full` | `code` = `writes_code: [true]` |
| `integration-tests` | `rex-code-tests-integration-testing` | 40 | `ask` → `full` | `code` = `writes_code: [true]` |
| `boundary-tests` | none; the file `rex-code-tdd/mocking.md` | 40 | `open-when` → the file is a topic file, always open-when | `code` = `writes_code: [true]`, on `rex-code-tdd` |
| `folders` | `rex-code-folder-management` | 40 | `ask` → `full` | `code` = `writes_code: [true]` |
| `dependencies` | `rex-code-external-libraries` | 40 | `ask` → `full` | `code` = `writes_code: [true]` |
| `failure-policy` | `rex-code-failure-policy` | 30 | `ask` → `full` | `code` = `writes_code: [true]` |
| `design` | `rex-code-design-sketch`, `rex-code-validation-plan` | 20 | `ask` → `full` | `any`: no scope key |
| `spike` | `rex-code-spike` | 20 | `ask` → `full` | `any`: no scope key |
| `smells` | `rex-code-smells` | 30 | `ask` → `full` | `code` = `writes_code: [true]` |
| `architecture` | `rex-code-improve-codebase-architecture` | 50 | `ask` → `full` | `code` = `writes_code: [true]` |
| `priorities` | `rex-code-priorities` | 50 | `ask` → `full` | `any`: no scope key |
| `latency` | `rex-code-latency` | 30 | `ask` → `full` | `code` = `writes_code: [true]`, plus `languages: [rust]` and `latency_scale: [ns_us]` |
| `pr-review` | `rex-pr-review` | 20 | `ask` → `full` | `any`: no scope key |
| `diagrams` | `rex-mermaid-diagrams` | 60 | `ask` → `full` | `any`: no scope key |
| `skill-text` | `rex-skill-language` | 20 | `ask` → `full` | `any`: no scope key |
| `briefs` | `rex-agent-prompting` | 30 | `ask` → `full` | `any`: no scope key |
| `plan-review` | `rex-codex-adversarial-review` | 20 | `read-first` → `full` | `any`: no scope key |
| `plan-writing` | `rex-code-design-sketch`, `rex-code-validation-plan` | 20 | `ask` → `full` | `any`: no scope key |

Some skills come from two rows. `rex-code-design-sketch` and `rex-code-validation-plan` come from `design` and `plan-writing`, both of scope `any`. `rex-code-tdd` comes from `tdd`, of scope `code`; its file `mocking.md` comes from `boundary-tests`, whose claim the header no longer carries since gate A. `rex-code-folder-management` and `rex-code-failure-policy` come from their own rows and, through their files, from `smells`, all of scope `code`. No skill gets rows of two different scopes, so each skill's header gets one scope with no conflict. A skill's claims are the union of the terms of all its rows.

Every skill of scope `code` gets a `scope`, so section 10 requires a `not_when` line on each of them. WP2 must write one for each of the 11 skills concerned: the eight skills of the rows `tdd` to `failure-policy`, `rex-code-smells`, `rex-code-improve-codebase-architecture` and `rex-code-latency`. After gate A, `rex-code-spike`, `rex-code-design-sketch` and `rex-code-validation-plan` also have `scope: { writes_code: [true] }` with a `not_when` line, so 14 library skills have a scope.

The table-level fields are not vocabulary. `cap` (6) is replaced by the cap of three in section 9.3. `format` and `about` describe the table file. The row field `work` and each criterion's `text` are source text for each skill's `intents` in WP2.

### 1.3 Sibling files

Each line maps one `files` item of the table. Each item becomes an entry of the `files` list of its own skill, with role `topic` and the case as its `when`. The cases below are quoted from the table; WP2 may reword them into the "you ..." form of the example in section 7.1.

- **F1.** Row `integration-tests`: `rex-code-tests-integration-testing/real-funds.md`, case "a test moves real funds". It becomes `{ path: real-funds.md, role: topic, when: "a test moves real funds" }` in that skill.
- **F2.** Row `boundary-tests`: `rex-code-tdd/mocking.md`, case `null`. Today a `null` case falls back to the text of the criterion that fitted (`plugin/hooks/assign/rules.ts`, line 101). It becomes `{ path: mocking.md, role: topic, when: "a test crosses to an outside API or database, depends on time or randomness, or crosses the file system" }` in `rex-code-tdd`, which also claims `touches:system-boundary`. WP2 may also add an `open_when` item `{ skill: rex-code-tdd, file: mocking.md, when: ... }` to the four test-rule skills, so a test task reaches the file without the TDD method.
- **F3.** Row `folders`: `rex-code-folder-management/move.md`, case "it moves a file, module or folder". It becomes a topic file of that skill with that case.
- **F4.** Row `failure-policy`: `rex-code-failure-policy/failure-table.md`, case "it plans such a feature: writes its failure table". It becomes a topic file of that skill with that case.
- **F5.** Row `smells`: `rex-code-folder-management/audit.md`, case "it audits code". It becomes a topic file of `rex-code-folder-management` with that case, and an `open_when` item of `rex-code-smells`: `{ skill: rex-code-folder-management, file: audit.md, when: "it audits code" }`.
- **F6.** Row `smells`: `rex-code-failure-policy/audit.md`, case "it audits code". It becomes a topic file of `rex-code-failure-policy` with that case, and an `open_when` item of `rex-code-smells`: `{ skill: rex-code-failure-policy, file: audit.md, when: "it audits code" }`.
- **F7.** Row `latency`: `rex-code-latency/measure.md`, case "it measures the hot path". It becomes a topic file of that skill with that case. The example in section 8.2 uses the case "you have no measurement yet"; this mapping keeps the table's case.

Three of the seven items (F2, F5, F6) are files that the table gives with a row of a different skill, or with a row that has no skill. Section 7.1 lets an `open_when` item name only another skill or a file of the same skill, which would lose them. Part 4, choice 1, adds an optional `file` to an `open_when` item to keep them.

Two behaviours change, and WP2 and gate A should watch them. First, today `mocking.md` reaches a boundary test as one file, with no skill. After the change, a boundary test finds `rex-code-tdd` by lookup B, and the assigner should give it as `open-when` with `mocking.md` when the task needs nothing else of TDD. Second, `activities:review` (C27) also fits a plan reviewer, which today answers "no" to `reviews-others`. Lookup B will return `rex-pr-review` to a plan reviewer that ticks `review`, and the assigner must decline it.

---

## Part 2. Value domains

### 2.1 Facts

A fact is something about the task that code or the decision agent states. A skill claims facts under `claims.facts`; lookup A matches them. "Closed" means a value outside the list is an error. "Open" means any value is allowed: a pattern, when given, is checked, and otherwise the value is not checked.

| Fact | Type | Values | Supplied by | May a skill claim it? |
| --- | --- | --- | --- | --- |
| `writes_code` | boolean | Closed: `true`, `false`. | The decision agent. Always present. | Yes. |
| `languages` | list of text | Closed: the 24 values of `rex codebase --only`, from `src/commands/codebase/language.rs`: `rust`, `go`, `c`, `cpp`, `python`, `js`, `ts`, `shell`, `java`, `kotlin`, `scala`, `csharp`, `swift`, `objc`, `dart`, `php`, `ruby`, `lua`, `haskell`, `elixir`, `r`, `sql`, `zig`, `perl`. | The decision agent; left out, the job's languages, as `rex_assign` does today. | Yes. |
| `role` | text | Open. Checked against the pattern `^[a-z0-9][a-z0-9-]*$`, the id pattern of `plugin/hooks/assign/table.ts`. Roles are named per plan, so no list exists. | The decision agent. | Yes. |
| `repo_has` | list of text | Open and not checked: paths of files in the repo, relative to its root, such as `Cargo.toml`. | The mod, by code. It overwrites what the agent wrote (section 9.1). | Yes. |
| `repo_dependencies` | list of text | Open and not checked: names of packages the repo depends on, as its manifests name them. | The mod, by code. | Yes. |
| `latency_scale` | text | Closed: `ns_us` (nanoseconds to tens of microseconds) and `ms` (milliseconds or slower). | The decision agent, when the task names a time budget; left out otherwise. | No. It is only checked against a skill's `scope`. |

In `claims.facts`, a value has the type of its fact: a boolean for `writes_code`, text for `role`, and a list of at least one value for `languages`, `repo_has` and `repo_dependencies`. A fact claim holds when every fact it names holds for the request. A boolean or text fact holds when the request's value is equal. A list fact holds when the request's list shares at least one value with the claim's list. A fact the request leaves out does not hold.

### 2.2 Scope keys

A scope key limits a skill that lookup B found. Each key names a fact, and the scope check reads that fact from the request. The value of a key in a header's `scope` is always a list of at least one allowed value, taken from the closed list of its fact. A key can only name a fact whose values are closed.

| Scope key | Checked against | Type in the header | Values |
| --- | --- | --- | --- |
| `writes_code` | `facts.writes_code` | list of boolean | Closed: `true`, `false`. In practice `[true]`. |
| `languages` | `facts.languages` | list of text | Closed: the 24 language values of section 2.1. |
| `latency_scale` | `facts.latency_scale` | list of text | Closed: `ns_us`, `ms`. |

For each key, the result is "unknown" when the request leaves the fact out, "yes" when the request's value (or, for a list fact, any one of its values) is in the key's list, and "no" otherwise. The skill's scope result is "no" if any key gives "no", else "unknown" if any key gives "unknown", else "yes". Section 8.3 says what each result does. For example, `rex-code-latency` with `scope: { writes_code: [true], languages: [rust], latency_scale: [ns_us] }` gives "yes" for a Rust matching loop with `latency_scale: ns_us`, "unknown" for a Rust task that names no budget, and "no" for a Go service with `latency_scale: ms`.

---

## Part 3. Terms that do not come from `table.json`

- **Kinds** `rule`, `method`, `tool`, `reference`: section 7.2 (DECIDED).
- **Categories**, all eight: section 7.3, as Q1 accepts them.
- **Fact values of `languages`**: the `--only` values of `rex codebase`, read from `src/commands/codebase/language.rs`. The fact itself comes from the table's rule `language-not-rust`.
- **Fact `repo_has`**: sections 8.2 and 9.1.
- **Fact `repo_dependencies`**: sections 8.3 ("files and dependencies present in the repo") and 9.1 ("The mod reads `repo_has` and dependencies"). The name is new; see Part 4, choice 8.
- **Fact and scope key `latency_scale`**, with values `ns_us` and `ms`: the notes of section 7.1.
- **Scope key `languages`**: the notes of section 7.1.
- **Activity `refactor`**: the example of section 7.1. No criterion maps to it, because the table's only refactor criterion (C22) is limited to old or unfamiliar code. It is there so that a refactor task has an activity to tick.
- **Activities `test` and `document`**: added by this work. A request takes one activity (section 8.2), so a task that only writes tests, or only writes documentation such as a README diagram, needs an activity of its own.
- **Touches `threads` and `allocation`**: the example of section 7.1. They give the topic files of `rex-code-latency` a coarse pre-filter (section 8.5).
- **Quality `readability`**: the example of section 7.1.

Every other term comes from a criterion, a rule or a row field of the table, as Part 1 shows. The scope key `writes_code` comes from the row field `scope`.

---

## Part 4. Choices this work made

The specification marks the vocabulary AGREED in substance and the file names and field spellings PROPOSED. These are the choices it did not fix, with their reasons.

1. **An `open_when` item may name a file of another skill.** Section 7.1 lets an item name another skill or a file of the same skill. This work adds a third form, `{ skill, file, when }`. Three of the seven sibling files of the table (F2, F5, F6) reach a task through a row of another skill, and they would be lost without it. The check that each target exists then checks the skill and the file.
2. **`writes_code` is a scope key.** It carries the row field `scope: code`. Without it, a skill such as `rex-code-tdd`, found by `activities:build`, would also be returned to a task that writes no code.
3. **A scope value is always a list.** Section 7.1 writes `latency_scale: ns_us | ms`. One form for every key lets the check and the scope test treat all keys the same way.
4. **`ask` becomes `full`.** The header holds only a default, and the assigner chooses `full` or `open-when` per task. The rows of placement `ask` hold skills that shape a whole task when they apply.
5. **Criteria that select the same skill share a term.** This keeps the vocabulary near 50 terms. Each original wording is kept as an alias.
6. **The Linux-host condition of C26 is not a scope key.** No fact could carry it, so its result would always be unknown. It goes into `not_when`.
7. **`ms` covers milliseconds or slower**, so that every hot path has a value.
8. **The dependency fact is named `repo_dependencies`.** The name `dependencies` is already the touches term of C15, and the fact name follows `repo_has`, since the mod reads both from the repo.
9. **Each fact says whether a skill may claim it.** `latency_scale` may not: it describes a budget, and only a scope uses it.
10. **`public-api` is left out.** The example of section 7.1 lists it under `touches`, but no row needs it, and the skills it would fit are found by lookup A.
11. **`rex-pr-review` gets kind `method` and category `review`.** Section 7.4 does not list it, because it is a listed skill. Q7's default lets it opt in.
12. **The alias file has two lists.** `aliases` maps a free word onto a term, with a relation. `broader_terms` puts a term under a broader term, as section 7.5 puts `latency` under `performance`. A free word may point only to a term of a lookup B group (activities, touches, qualities, artefacts). Version 2 adds a third list, `value_aliases`: a common full name of an allowed value of a closed fact, such as `typescript` for the language `ts`. Without it, a request that names `typescript` gets no gap skill and only an `unrecognised` line. Facts and scope keys themselves are set by code and the agent, so they need no aliases.

Two points are for later work packages and do not change these files.

- `src/bundle.rs` embeds `plugin/skills`, `plugin/agents` and `plugin/library`, not `plugin/vocab`. WP1 or WP5 decides whether the binary embeds the vocabulary too.
- Section 8.2 puts the vocabulary version into the library `version`. A change to `aliases.json` changes keyword results, so WP4 should fold the `version` of `aliases.json` into that hash too. `head` reports the `version` of `terms.json` as its `vocabulary`.

---

## Part 5. The exact form of both files

### 5.1 `plugin/vocab/terms.json`

The file is one JSON object with these keys, and no others.

- `version`: text of decimal digits, now `"2"`. It goes up by one with every change to the file.
- `about`: text that says what the file is for.
- Eight group keys: `kinds`, `categories`, `facts`, `scope_keys`, `activities`, `touches`, `qualities`, `artefacts`. Each is an object with three keys:
  - `header`: text, the header field that takes the group's terms, as in the table of "How to read a term".
  - `request`: text, the request field that takes them, or `null` for `scope_keys`.
  - `terms`: a list of objects, in a fixed order.

For the six groups `kinds`, `categories`, `activities`, `touches`, `qualities` and `artefacts`, each term object has exactly two keys: `term` (text) and `about` (text). The term matches `^[a-z][a-z0-9]*(-[a-z0-9]+)*$`.

For `facts`, each term object has these keys:

- `term`: text matching `^[a-z][a-z0-9]*(_[a-z0-9]+)*$`.
- `type`: `"boolean"`, `"string"` or `"list"`. A `list` is a list of text.
- `values`: `"closed"` or `"open"`.
- `allowed`: only when `values` is `"closed"`; the list of allowed values, booleans for a boolean fact and text otherwise.
- `pattern`: only when `values` is `"open"`; a regular expression that each value must match, or `null` when values are not checked.
- `supplied_by`: `"decision-agent"` or `"mod"`.
- `claimable`: boolean, whether the fact may appear in `claims.facts`.
- `about`: text.

For `scope_keys`, each term object has these keys:

- `term`: text matching the same pattern as a fact.
- `fact`: text, the name of a fact whose `values` is `"closed"`. The key's allowed values are that fact's `allowed` list.
- `about`: text.

A term is unique inside its group. The engineer can read the file with this shape (written here as a type, not as code to copy):

```
Terms = {
  version: string, about: string,
  kinds | categories | activities | touches | qualities | artefacts:
    { header: string, request: string, terms: [ { term: string, about: string } ] },
  facts: { header: string, request: string, terms: [
    { term: string, type: "boolean" | "string" | "list", values: "closed" | "open",
      allowed?: [bool | string], pattern?: string | null,
      supplied_by: "decision-agent" | "mod", claimable: bool, about: string } ] },
  scope_keys: { header: string, request: null, terms: [ { term: string, fact: string, about: string } ] }
}
```

### 5.2 `plugin/vocab/aliases.json`

The file is one JSON object with these keys, and no others.

- `version`: text of decimal digits, now `"2"`. It goes up by one with every change to the file.
- `vocabulary`: text, the `version` of `terms.json` that this alias table was written for. It must be equal to it.
- `about`: text.
- `aliases`: a list of objects, each with exactly four keys:
  - `word`: text in lower case, with no space at either end and no two spaces in a row. It is never equal to a term of the group it points to.
  - `relation`: `"synonym"` (the word means the term), `"narrower"` (the word is one case of the term, so a match is a match on a broader term) or `"broader"` (the word covers the term and other terms).
  - `group`: `"activities"`, `"touches"`, `"qualities"` or `"artefacts"`.
  - `term`: a term of that group in `terms.json`.
- `broader_terms`: a list of objects, each with exactly four keys:
  - `group` and `term`: a term of `activities`, `touches`, `qualities` or `artefacts`.
  - `broader_group` and `broader_term`: the term it sits under, in one of those four groups or in `categories`.

- `value_aliases`: a list of objects, each with exactly three keys:
  - `word`: text in lower case, as in `aliases`. It is never an allowed value of its fact.
  - `fact`: a fact of `terms.json` whose `values` is `"closed"` and whose allowed values are text. Today only `languages`.
  - `value`: one allowed value of that fact.

The same `word`, `group` and `term` appear together at most once. A word with relation `synonym` points to one term only; a word with relation `broader` may point to several. A term appears at most once as the `group` and `term` of a `broader_terms` entry, and the links hold no cycle. A word of `value_aliases` means its value: a search request may send it in place of the value, and a skill header may not. Version 2 holds six: `typescript` (`ts`), `javascript` (`js`), `golang` (`go`), `c++` (`cpp`), `c#` (`csharp`) and `objective-c` (`objc`).

```
Aliases = {
  version: string, vocabulary: string, about: string,
  aliases: [ { word: string, relation: "synonym" | "narrower" | "broader",
               group: "activities" | "touches" | "qualities" | "artefacts", term: string } ],
  broader_terms: [ { group: string, term: string, broader_group: string, broader_term: string } ],
  value_aliases: [ { word: string, fact: string, value: string } ]
}
```

### 5.3 What `rex skills check` reads from them

These are the checks of section 10 that depend on the vocabulary, stated with these files.

- `metadata.kind` is present, is one text value, and is a term of `kinds`.
- `metadata.category` is present, is one text value, and is a term of `categories`.
- The keys of `metadata.claims` are only `facts`, `activity`, `touches`, `quality` and `artefact`. Each of the last four is a list, and every item is a term of the group whose `header` names that field.
- Every key of `claims.facts` is a fact whose `claimable` is `true`. Its value has the type of section 2.1. Every value of a closed fact is in `allowed`, and every value of an open fact with a `pattern` matches it.
- Every key of `metadata.scope` is a term of `scope_keys`. Its value is a list of at least one value, each in the `allowed` list of the key's fact.
- `metadata.not_when` is present and not empty when `metadata.scope` holds at least one key.
- A `files` item of role `language` or `code` names a `language` from the `allowed` list of the fact `languages`.
- Each `open_when` item has `when` and either `skill`, `file`, or both (Part 4, choice 1), and each target exists.

The other checks of section 10 (files listed and present, `requires` and `replaces` targets, the intent search and the wording of `description` and `when_to_use`) and the fixed values of section 7.1 (`placement` is `full` or `open-when`; a file's `role` is `topic`, `language` or `code`) do not read these files.

---

## Part 6. Where each row went in WP2

WP2 wrote the `metadata` map into the header of each skill in `plugin/library/`. This part says where each row of `plugin/assign/table.json`, each of its 33 criteria (C01 to C33 of Part 1.1) and each of its 7 `files` items (F1 to F7 of Part 1.3) went in those headers. `table.json` itself is unchanged: the mod still reads it until WP7, and section 9.6 decides which rows it keeps after that. Each skill's claims are the union of the terms of all its rows, as Part 1.2 says.

### 6.1 Rows

"wc" below means `scope: { writes_code: [true] }` with `not_when: "The task writes no code."`. "Rulebooks" means `rex-code-philosophy`, `rex-code-commenting`, `rex-code-ergonomics` and `rex-code-error-writing`. Every `priority` is dropped (Q13).

| Row | Criteria | Header of | Claims | `placement` | `scope` and `not_when` | `files` and `open_when` |
| --- | --- | --- | --- | --- | --- | --- |
| `code-rulebooks` | C01 | the four rulebooks; also the new `rex-code-essentials` | `rex-code-essentials`: `facts: { writes_code: true }`. The four rulebooks: none since gate A | `full` | none: the claim holds the fact | the rulebooks: none from the table. `rex-code-essentials` names the four rulebooks as `open_when` items |
| `gap-skill` | C02 | `rex-gap-ts` | `facts: { writes_code: true, languages: [ts] }` | `full` | none: the claim holds the facts | none from the table |
| `tdd` | C03, C04 | `rex-code-tdd` | `activity: [build, fix]` | `full` | wc | none from the table |
| `unit-tests` | C05 | `rex-code-tests-unit-testing` | `artefact: [unit-test]` | `full` | wc | `open_when` item for `rex-code-tdd/mocking.md` (F2) |
| `contract-tests` | C06 | `rex-code-tests-contract-seams` | `artefact: [contract-test]` | `full` | wc | `open_when` item for `rex-code-tdd/mocking.md` (F2) |
| `fitness-tests` | C07, C08 | `rex-code-tests-fitness-functions` | `artefact: [fitness-test]` | `full` | wc | `open_when` item for `rex-code-tdd/mocking.md` (F2) |
| `integration-tests` | C09 | `rex-code-tests-integration-testing` | `artefact: [integration-test]` | `full` | wc | `files` item `real-funds.md` (F1); `open_when` item for `rex-code-tdd/mocking.md` (F2) |
| `boundary-tests` | C10, C11, C12 | `rex-code-tdd` | none since gate A; WP2 gave `touches: [system-boundary]` | the row's `open-when` stays with the file: `mocking.md` is a topic file, and a topic file is always open-when (Q3). The skill keeps `full` from row `tdd` | wc, the same as row `tdd` | `files` item `mocking.md` with its case (F2) |
| `folders` | C13, C14 | `rex-code-folder-management` | `touches: [file-layout]` | `full` | wc | `files` item `move.md` (F3) |
| `dependencies` | C15 | `rex-code-external-libraries` | `touches: [dependencies]` | `full` | wc | none from the table |
| `failure-policy` | C16, C17, C18, C19 | `rex-code-failure-policy` | `touches: [outside-input, long-running, retry]` | `full` | wc | `files` item `failure-table.md` (F4) |
| `design` | C20 | `rex-code-design-sketch`, `rex-code-validation-plan` | `activity: [design]` | `full` | `writes_code: [true]` since gate A, each with its own `not_when` line; WP2 gave none (scope `any`) | none from the table |
| `spike` | C21 | `rex-code-spike` | `touches: [unproven-approach]` | `full` | `writes_code: [true]` since gate A, with its own `not_when` line; WP2 gave none (scope `any`) | none from the table |
| `smells` | C22, C23 | `rex-code-smells` | `activity: [audit]`; WP2 also gave `touches: [legacy-code]`, which gate A removed | `full` | wc | `open_when` items for `rex-code-folder-management/audit.md` (F5) and `rex-code-failure-policy/audit.md` (F6) |
| `architecture` | C24 | `rex-code-improve-codebase-architecture` | `touches: [architecture]` | `full` | wc | none from the table |
| `priorities` | C25 | `rex-code-priorities` | `touches: [trade-offs]` | `full` | none: scope `any` | none from the table |
| `latency` | C26 | `rex-code-latency` | `quality: [latency]` | `full` | `scope: { writes_code: [true], languages: [rust], latency_scale: [ns_us] }`, `not_when: "A hot path measured in milliseconds, or one on a host you do not control."` | `files` item `measure.md` (F7) |
| `pr-review` | C27 | not carried in WP2 | none | none | none | none |
| `diagrams` | C28 | `rex-mermaid-diagrams` | `artefact: [diagram]` | `full` | none: scope `any` | none from the table |
| `skill-text` | C29 | `rex-skill-language` | `artefact: [instructions]` | `full` | none: scope `any` | none from the table |
| `briefs` | C30, C31 | `rex-agent-prompting` | `touches: [sub-agent]`, `artefact: [brief]` | `full` | none: scope `any` | none from the table |
| `plan-review` | C32 | `rex-codex-adversarial-review` | `facts: { role: plan-reviewer }` | `full` | none: scope `any` | none from the table |
| `plan-writing` | C33 | `rex-code-design-sketch`, `rex-code-validation-plan` | `artefact: [plan]` | `full` | `writes_code: [true]` since gate A, as row `design`; WP2 gave none (scope `any`) | none from the table |

The row `pr-review` is an open item. Its only skill, `rex-pr-review`, is a listed skill in `plugin/skills/`, and WP2 writes no header outside `plugin/library/`. Q7 lets a listed skill opt in to search with a `metadata` map. When it does, its map gets kind `method`, category `review`, `claims.activity: [review]` and `placement: full`, with no scope (Part 4, choice 11). Until then, the table row is its only record.

### 6.2 Criteria

| ID | Criterion | Header field and value | Skill |
| --- | --- | --- | --- |
| C01 | `writes-code` | `claims.facts.writes_code: true` | `rex-code-essentials`; the four rulebooks until gate A |
| C02 | `language-not-rust` | `claims.facts: { writes_code: true, languages: [ts] }` | `rex-gap-ts` |
| C03 | `builds-feature` | `claims.activity: build` | `rex-code-tdd` |
| C04 | `fixes-bug` | `claims.activity: fix` | `rex-code-tdd` |
| C05 | `unit-test` | `claims.artefact: unit-test` | `rex-code-tests-unit-testing` |
| C06 | `contract-test` | `claims.artefact: contract-test` | `rex-code-tests-contract-seams` |
| C07 | `invariant` | `claims.artefact: fitness-test` | `rex-code-tests-fitness-functions` |
| C08 | `property-test` | `claims.artefact: fitness-test` | `rex-code-tests-fitness-functions` |
| C09 | `real-world` | `claims.artefact: integration-test` | `rex-code-tests-integration-testing` |
| C10 | `outside-system` | `claims.touches: system-boundary` | none since gate A; `rex-code-tdd` in WP2 |
| C11 | `time-or-randomness` | `claims.touches: system-boundary` | none since gate A; `rex-code-tdd` in WP2 |
| C12 | `file-system` | `claims.touches: system-boundary` | none since gate A; `rex-code-tdd` in WP2 |
| C13 | `creates-or-splits` | `claims.touches: file-layout` | `rex-code-folder-management` |
| C14 | `moves` | `claims.touches: file-layout`, and the case of `files` item `move.md` | `rex-code-folder-management` |
| C15 | `adds-or-upgrades` | `claims.touches: dependencies` | `rex-code-external-libraries` |
| C16 | `outside-input` | `claims.touches: outside-input` | `rex-code-failure-policy` |
| C17 | `long-running-state` | `claims.touches: long-running` | `rex-code-failure-policy` |
| C18 | `retry-or-reconnect` | `claims.touches: retry` | `rex-code-failure-policy` |
| C19 | `main-loop` | `claims.touches: long-running` | `rex-code-failure-policy` |
| C20 | `design-first` | `claims.activity: design` | `rex-code-design-sketch`, `rex-code-validation-plan` |
| C21 | `unproven` | `claims.touches: unproven-approach` | `rex-code-spike` |
| C22 | `refactor-unfamiliar` | `claims.touches: legacy-code` | none since gate A; `rex-code-smells` in WP2 |
| C23 | `audit` | `claims.activity: audit` | `rex-code-smells` |
| C24 | `architecture-friction` | `claims.touches: architecture` | `rex-code-improve-codebase-architecture` |
| C25 | `ranked-decision` | `claims.touches: trade-offs` | `rex-code-priorities` |
| C26 | `hot-path` | `claims.quality: latency`, with the scope and `not_when` of row `latency` | `rex-code-latency` |
| C27 | `reviews-others` | not carried in WP2: open item for Q7 (section 6.1) | `rex-pr-review`, in `plugin/skills/` |
| C28 | `draws-diagram` | `claims.artefact: diagram` | `rex-mermaid-diagrams` |
| C29 | `writes-instructions` | `claims.artefact: instructions` | `rex-skill-language` |
| C30 | `starts-agent` | `claims.touches: sub-agent` | `rex-agent-prompting` |
| C31 | `writes-brief` | `claims.artefact: brief` | `rex-agent-prompting` |
| C32 | `plan-reviewer` | `claims.facts.role: plan-reviewer` | `rex-codex-adversarial-review` |
| C33 | `writes-code-plan` | `claims.artefact: plan` | `rex-code-design-sketch`, `rex-code-validation-plan` |

### 6.3 Files items

Each case is reworded from the "it ..." form of the table into the "you ..." form of section 7.1. The meaning is unchanged.

| ID | File | `files` item, in the header of its own skill | `open_when` items in other headers |
| --- | --- | --- | --- |
| F1 | `rex-code-tests-integration-testing/real-funds.md` | `{ path: real-funds.md, role: topic, when: "a test moves real funds" }` | none |
| F2 | `rex-code-tdd/mocking.md` | `{ path: mocking.md, role: topic, when: "a test crosses to an outside API or database, depends on time or randomness, or crosses the file system" }`. The table's case is `null`, so the case joins the three criteria texts of row `boundary-tests` | `{ skill: rex-code-tdd, file: mocking.md, when: <the same case> }` in each of the four test-rule skills, so that a test task reaches the file without the TDD method |
| F3 | `rex-code-folder-management/move.md` | `{ path: move.md, role: topic, when: "you move a file, module or folder" }` | none |
| F4 | `rex-code-failure-policy/failure-table.md` | `{ path: failure-table.md, role: topic, when: "you plan a feature with outside input or long-running state and write its failure table" }`. The table's "such a feature" is named from the row's `work` | none |
| F5 | `rex-code-folder-management/audit.md` | `{ path: audit.md, role: topic, when: "you audit code" }` | `{ skill: rex-code-folder-management, file: audit.md, when: "you audit code" }` in `rex-code-smells` |
| F6 | `rex-code-failure-policy/audit.md` | `{ path: audit.md, role: topic, when: "you audit code" }` | `{ skill: rex-code-failure-policy, file: audit.md, when: "you audit code" }` in `rex-code-smells` |
| F7 | `rex-code-latency/measure.md` | `{ path: measure.md, role: topic, when: "you measure the hot path" }` | none |

### 6.4 Sibling files that the table does not name

The library holds 75 sibling files: the 7 above and 68 more. Rule M9 makes each header list every file of its folder, so each one has a `files` item. Their roles:

- **Code, `rust` (14 files).** Each `-code.md` file of `rex-code-latency`, the code twin of the topic file beside it.
- **Topic (61 files, the 7 above among them).** Every other file. No file of the library has role `language`.
  - The `examples.md` of `rex-code-ergonomics`, `rex-code-error-writing`, `rex-code-tests-contract-seams`, `rex-code-tests-fitness-functions` and `rex-code-tests-integration-testing` hold the Rust code of their skills' rules. They are topic files, not language files. Each of these skills is a Rust rulebook with one page of examples, not a general skill with one file for each language, as the `rust.md` and `go.md` of section 7.1 are. Each `when` is the read case that its skill's body gives the file, so the file stays open-when, as it is today. By D17, a language file would go in full with its skill on each Rust task. Gate B can show whether that helps. Change the role only after that.
  - `rex-code-commenting/examples.md` and `rex-code-validation-plan/examples.md` hold some Rust, but their subject is the wording of a comment and the shape of a trace in a plan.
  - `rex-gap-ts/setup.md`: its skill reads it only when a task changes `tsconfig.json`, lint settings or a package.

A topic file outside the table carries the case that its skill's router text gives it, in the "you ..." form, except in `rex-code-latency`. Its 18 topic files and 14 code twins take 46 words of rule M13 with no case, and the whole map takes 94 of the 120 words with one case. So only `measure.md` carries a case (F7), and the router table of its `SKILL.md` keeps the cases of the others, as section 8.5 expects. Its header also gives `max_topics: 2`, from the rule "Max 2 topic files" of that `SKILL.md`.

### 6.5 Choices WP2 made

1. **The rulebooks kept their row in WP2, and lost its claim after gate A.** WP2 gave the four rulebooks `writes_code: true` with `placement: full`, as row `code-rulebooks` gives today, so search returned all five rule skills for a code task, the digest fourth by name. Since gate A only the digest `rex-code-essentials` makes that claim, and it names the four as `open_when` items, so search returns the digest alone. The digest is not in `table.json`, so inside a job no task gets it yet.
2. **No language scope on general rules.** No rulebook and not the digest has `scope.languages` (section 7.1). `rex-gap-ts` claims `languages: [ts]` under `claims.facts`, so lookup A finds it, and it has no scope.
3. **`rex-code-latency` claims only `quality: latency`.** Part 3 names the touches `threads` and `allocation` as a coarse filter for its topic files. A skill-level claim of them would return the skill to any task that touches threads, which no row of the table does. Section 7.1 gives a `files` item no `touches` field, so WP4 must say where that filter is declared.
4. **The scope `writes_code` of a row is kept even where it is narrow.** Row `smells` has scope `code`, so an audit that writes no code does not reach `rex-code-smells`, today or now.
5. **Cross-references in the text of a skill are not `open_when` items.** For example, the step table of `rex-code-improve-codebase-architecture` names `rex-code-folder-management/audit.md`. The table does not, so WP2 did not declare it. The metadata rules that WP8 adds to `rex-skill-language` can decide whether such references must be declared.

### 6.6 Open items

- **Q7:** row `pr-review` (C27), section 6.1.
- **Text that says the library holds 24 skills.** The library holds 25 now. `plugin/skills/orchestration-builder/SKILL.md`, `plugin/skills/orchestration-builder/stages/1-skills/stage.md`, `plugin/skills/rex-rules/SKILL.md` and `README.md` still say 24. They are outside WP2 and belong to the skill text of WP8 (section 9.9).
- **No map is not a finding.** `rex skills check` gives no finding for a library skill whose header has no `metadata` map. A fitness test that each skill of `plugin/library/` has one would keep the 25 maps from being lost.
- **No term was missing.** Every claim, scope key, kind and category of the 25 headers is a term of `plugin/vocab/terms.json` version 1, and of version 2, which removed no term.

---

## Part 7. Version 2

Gate A (`docs/skill-library-gate-a.md`, part 1) found skills that search gave to tasks they do not fit, because the meaning of a term said more than the skills that claim it cover. Version 2 narrows six meanings. It removes no term and adds none. Search matches a term, never its meaning, so a new meaning changes no answer by itself: it changes what a person or an agent who fills a request form ticks.

| Term | Version 1 | Version 2 |
| --- | --- | --- |
| `touches:file-layout` | Creates, splits or moves a file, module or folder. | Creates, splits or moves a code file, module or code folder, and decides where it goes. A folder of skill text or documents is not code. |
| `touches:sub-agent` | Starts a sub-agent. | Starts a sub-agent of the job and writes its brief. |
| `touches:architecture` | Module grouping, refactor candidates and architecture friction. | Finds friction or refactor candidates in the module structure of existing code. |
| `artefacts:unit-test` | A unit test of a pure function. | An inline unit test of a pure function, kept only until a contract or integration test covers the same code. |
| `touches:legacy-code` | Changes code that is old or that the agent does not know. | Refactors code that is old or that the agent does not know. |
| `touches:system-boundary` | A test crosses to an outside API or database, the file system, time or randomness. | A test replaces an outside API or database, the file system, time or randomness with a stand-in. |

No library skill claims `legacy-code` or `system-boundary` since gate A. Both stay, with the case that their words name, so that a later skill can claim them.

Version 2 also adds `value_aliases` to `aliases.json` (Part 5.2), so that a request with `languages: ["typescript"]` finds `rex-gap-ts`.
