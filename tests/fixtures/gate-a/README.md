# Gate A inputs and scripts

This folder holds the inputs and the measuring scripts of gate A of the skill library spec (sections 8.2 to 8.4, 9.1, 10, 11.1 and 12). The scripts call only the binary built in this worktree, `target/debug/rex`. Run `cargo build` first.

Every script is TypeScript with node built-ins only. Node 24 runs it directly: `node tests/fixtures/gate-a/<script>.ts`. Run each command from the repo root. Each script prints numbered lines, so a result can be quoted by its number, and each line gives the count that it rests on. The scripts give the same output on a second run. The one exception is `latency-check.ts --time`, which prints wall time.

Type check: `tsc -p tests/fixtures/gate-a/tsconfig.json`. It prints nothing when it passes.

## Parties and what each one may read

| Party | Writes | Must not read before writing |
| --- | --- | --- |
| Form writers (blind, two runs) | One folder of request forms each | `old-table.json`, `header-quotes.json`, labels, results, other runs |
| Reference writer | One folder of reference forms | Labels, results, form runs |
| Label writer | `labels.json` | Forms, results, `old-table.json` |
| Judge | `judgments.json` | Nothing is withheld |

`tasks.json` is the only task text that a form writer or label writer needs. It was written by script before `old-table.json` and `header-quotes.json` existed (`step-c-record.txt` gives the times and the sha256 values).

## Files

| File | What it holds |
| --- | --- |
| `tasks.json` | One item per row of the WORK BREAKDOWN table of seven jobs: `id` (`<job>-<task>`), `job`, `task`, `row` (`task`, or `role` for a role with no task, such as `reviewer`), and `outcome`, `evidence`, `owns` verbatim. |
| `old-table.json` | For each item, the skills that the old table gave it: `skills` and `open_when` (names in code spans), and both cells verbatim. |
| `header-quotes.json` | Items whose own text holds an intent phrase of a library skill, or a claim term, kind or category of two or more words, such as `sub-agent` or `coding-standards`. One-word terms such as `build` or `latency` are everyday words: they are under `single_word` and do not put an item in `items`. The hit-rate scorer reports again without the items of `items`. |
| `step-c-record.txt` | The time-ordered record of the extraction. |
| `latency/` | The two forms of spec 8.3, written by hand: a Rust matching loop and a Go service in milliseconds. They are not task forms. |
| `steps-e-h-output.txt` | The output of `steps-e-h.ts` at the base commit. |

The filler pool and every full search answer live outside the repo, in `/Users/shaun/Code/DEVELOPMENT/gate-a-pool/`. The pool is outside text that nobody reviewed: only `rex skills search` and `rex skills check --pool` read it. Full answers hold pool text, so they go to `gate-a-pool/raw/`. Results in the repo name a pool item by skill, library, tier and rank only. The library name of a folder is its base name: `library` for `plugin/library`, `skills` for the pool.

## Form file format

One file per item, named `<id>.json`, holding one request form: the `requests` item of spec 8.2.

```json
{ "id": "0h0962klnd-T1",
  "facts": { "writes_code": true, "languages": ["rust"] },
  "activity": "build",
  "touches": ["file-layout"],
  "must_have": [],
  "artefacts": ["integration-test"],
  "needs": ["at most three short free-text needs"],
  "evidence": [ { "term": "file-layout", "quote": "words copied from the task" } ] }
```

- `id` is the `id` of the item in `tasks.json`.
- Every field except `needs` and `evidence` takes only terms of `plugin/vocab/terms.json`. `activity` is one term; the other term fields are lists.
- Each term of `activity`, `touches`, `must_have` and `artefacts` needs one `evidence` entry whose `quote` is copied from the item's Outcome, Evidence or Owns text. A fact may have one too.
- Leave out `repo_has` and `repo_dependencies`: `fill-facts.ts` fills them from the repo, as the mod will (spec 9.1).

## Labels file format

