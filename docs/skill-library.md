# Make a folder of skills searchable

This guide shows how to make a folder of skills in your own repo searchable in the same way as the SuperRex skill library. SuperRex's library is the worked example. The steps are:

1. Give each skill a `metadata` map in its header.
2. Use only terms of the shared vocabulary in that map.
3. Run `rex skills check` before each publish.
4. Serve the folder with `rex skills serve`, so that any MCP client can call `head`, `search` and `get`.
5. Test your headers with `rex skills search`, and publish a static index with `rex skills index` if you want one.

**You need a `rex` that has the new commands.** Run `rex skills --help`. If it lists `serve`, `search` and `index`, your `rex` is new enough. A binary of this release still prints `rex 0.14.0`, so the version number does not tell you.

## 1. How SuperRex lays out its library

| Path | What it holds |
| --- | --- |
| `plugin/library/` | The library: 25 skill folders. Each folder has a `SKILL.md`, and some have other files. |
| `plugin/vocab/terms.json` | The shared vocabulary, version `2`: each term that a `metadata` map may use. |
| `plugin/vocab/aliases.json` | The alias table, version `2`: free words that a search maps onto a term, such as `p99` onto `latency`. |
| `plugin/.claude-plugin/plugin.json` | The plugin manifest. Its `mcpServers` entry starts the library server (section 5). |
| `docs/skill-library-vocabulary.md` | Why each term exists, and the exact form of both vocabulary files. |

Copy the same layout in your repo. This guide uses an example repo with one folder of skills and the vocabulary file beside it:

```
team-repo/
  skills/
    py-unit-tests/
      SKILL.md
      fixtures.md
    release-notes/
      SKILL.md
    rex-mermaid-diagrams/        (a copy of SuperRex's plugin/library/rex-mermaid-diagrams)
  vocab/
    terms.json                   (a copy of SuperRex's plugin/vocab/terms.json)
```

**Where the vocabulary comes from.** `rex skills check` and `rex skills search` use the copy of `terms.json` and `aliases.json` that is built into the `rex` binary. You cannot add terms in your own repo. The `vocab/terms.json` file beside your folder is read only for its top-level `"version"` string. The server reports that string as `vocabulary` in its `head` answer. If the file is not there, `vocabulary` is `null`. If the file is there but has no string `version`, the server does not start. Copy the file, so that `head` tells a client which vocabulary version your headers were checked against.

## 2. The header

A skill is a folder with a `SKILL.md`. The header keeps the three keys that Claude Code reads: `name`, `description` and `when_to_use`. Add one `metadata` map below them. Claude Code accepts the key `metadata` in a skill header, and shows a model only `description` and `when_to_use`. The search reads the `metadata` map itself.

### A real example from SuperRex's library

This is the header of `plugin/library/rex-mermaid-diagrams/SKILL.md`, as it is in the library:

```yaml
---
name: rex-mermaid-diagrams
description: >
  Guide to software diagrams in Mermaid syntax. Types: class, sequence,
  flowchart, ERD, C4, architecture, state, git graph, pie, gantt, and others.
when_to_use: >
  Use when you create, visualize, or document software with diagrams, or
  explain architecture, database design, code structure, or user or
  application flows, or for "diagram", "visualize", "model", "map out", "show
  the flow".
metadata:
  kind: tool
  category: diagrams
  claims:
    artefact: [diagram]
  placement: full
  intents:
    - draw a diagram in a plan or document
    - show a flow as a diagram
    - model a database or a class design
  files:
    - { path: references/advanced-features.md, role: topic, when: "you need themes, styling or layout options" }
    - { path: references/architecture-diagrams.md, role: topic, when: "you draw cloud services or infrastructure" }
    - { path: references/c4-diagrams.md, role: topic, when: "you draw a C4 diagram" }
    - { path: references/class-diagrams.md, role: topic, when: "you draw a class diagram or a domain model" }
    - { path: references/erd-diagrams.md, role: topic, when: "you draw an entity relationship diagram" }
    - { path: references/flowcharts.md, role: topic, when: "you draw a flowchart" }
    - { path: references/sequence-diagrams.md, role: topic, when: "you draw a sequence diagram" }
---
```

It says: this skill is a `tool` in the category `diagrams`. Its author claims that it is for the artefact `diagram`. When it is chosen, it goes into an agent's brief in full. Three phrases say what a caller might ask for. Each of its seven other files is listed, with the case in which an agent opens it.

### An example with a scope

This is `team-repo/skills/py-unit-tests/SKILL.md` of the example repo. It shows the keys that the real example does not use: `scope`, `not_when` and `open_when`.

```markdown
---
name: py-unit-tests
description: Rules for Python unit tests with pytest. One behaviour per test, a name that says it, no network.
when_to_use: Use when you write or change a Python unit test.
metadata:
  kind: rule
  category: testing
  claims:
    activity: [test]
    artefact: [unit-test]
  placement: full
  scope: { languages: [python] }
  not_when: "Tests in a language other than Python."
  intents:
    - write a pytest unit test
    - name a Python test
    - share setup between Python tests
  files:
    - { path: fixtures.md, role: topic, when: "you share setup between tests" }
  open_when:
    - { skill: rex-mermaid-diagrams, file: references/sequence-diagrams.md, when: "you draw how a test calls its fakes" }
---

# Python unit tests

- Test one behaviour in each test.
- Name the test after the behaviour: `test_refund_over_total_is_refused`.
- Do not call the network. Use a fake.
- IF tests share setup, THEN read `fixtures.md`.
```

`team-repo/skills/py-unit-tests/fixtures.md`:

```markdown
# Fixtures

- Put shared setup in a pytest fixture in `conftest.py`.
- Give each fixture the narrowest scope that works.
```

`team-repo/skills/release-notes/SKILL.md` has no `metadata` map. A search finds it by keyword only, on its name and `description`:

```markdown
---
name: release-notes
description: How this team writes release notes for a version.
when_to_use: Use when you write release notes, or when the user says "release notes".
---

# Release notes

- One line for each change a user can see.
```

### The keys of the `metadata` map

The Rule column names the check of `rex skills check` that finds a fault in the key. Section 4 lists the checks.

| Key | What to write | Rule |
| --- | --- | --- |
| `kind` | One term of `kinds`. | M1 |
| `category` | One term of `categories`. | M2 |
| `claims` | What the skill is for, in terms only. `facts` is a map, such as `{ writes_code: true, languages: [ts] }`. `activity`, `touches`, `quality` and `artefact` are lists of terms. No other field. | M3 |
| `placement` | `full` or `open-when`: how a caller that chooses the skill gives it to an agent. `full`: the whole `SKILL.md` goes into the agent's prompt. `open-when`: the prompt names the skill and the case, and the agent reads it only in that case. | M4 |
| `scope` | Conditions that code checks. Each key is a scope key, and each value is a list of one or more allowed values, such as `{ languages: [python] }`. | M5 |
| `not_when` | One line: when the skill does not apply. Required when `scope` has a key. A search never searches this text. It returns it beside the item. | M6 |
| `intents` | 3 to 5 phrases, none empty. Each phrase is a request that the skill answers. | M7, M14 |
| `files` | One item for each file of the folder except `SKILL.md`: `path` and `role`. The role is `topic` for a file on one subject, `language` for a file of text for one programming language, or `code` for the code examples of one topic file (its code twin, such as `measure-code.md` beside `measure.md`). A `language` or `code` item also has `language`, a language term. `when` (the case in which to open the file) and `intents` (1 to 5 phrases) are optional. | M8, M9 |
| `open_when` | What an agent may open, and when. `{ skill, when }` names another skill, `{ file, when }` a file of this skill, and `{ skill, file, when }` a file of another skill. `when` is required, and the target must exist. | M10 |
| `requires`, `replaces` | Lists of names of skills that exist. A search returns a required skill with the skill that requires it, and drops a replaced skill with the reason `replaced by <skill>`. | M11 |
| `max_topics` | A positive whole number: the most topic files that one search returns for this skill. Left out, it is 5. | M12 |
| (whole map) | The keys above hold at most 120 words together. Keys and file paths do not count. | M13 |

Other keys under `metadata`, such as `author`, are ignored.

**Write what the skill is for, not what it mentions.** A claim is the author's statement that the skill serves that term. A word in the text is not a claim. A rule for code in any language must not have `scope.languages`, or a task in another language would lose it.

**When the body sends an agent to another skill**, declare that as an `open_when` item with the same case.

`rex skills check` does not check the rules of these last two paragraphs. Make sure of them by eye.

## 3. The shared vocabulary, version 2

Every term of a `metadata` map comes from `plugin/vocab/terms.json`. These are all of them.

| Group | Header field | Terms |
| --- | --- | --- |
| `kinds` | `kind` | `rule`, `method`, `tool`, `reference` |
| `categories` | `category` | `coding-standards`, `testing`, `design`, `code-health`, `performance`, `review`, `diagrams`, `agent-instructions` |
| `facts` | `claims.facts` | `writes_code` (true or false), `languages` (list), `role` (text), `repo_has` (list), `repo_dependencies` (list), `latency_scale` (`ns_us` or `ms`; a scope only, never a claim) |
| `scope_keys` | `scope` | `writes_code`, `languages`, `latency_scale` |
| `activities` | `claims.activity` | `build`, `fix`, `refactor`, `design`, `review`, `audit`, `test`, `document` |
| `touches` | `claims.touches` | `system-boundary`, `file-layout`, `dependencies`, `outside-input`, `long-running`, `retry`, `unproven-approach`, `legacy-code`, `architecture`, `trade-offs`, `sub-agent`, `threads`, `allocation` |
| `qualities` | `claims.quality` | `latency`, `readability` |
| `artefacts` | `claims.artefact` | `unit-test`, `contract-test`, `fitness-test`, `integration-test`, `diagram`, `plan`, `instructions`, `brief` |