```json
{ "labeller": "who wrote it",
  "items": [
    { "id": "0h0962klnd-T1",
      "skills": [ { "skill": "rex-code-tdd", "lookup": "B" },
                  { "skill": "rex-code-essentials", "lookup": "A" } ],
      "needs": [ { "need": "the free-text need in the labeller's words",
                   "skills": [ { "skill": "rex-code-spike" },
                               { "skill": "some-pool-skill", "library": "skills" } ] },
                 { "need": "a need that no skill serves", "skills": [] } ] } ] }
```

- `skills`: each library skill that is right for the item, with the lookup that should find it: `A` (rules on facts), `B` (declared terms) or `C` (keyword search on a free-text need). `library` may be given; it defaults to `library`.
- `needs`: each free-text need of the item, with every skill (library or pool) that serves it. A need is hit when any of its skills is in the top five. A need with an empty `skills` list is counted apart and never scored.
- A right skill is hit when it is at rank 5 or better on the first page of the item's answer. Rank is the place in the answer, over all tiers.

## Judgments file format

```json
{ "judge": "who judged",
  "items": [
    { "id": "0h0962klnd-T1",
      "assignments": [ { "skill": "rex-code-tdd", "library": "library", "tier": "B",
                         "verdict": "right", "reason": "one sentence" } ],
      "forms": [ { "run": "run1", "field": "touches", "verdict": "wrong", "reason": "one sentence" } ] } ] }
```

- `assignments`: one verdict, `right`, `wrong` or `doubtful`, for each tier A or B item of the item's answer. `score-hits.ts --judgments` lists each one that is not `right` and each one with no verdict.
- `forms`: optional verdicts on fields of a form run, for the form test. No script reads them; `compare-forms.ts` gives the counts they are checked against.

## Commands

Paths in angle brackets are yours. `--pool` defaults to `/Users/shaun/Code/DEVELOPMENT/gate-a-pool/terminalskills/skills` and `--raw` to `/Users/shaun/Code/DEVELOPMENT/gate-a-pool/raw`.

| Step | Command |
| --- | --- |
| Task texts (done) | `node tests/fixtures/gate-a/extract-tasks.ts` (`--sample` parses the format job 0ganyb6d6v only and writes nothing) |
| Old table (done) | `node tests/fixtures/gate-a/extract-old-table.ts` |
| Header quotes (done) | `node tests/fixtures/gate-a/header-quotes.ts` |
| Check forms | `node tests/fixtures/gate-a/validate-form.ts <forms folder or file>...` (exit 1 on a format or vocabulary finding) |
| Fill repo facts | `node tests/fixtures/gate-a/fill-facts.ts --forms <forms folder> --out <filled folder>` |
| Search | `node tests/fixtures/gate-a/run-search.ts --forms <filled folder> --out <results folder> --label <run name> --all-pages yes` (`--limit 10` is the default; `--library-only yes` leaves out the pool) |
| Repeat check | `node tests/fixtures/gate-a/repeat-check.ts --forms <filled folder>` |
| Pool invariance | `node tests/fixtures/gate-a/pool-invariance.ts --forms <filled folder>` |
| Hit rate | `node tests/fixtures/gate-a/score-hits.ts --labels <labels.json> --results <results folder> [--judgments <judgments.json>] [--top 5]` |
| Form comparison | `node tests/fixtures/gate-a/compare-forms.ts --run <run1 folder> --run <run2 folder> --reference <reference folder>` |
| Answer size | `node tests/fixtures/gate-a/answer-size.ts --forms <filled folder>` |
| Intent check (e) | `node tests/fixtures/gate-a/intent-check.ts` |
| Latency example (f) | `node tests/fixtures/gate-a/latency-check.ts` and, for wall time, `node tests/fixtures/gate-a/latency-check.ts --time 10` |
| Ratio setting (g) | `node tests/fixtures/gate-a/ratio-check.ts` |
| Size estimator proof (h) | `node tests/fixtures/gate-a/size-proof.ts` |
| Steps (e) to (h) at once | `node tests/fixtures/gate-a/steps-e-h.ts` |

`size-proof.ts` and `answer-size.ts` share `size-lib.ts`: a copy of the binary's token estimate and page shortening at the base commit, and a rebuild of the full page from one-item pages. The proof shortens each rebuilt page with the copy and checks that it gives the binary's page byte for byte. When the binary's page module changes, run the proof again before you trust a size.