The languages are the names of `rex codebase --only`: `rust`, `go`, `c`, `cpp`, `python`, `js`, `ts`, `shell`, `java`, `kotlin`, `scala`, `csharp`, `swift`, `objc`, `dart`, `php`, `ruby`, `lua`, `haskell`, `elixir`, `r`, `sql`, `zig`, `perl`. A search request may also give a common full name, `typescript`, `javascript`, `golang`, `c++`, `c#` or `objective-c`, and the search reads it as `ts`, `js`, `go`, `cpp`, `csharp` or `objc`. A header must use the short name.

A wrong term is a finding, and the finding lists the allowed terms.

## 4. Check before each publish

Run `rex skills check` on the folder after each change to a header or to the files of a skill. Repeat until each skill prints `ok`.

```console
$ rex skills check skills
ok skills/py-unit-tests
ok skills/release-notes
ok skills/rex-mermaid-diagrams
warning: intent check skipped; give --pool <folder> of other skills to check that each intent finds its skill
```

The command exits 1 when there is a finding. Here is a copy of `py-unit-tests` with four faults: `kind: rules`, `artefact: [unit-tests]`, no `not_when`, and an extra file `notes.md` that `files` does not list. The check runs on that skill folder alone.

```console
$ rex skills check skills/py-unit-tests
skills/py-unit-tests/SKILL.md: M1: kind `rules` is not one of rule, method, tool, reference
skills/py-unit-tests/SKILL.md: M3: claims.artefact holds `unit-tests`, which is not one of unit-test, contract-test, fitness-test, integration-test, diagram, plan, instructions, brief
skills/py-unit-tests/SKILL.md: M6: not_when is missing; a skill with a scope needs one line that says when not to use it
skills/py-unit-tests/SKILL.md: M9: `notes.md` is in the skill folder, but files does not list it
warning: intent check skipped; give --pool <folder> of other skills to check that each intent finds its skill
error: 4 findings in 1 of 1 skills
```

The rules:

| Rule | Passes when |
| --- | --- |
| M1 | `metadata.kind` is a term of `kinds`. |
| M2 | `metadata.category` is a term of `categories`. |
| M3 | `metadata.claims` holds only `facts`, `activity`, `touches`, `quality` and `artefact`, and each claim is a term that a skill may claim, with a value of the right type. |
| M4 | `metadata.placement` is `full` or `open-when`. |
| M5 | Each key of `metadata.scope` is a scope key, with a list of at least one allowed value. |
| M6 | `metadata.not_when` is one line, and is there when `scope` has a key. |
| M7 | `metadata.intents` holds 3 to 5 pieces of text, none empty. |
| M8 | Each `files` item has a `path` other than `SKILL.md`, a `role` of `topic`, `language` or `code`, and a language term when the role is `language` or `code`. Its own `intents`, if any, are 1 to 5 pieces of text. |
| M9 | `files` lists every file of the folder except `SKILL.md`, and every file that it lists is in the folder. Hidden files such as `.DS_Store` do not count. |
| M10 | Each `open_when` item has `when`, and its skill or file exists. |
| M11 | Each skill that `requires` or `replaces` names exists. |
| M12 | `max_topics` is a positive whole number. |
| M13 | The map holds at most 120 words, not counting keys or file paths. |
| M14 | Each phrase of `intents` finds its own skill in the top five of a keyword search. Runs only with `--pool`. |

Rules M1 to M13 run only on a map that holds at least one of the keys of section 2. A header with no `metadata` map gets no finding, and neither does a map that holds only other keys. A map with `category` and no `kind` gets `M1: kind is missing; give one of rule, method, tool, reference`.

**The intent check, M14.** Give `--pool <folder>` with a folder of other skills. The check searches your skills and the pool together, and each intent phrase must find its own skill in the top five. Give `--pool` more than once for several folders. Among a few dozen skills the check proves little. The command says so when the search holds fewer than 1,000 skills:

```console
$ rex skills check skills --pool /path/to/super-rex/plugin/library
warning: the intent check searched 28 skills; a pool of 1000 or more is needed for a real check
ok skills/py-unit-tests
ok skills/release-notes
ok skills/rex-mermaid-diagrams
```

This release ships no pool of 1,000 skills. Use any large folder of other skills. The check only searches the name, description and intents of each pool skill. It never runs their text. A phrase that fails gives a line of the form `M14: intent "<phrase>" does not find this skill in the top 5 of <n> skills; it ranks <place>`, or `...; no skill matches it` when the phrase has no word to search, such as `"!!!"`.

The older rules still apply to every skill. H1 to H9 check the header. Among other things: it is closed by a `---` line and is valid YAML, `name` equals the folder name, `description` is not empty and holds no "Use when" text, `when_to_use` starts with `Use when`, the two hold at most 60 words together, and every key is one that Claude Code knows. B1 to B5 check the text: there is text after the header, `SKILL.md` has at most 500 lines, its text after the header has at most 1,500 words, a relative link points to a file of the same skill, and a `<skill>/<file>.md` span names a file that exists. The SuperRex README (https://github.com/coderaidershaun/super-rex) gives each of them in full under "Reference".

## 5. Serve the folder

```console
$ rex skills serve --name team --library skills
```

The server reads one JSON-RPC message for each line on stdin and writes one answer for each line on stdout, as an MCP server over stdio does. It answers `initialize`, `ping`, `tools/list` and `tools/call`, and its tools are `head`, `search` and `get`.

| Option | What it does |
| --- | --- |
| `--library <path>` | A folder of skill folders. Give it more than once for several folders. When two folders hold a skill of the same name, the first folder wins for the whole skill. With no `--library`, the server serves the library built into the binary. |
| `--name <name>` | The name that `head` and each search item report. Left out, it is the base name of the first folder (`skills` here), or `super-rex` for the built-in library. |

The server reads the folders again on each call, so a changed file shows at once. A folder that does not exist stops it:

```console
$ rex skills serve --library nowhere
error: library folder nowhere does not exist or is not a folder
```

### What the three tools take and answer

Each answer is one JSON text in the `content` of the tool result.

| Tool | Takes | Answers |
| --- | --- | --- |
| `head` | Nothing. | `protocol` (`skills/1`), `library` (the name), `version`, `skills` (how many are served) and `vocabulary` (the version string, or `null`). |
| `search` | `requests` (required), `limit`, `cursor`, `kinds`, `categories`, `token_budget`, `keep_hashes`. Section 6 gives the fields. | `index_version` and one result for each request. |
| `get` | `skill` and `file` (both required; `"file": "SKILL.md"` for the entry file). | `skill`, `file`, `sha256` (of the text) and `text`. |

`version` changes when any served file or the vocabulary version changes, and not otherwise. To build it, the server writes one line for each served file, `<skill>/<file>`, a tab and the file's SHA-256, sorts the lines, and adds the line `vocabulary`, a tab and the vocabulary version. It hashes that whole text once: `version` is `sha256:` followed by that hash. Each search answer carries the same value as `index_version`.

A failed call answers with `isError: true` and the text `{"code": "...", "message": "..."}`:

| Code | When | What the caller does |
| --- | --- | --- |
| `not_found` | `get` names a skill or a file that the library does not serve. | Report it. Do not guess another name. |
| `bad_cursor` | The `cursor` of a search cannot be read. | Search again without it. |
| `stale_cursor` | The library changed since the cursor was made. | Search again without it. |
| `bad_arguments` | The arguments do not have the right form, such as an unknown field. | Correct the call. |
| `unreadable` | A file or a folder cannot be read. | Report it. |
| `search_failed` | The keyword search failed. | Report it. |
| `unknown_tool` | The call names no tool of the server. | Correct the call. |

### A session with the example library

Run from `team-repo/`, with the input file `session.jsonl`:

```json
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}
{"jsonrpc":"2.0","method":"notifications/initialized"}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"head","arguments":{}}}
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"get","arguments":{"skill":"py-unit-tests","file":"fixtures.md"}}}
{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"get","arguments":{"skill":"py-unit-tests","file":"missing.md"}}}
```

```console
$ rex skills serve --name team --library skills < session.jsonl
{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-06-18","capabilities":{"tools":{}},"serverInfo":{"name":"super-rex-skills","version":"0.14.0"}}}
{"jsonrpc":"2.0","id":2,"result":{"content":[{"type":"text","text":"{\"protocol\":\"skills/1\",\"library\":\"team\",\"version\":\"sha256:fd5bcba6d91ad638e974438ea6bd72358cdd2166ef9dd263383f94d307b9f826\",\"skills\":3,\"vocabulary\":\"2\"}"}],"isError":false}}
{"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"{\"skill\":\"py-unit-tests\",\"file\":\"fixtures.md\",\"sha256\":\"fd80949bce1f9843a74b65de0a40c1a8f6099248d1ae27b6f335de6e931abcbb\",\"text\":\"# Fixtures\\n\\n- Put shared setup in a pytest fixture in `conftest.py`.\\n- Give each fixture the narrowest scope that works.\\n\"}"}],"isError":false}}
{"jsonrpc":"2.0","id":4,"result":{"content":[{"type":"text","text":"{\"code\":\"not_found\",\"message\":\"no file \\\"missing.md\\\" in skill \\\"py-unit-tests\\\"\"}"}],"isError":true}}
```

The notification on line 2 gets no answer. The `sha256` of `get` is the SHA-256 of `fixtures.md` on disk, so a client can check what it got.

### How SuperRex starts its own server

The SuperRex plugin declares the server in `plugin/.claude-plugin/plugin.json`, so Claude Code starts it with each session:

```json
"mcpServers": {
  "library": {
    "command": "rex",
    "args": ["skills", "serve", "--name", "super-rex", "--library", "${CLAUDE_PLUGIN_ROOT}/library"]
  }
}
```

Claude Code puts the plugin's folder in place of `${CLAUDE_PLUGIN_ROOT}`, so the server serves the installed plugin's `library/` folder, with `vocab/terms.json` beside it. Another MCP client starts the same command for your folder: `rex skills serve --name <name> --library <path>`.

**CAUTION: Claude Code runs the first `rex` on your PATH.** If that `rex` has no `skills serve`, the library server does not start, and Claude Code shows only `status: failed` for it, with no message. Install a `rex` that has `skills serve` first on PATH. `rex doctor --plugin <plugin folder>` starts the server as the manifest declares it and reports this case. Run it inside a Git repo: outside one, `rex doctor` prints `error: no Git repository found at or above ...` and exits 1.

```console
$ rex doctor --plugin /path/to/super-rex/plugin
...
library=unreachable
...
problem=library server: it ended with no answer to head: error: unrecognized subcommand 'serve'
```

With a `rex` that has the command, the same check prints `library=super-rex sha256:<version>`. `rex doctor --plugin` reads only the server key `library` of the manifest.

SuperRex runs work as jobs (a job is one request that SuperRex plans and runs with sub-agents), and its mod (the code of the plugin that runs inside Claude Code) refuses the raw tools of its own server (`mcp__plugin_super-rex_library__head`, `__search` and `__get`) to the agents of a job. Outside a job, a person or another client can call them. This rule is about SuperRex's own server and does not touch a server that you start.

## 6. Test a search with `rex skills search`

`rex skills search --library <path>` reads the arguments of the `search` tool as one JSON object on stdin, and prints the answer as one JSON line. Its answer is the same text as the `search` tool of the server. It has no `--name`, so `library` in each item is the base name of the first folder. The examples below pipe the answer through `python3 -m json.tool` to make it easy to read.

A search runs three lookups for each request form: A matches the facts that a skill claims, B matches the terms that a skill claims, and C searches for keywords. "The three lookups" below explains each one.

### The request

| Field | What it takes |
| --- | --- |
| `requests` | A list of request forms, one for each task. Required. |
| `limit` | The most items in each result. Left out, 10. |
| `cursor` | The `cursor` of an earlier result. `requests` must then hold that one request, as it was sent. |
| `kinds`, `categories` | Lists of terms. Only skills of these kinds or categories are returned. |
| `token_budget` | The most tokens that one result may cost, 1 or more. Left out, 200 for each item of `limit`: 2,000 for a page of 10. "The size of an answer" below says how a result keeps to it. |
| `keep_hashes` | `true` to keep the `sha256` of every item and file, whatever the size. Left out, `false`. |

Each request form takes:

| Field | What it takes | Lookup |
| --- | --- | --- |
| `id` | A name for the request, such as the task ID. Required. | |
| `facts` | A map of facts: `writes_code`, `languages`, `role`, `repo_has`, `repo_dependencies`, `latency_scale`. | A, and the scope check |
| `activity` | One activity term. | B |
| `touches` | A list of touches terms. | B |
| `must_have` | A list of quality terms. | B |
| `artefacts` | A list of artefact terms. | B |
| `needs` | Free text, at most 3 items. | C |
| `evidence` | A list of `{ "term": ..., "quote": ... }`: the words of the task that justify a term. The words of `needs` and of each quote are the task's own words, which lookup A reads for the name of a tool skill. | A |

A field that the form does not know is refused with `bad_arguments`. A word in a term field that is not a term is not an error. If the alias table maps it onto a term, the search uses that term. If not, it is listed under `unrecognised` and searched by keyword.

### The three lookups

| Tier | Lookup | Finds a skill when |
| --- | --- | --- |
| A | Rules | Every fact that the skill claims in `claims.facts` holds for the request. A skill of kind `tool` also needs evidence: a `repo_has` or `repo_dependencies` fact that holds, or its name in the task's own words (`needs` and the `evidence` quotes). |
| B | Declared matches | The skill claims a term of `activity`, `touches`, `must_have` or `artefacts`. Then code checks its `scope` against the request's facts: met, the skill is returned; not met, it is left out and reported under `gaps` with its `not_when` line; unknown, it is returned with a `confirm` question. |
| C | Keyword search | A `needs` item, or a word that is not a term, matches its name, card or intents. The search uses SQLite FTS5 with BM25 weights 4 (name), 2 (card) and 3 (intents), one query for each need. It keeps at most 5 candidates for each need, each with at least half the score of the best one. That share is provisional; "Measure the keyword search" below shows how to try another. |

Lookups A and B use no word counts of the library, so adding other skills does not change what they find. Every tier C item has a `confirm` question, and the caller answers it before it uses the skill.

### The answer

Each result holds `request`, `items`, `more`, `cursor`, `gaps`, `unrecognised` and `dropped`, and `over_budget` when the result is over its budget (see "The size of an answer").

| Field of an item | What it holds |
| --- | --- |
| `skill`, `library` | The skill and the library that serves it. |
| `kind`, `category` | From the header. Left out for a skill with no `metadata`. |
| `slot` | The need it fills: `rule`, `method`, `tool`, `reference`, `quality:<term>`, `artefact:<term>`, or `keyword` for a skill with no `metadata`. |
| `tier`, `why` | Which lookup found it, and one line that a person can check. |
| `card` | The first 30 words of `description`. Never the skill's text. |
| `not_when`, `confirm` | The skill's limit, and a yes or no question when one is needed. |
| `placement`, `sha256`, `words` | The header's placement, the SHA-256 of `SKILL.md`, and the number of words after its header. |
| `files` | The files of the skill that match the request: topic files ranked by their `when` and `intents` against `needs`, at most `max_topics`, and a code twin or language file only when its language is in `facts.languages`. |
| `open_when` | The skill's own `open_when` items, and its topic files when no `needs` item chose one. A topic file with no `when` gets its path as its case. |

`more` is how many relevant skills come after this page, and `cursor` fetches them (`null` when `more` is 0). `gaps` lists each need that nothing matched, each skill that only its scope left out, and each language that no skill claims under `claims.facts.languages` when `writes_code` is true. `rust` is left out of that last check, because SuperRex's code rules are written for Rust. `dropped` lists skills that another skill replaces or shadows.

**The order of the items.** Every tier A item comes before every tier B item, and every tier B item before every tier C item, on every page. Inside tier A and inside tier B, each need of the request (a set of facts that a skill claims, or a term of the form) gets its first item before any need gets its second. Inside tier C, each `needs` item and each word that is not a term does the same. A need that an item of an earlier tier already fills waits until the other needs have an item. Among the items for one need, a skill of an earlier `--library` folder comes first, then the skill that matches more needs, then a match on the term itself before a match through an alias, then the name. In tier C, the higher keyword score comes first, then the name. A skill that another item requires comes right after that item. So a keyword candidate never pushes a tier A or B item down the page. A library added after yours, whose skills claim nothing that the request asks for, changes no tier A or B item and the rank of none.

**The size of an answer.** Each result has a budget of tokens: `token_budget`, or 200 for each item of `limit`. The estimate counts 2 tokens for each 5 bytes of the result's compact JSON, and 5 tokens for each 8 bytes of a hash of 64 hex digits. If a result is over its budget, the server takes these steps, one at a time, until it fits:

1. It cuts each `why` and each `reason` of a gap before its first `; `, so `declared: artefact unit-test; scope met` becomes `declared: artefact unit-test`.
2. It leaves out the `sha256` of every item and every file. With `"keep_hashes": true`, it skips this step.
3. It leaves out `words`, then `placement`, of every item.
4. It moves the last item to the next page, one at a time, and counts it in `more` and `cursor`.

`files` and `open_when` are never left out. A page always keeps its first item. If that item, with its files and open-when items, and the request's `gaps`, `unrecognised` and `dropped` are still over the budget, the result holds `"over_budget": true`. A result with no items holds it too when its `gaps`, `unrecognised` and `dropped` alone are over the budget, because the size steps only cut each reason to its head and never leave a gap out. Every other result is within its budget, and has no `over_budget` field.

### Example 1: a Python unit test

```console
$ echo '{"requests":[{"id":"T1","facts":{"writes_code":true,"languages":["python"]},"activity":"test","artefacts":["unit-test"]}]}' \
    | rex skills search --library skills | python3 -m json.tool
{
    "index_version": "sha256:fd5bcba6d91ad638e974438ea6bd72358cdd2166ef9dd263383f94d307b9f826",
    "results": [
        {
            "request": "T1",
            "items": [
                {
                    "skill": "py-unit-tests",
                    "library": "skills",
                    "kind": "rule",
                    "category": "testing",
                    "slot": "artefact:unit-test",
                    "tier": "B",
                    "why": "declared: artefact unit-test, activity test; scope met",
                    "card": "Rules for Python unit tests with pytest. One behaviour per test, a name that says it, no network.",
                    "not_when": "Tests in a language other than Python.",
                    "placement": "full",
                    "sha256": "f6828d24a552e35c23348bc48210b4d8b2ce4e1345d2ed1f7fb16a7fd93f7aad",
                    "words": 36,
                    "open_when": [
                        {
                            "skill": "rex-mermaid-diagrams",
                            "file": "references/sequence-diagrams.md",
                            "when": "you draw how a test calls its fakes"
                        },
                        {
                            "file": "fixtures.md",
                            "when": "you share setup between tests"
                        }
                    ]
                }
            ],
            "more": 0,
            "cursor": null,
            "gaps": [
                {
                    "need": "languages:python",
                    "reason": "no skill claims the language python"
                }
            ],
            "unrecognised": [],
            "dropped": []
        }
    ]
}
```

The skill is found by its declared claims (tier B), and its scope `languages: [python]` is met. The gap says that no skill of the library claims Python under `claims.facts`. In SuperRex, the skill that fills such a gap is a gap skill: a skill for one language that covers what Rust gives by default and that language does not. SuperRex ships `rex-gap-ts` for TypeScript, which claims `facts: { writes_code: true, languages: [ts] }`.

### Example 2: the same test in Go

```console
$ echo '{"requests":[{"id":"T2","facts":{"writes_code":true,"languages":["go"]},"artefacts":["unit-test"]}]}' \
    | rex skills search --library skills | python3 -m json.tool
{
    "index_version": "sha256:fd5bcba6d91ad638e974438ea6bd72358cdd2166ef9dd263383f94d307b9f826",
    "results": [
        {
            "request": "T2",
            "items": [],
            "more": 0,
            "cursor": null,
            "gaps": [
                {
                    "need": "artefacts:unit-test",
                    "reason": "scope: languages go not in python",
                    "skill": "py-unit-tests",
                    "library": "skills",
                    "not_when": "Tests in a language other than Python."
                },
                {
                    "need": "languages:go",
                    "reason": "no skill claims the language go"
                }
            ],
            "unrecognised": [],
            "dropped": []
        }
    ]
}
```

The scope is not met, so the skill is not returned. The caller sees why, with the skill's own `not_when` line. If the request gives no `languages`, the scope is unknown: the skill is returned with `"why": "declared: artefact unit-test; scope unknown: languages"` and `"confirm": "Does the task have languages python?"`.

### Example 3: free text and an unknown word

```console
$ echo '{"requests":[{"id":"T3","needs":["write the release notes"],"touches":["flaky-ci"]}]}' \
    | rex skills search --library skills | python3 -m json.tool
{
    "index_version": "sha256:fd5bcba6d91ad638e974438ea6bd72358cdd2166ef9dd263383f94d307b9f826",
    "results": [
        {
            "request": "T3",
            "items": [
                {
                    "skill": "release-notes",
                    "library": "skills",
                    "slot": "keyword",
                    "tier": "C",
                    "why": "keyword: need 1",
                    "card": "How this team writes release notes for a version.",
                    "confirm": "Is release-notes right for need 1?",
                    "sha256": "b8d0518298be684e28bfa06ec9dff7755fa1459dfeca607ed18a8023cec8e040",
                    "words": 13
                }
            ],
            "more": 0,
            "cursor": null,
            "gaps": [
                {
                    "need": "touches:flaky-ci",
                    "reason": "no keyword match"
                }
            ],
            "unrecognised": [
                "touches:flaky-ci"
            ],
            "dropped": []
        }
    ]
}
```

`release-notes` has no `metadata`, so only the keyword search finds it, and the caller must confirm it. `flaky-ci` is no term, so it is listed under `unrecognised`, and its keyword search found nothing.

### Example 4: a limit and a cursor

```console
$ echo '{"requests":[{"id":"T5","facts":{"languages":["python"]},"artefacts":["unit-test"],"needs":["write the release notes"]}],"limit":1}' \
    | rex skills search --library skills | python3 -m json.tool
```

The first page holds `py-unit-tests`, with `"more": 1` and `"cursor": "c1.1.fd5bcba6d91ad638.69944676f9c79a54"`. Send the same request again with that `cursor` to get `release-notes`. With `"limit": 1` the budget is 200 tokens, and `py-unit-tests` with its two open-when items costs 262 tokens even after the steps of the size limit, so the first page also holds `"over_budget": true`. With `"token_budget": 400` the same page holds the item whole, at 316 tokens, and no `over_budget`. A cursor that cannot be read gives `bad_cursor`:

```console
$ echo '{"requests":[{"id":"T3"}],"cursor":"nonsense"}' | rex skills search --library skills
error: the cursor cannot be read: it is not four parts joined by `.`; search again without it
```

### Measure the keyword search

`rex skills search` takes two options for measurement. The `search` tool of the server has neither, and with neither the answer is the same, byte for byte, as an answer of a `rex` that does not have them.

- `--keyword-ratio <share>` replaces the built-in share of 0.5 for this run: a keyword candidate is kept only when its score is at least that share of the best score of its need. It takes a number above 0 and at most 1. The choice of topic files inside a skill keeps the share of 0.5. A cursor binds the share, so a cursor made with one share gives `bad_cursor` with another.
- `--keyword-scores` adds `score` and `best_score` to each tier C item: its keyword score for the need that its `why` names, and the best score of that need. A higher score is a better match. The two fields are added after the size limit, so they never change which items a page holds, and a page with them can be over its budget by their size.

Example 3 again, with the scores:

```console
$ echo '{"requests":[{"id":"T3","needs":["write the release notes"],"touches":["flaky-ci"]}]}' \
    | rex skills search --library skills --keyword-scores | python3 -m json.tool
```

The item of `release-notes` now holds `"score": 2.9464782214535807` and `"best_score": 2.9464782214535807` after its `why`: it is the best match of need 1. The rest of the answer is the same as in example 3.

### SuperRex's own library: the latency reference

`rex-code-latency` claims the quality `latency`, with `scope: { writes_code: [true], languages: [rust], latency_scale: [ns_us] }`. Run from the root of the SuperRex repo:

```console
$ echo '{"requests":[{"id":"T1","facts":{"writes_code":true,"languages":["rust"],"latency_scale":"ns_us"},"must_have":["latency"],"needs":["measure the hot path"]}],"kinds":["reference"]}' \
    | rex skills search --library plugin/library
{"index_version":"sha256:7644ab358e62ca44215c4cc877f5ada11d9c0c62a24b63183e86672efa8a7be9","results":[{"request":"T1","items":[{"skill":"rex-code-latency","library":"library","kind":"reference","category":"performance","slot":"quality:latency","tier":"B","why":"declared: quality latency; scope met","card":"Reference for Rust hotpath latency of nanoseconds to tens of microseconds, on a Linux host you control: measure, then cause, proof, fix and numbers for jitter, caches, allocation, atomics, queues,","not_when":"A hot path measured in milliseconds, or one on a host you do not control.","placement":"full","sha256":"d9b745dc4c0d4a761ecc969f94d71a6b8b9f64793095b67ea6f441706c536e58","words":697,"files":[{"path":"measure.md","role":"topic","when":"you measure the hot path","sha256":"4ad170cd4f7de333c1e874e937153c8fca6608c61f498c1b2c928ec29ee910b1"},{"path":"measure-code.md","role":"code","language":"rust","sha256":"6fa49c8410388f63082ff6368f0194b3647df1cc85d78b68c87bf5708bbff456"}]}],"more":0,"cursor":null,"gaps":[],"unrecognised":[],"dropped":[]}]}
```

The need "measure the hot path" chose the topic file `measure.md` by its `when`, and its Rust code twin `measure-code.md` came with it, because the request's language is `rust`. For a Go service measured in milliseconds, the scope is not met:

```console
$ echo '{"requests":[{"id":"T2","facts":{"writes_code":true,"languages":["go"],"latency_scale":"ms"},"must_have":["latency"]}],"kinds":["reference"]}' \
    | rex skills search --library plugin/library
{"index_version":"sha256:7644ab358e62ca44215c4cc877f5ada11d9c0c62a24b63183e86672efa8a7be9","results":[{"request":"T2","items":[],"more":0,"cursor":null,"gaps":[{"need":"must_have:latency","reason":"scope: languages go not in rust; latency_scale ms not in ns_us","skill":"rex-code-latency","library":"library","not_when":"A hot path measured in milliseconds, or one on a host you do not control."},{"need":"languages:go","reason":"no skill claims the language go"}],"unrecognised":[],"dropped":[]}]}
```

## 7. Publish a static index with `rex skills index`

`rex skills index --library <path>` prints one JSON document for the library. Write it to a file to publish it beside the folder:

```console
$ rex skills index --library skills > index.json
```

The document holds `protocol`, `library`, `vocabulary` and `skills`. Each skill has `name`, `type` (`skill-md`), `description`, `url` (the path of its `SKILL.md` from the folder), `digest` (`sha256:` and the SHA-256 of `SKILL.md`), `library`, `when_to_use`, `card`, `words`, its `metadata` map as the search reads it (left out for a skill with no map), and `files`. Each file has `path`, `url`, `digest` and, when `files` lists it, its `role`, `language` and `when`. Like `rex skills search`, it has no `--name`.

## 8. What this release does not do yet

- `head` takes an `include` argument in its schema, but ignores it. Its answer never holds the vocabulary terms.
- Inside a SuperRex job, the mod still chooses the skills of each agent from its own fixed table, as before, and a search does not change that choice. This does not touch your library or your server.
