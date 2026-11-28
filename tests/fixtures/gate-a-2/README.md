# Second gate A measurement: scripts and raw measures

This folder holds the scripts and the measures that need no labels for the second measurement of gate A of the skill library spec (sections 8.2 to 8.4 and 12). The first measurement is in `tests/fixtures/gate-a/` and `docs/skill-library-gate-a.md`; nothing there is changed. Its forms, tasks, latency forms, labels and judgments are read from that folder.

The scripts call only the binary built in this worktree, `target/debug/rex`, from base commit `54d74f204205463c037361f6f158c52a5d0e9a67` (vocabulary version 2). Run `cargo build` first. The `rex` on `PATH` is an older build and is never used.

Every script is TypeScript with node built-ins only. Node 24 runs it directly: `node tests/fixtures/gate-a-2/<script>.ts`. Run each command from the repo root. Each script prints numbered lines, so a result can be quoted by its number, and each line gives the count that it rests on.

Type check: `tsc -p tests/fixtures/gate-a-2/tsconfig.json`. It prints nothing when it passes.

## Outside text and outside folders

The filler pool, `/Users/shaun/Code/DEVELOPMENT/gate-a-pool/terminalskills/skills` (TerminalSkills/skills at `42289059901710441bfa8081c08c3a9f36b93f9c`, 1,146 skills), is outside text that nobody reviewed. Only `rex skills search` and `rex skills check --pool` read it. The scripts parse answers that hold its text as JSON data only, and stop on the first error.

Full answers hold pool text, so they go outside the repo, to `/Users/shaun/Code/DEVELOPMENT/gate-a-pool/gate-a-2/raw/<label>/`. Results in the repo name a pool item by skill, library, tier and rank only. The library name of a folder is its base name: `library` for `plugin/library`, `skills` for the pool.

| Outside folder | What it holds |
| --- | --- |
| `/Users/shaun/Code/DEVELOPMENT/gate-a-pool/filled/<set>/` | The first measurement's filled forms of `run-1`, `run-2` and `reference`, with the repo facts of `75566e3`. Read, never changed. The primary runs use them. |
| `/Users/shaun/Code/DEVELOPMENT/gate-a-pool/gate-a-2/filled/<set>/` | Filled forms of the fresh sets, made by the judge with `fill-facts.ts --commit 75566e3`. |
| `/Users/shaun/Code/DEVELOPMENT/gate-a-pool/gate-a-2/filled-54d74f2/<set>/` | The second run: the same forms with the repo facts of the base commit. |
| `/Users/shaun/Code/DEVELOPMENT/gate-a-pool/gate-a-2/raw/<label>/` | Full answers of `run-search.ts` and `ratio-sweep.ts`. |
| `/Users/shaun/Code/DEVELOPMENT/gate-a-pool/gate-a-2/tmp/` | The briefs that `repeat-all.ts` extracts to compare with `briefs/`. |

## Files

| File | What it is for |
| --- | --- |
| `lib.ts` | Paths, flag reading, JSON reading and calls of the binary that every script shares. |
| `search-lib.ts` | The search flags (`--limit`, `--keep-hashes`, `--token-budget`, `--keyword-ratio`, `--keyword-scores`) and the name-only copy of a result. |
| `judgments-lib.ts` | Reads verdicts from one or more judgments files. A later file may add a verdict, never change one. |
| `transcript-lib.ts` | Reads a sub-agent transcript: its first user message and its tool calls. |
| `size-lib.ts` | A copy of the binary's token estimate and fit from `src/commands/skills/search/page/size.rs` at the base, and the rebuild of a full page from one-item pages. |
| `run-search.ts`, `repeat-check.ts`, `pool-invariance.ts`, `validate-form.ts`, `fill-facts.ts`, `compare-forms.ts`, `intent-check.ts`, `latency-check.ts`, `score-hits.ts` | Copies of the first measurement's scripts, changed as the next section says. |
| `pairs.ts`, `ratio-sweep.ts`, `size-proof.ts`, `facts-change.ts`, `extract-brief.ts`, `transcript-check.ts`, `binary-info.ts`, `repeat-all.ts` | New scripts. |
| `briefs/first-form-writer-run-1.txt`, `-run-2.txt` | The first user message of the two blind form writers of the first measurement, byte for byte. They end with no newline, as the briefs did. |
| `results/` | Every output of `repeat-all.ts`, and `latency-time.txt`. |

## What changed in the copies

- `run-search.ts`, `repeat-check.ts`: take `--keep-hashes yes`, `--token-budget <n>`, `--keyword-ratio <share>` and `--keyword-scores yes`. A request field is sent only when its flag is given, so a plain run sends what the first measurement sent. Results also record `over_budget`.
- `pool-invariance.ts`: also checks that each of the first five tier A and B items keeps its rank (its place over all tiers) with and without the pool, over all pages and on the first page (line `p3`).
- `score-hits.ts`: the counting rules are those of the first measurement. A need is an item with one labelled library skill, or one labelled free-text need; it is hit when one of its skills is at rank 1 to 5 of the first page, over all tiers; a need labelled with no right skill is counted apart; `rex-code-essentials` is the one coding-rule label. Line `k1.8` adds the rank of `rex-code-essentials` on each item that a label gives it or whose answer holds it. `--judgments <old>` and `--judgments-new <new>` may both be given.
- `compare-forms.ts`: unchanged apart from paths; with two `--run` folders and no `--reference` it gives agreement per field of the two runs.
- `validate-form.ts`: reads vocabulary version 2 and its alias table. A word that the aliases map onto a term or an allowed value, such as `typescript` for the language `ts`, is a note, because search reads it as that term or value. The request fields `token_budget` and `keep_hashes` are allowed.
- `fill-facts.ts`: `--commit` is required.
- `size-lib.ts`: follows `page/size.rs` at the base: the budget is `token_budget`, by default 200 tokens for each item of `limit`; the steps are cut reasons (each `why` and each gap reason), leave out hashes unless `keep_hashes`, leave out words, leave out placement; then items move to the next page down to one; a page still over is marked `over_budget`. `files` and `open_when` always stay. One-item pages are asked with a budget that no page reaches, so they are never shortened.

## Commands

`<set>` is `run-1`, `run-2` or `reference`; `<forms>` is `tests/fixtures/gate-a/forms/run-1`, `forms/run-2` or `tests/fixtures/gate-a/reference`; `<filled>` is `/Users/shaun/Code/DEVELOPMENT/gate-a-pool/filled/<set>`; `<filled-new>` is `/Users/shaun/Code/DEVELOPMENT/gate-a-pool/gate-a-2/filled-54d74f2/<set>`. `--pool` defaults to the pool and `--raw` to `/Users/shaun/Code/DEVELOPMENT/gate-a-pool/gate-a-2/raw`. Each output file is under `tests/fixtures/gate-a-2/results/`.

| Measure | Command | Output |
| --- | --- | --- |
| Binary and base | `node tests/fixtures/gate-a-2/binary-info.ts` | `binary.txt` |
| First briefs | `node tests/fixtures/gate-a-2/extract-brief.ts --transcript /Users/shaun/.claude/projects/-Users-shaun-Code-scratch-skilled/90965a73-8bf5-4eb6-aef9-e8b37390eb29/subagents/agent-a0a8b4c997abef5ee.jsonl --out /Users/shaun/Code/DEVELOPMENT/gate-a-pool/gate-a-2/tmp/first-form-writer-run-1.txt --compare tests/fixtures/gate-a-2/briefs/first-form-writer-run-1.txt` (run 2: `agent-acd4eceb203625040.jsonl`) | `brief-check-run-1.txt`, `brief-check-run-2.txt` |
| Transcript check, proven on the first runs | `node tests/fixtures/gate-a-2/transcript-check.ts --transcript <that transcript> --input /Users/shaun/Code/DEVELOPMENT/gate-a-sealed/run-1/input --allow /Users/shaun/Code/DEVELOPMENT/gate-a-sealed/run-1/out --allow /private/tmp/claude-501/-Users-shaun-Code-scratch-skilled/90965a73-8bf5-4eb6-aef9-e8b37390eb29/scratchpad --allow /Users/shaun/Code/DEVELOPMENT/super-rex/.rex/plans/0h0vht64cg --expect tests/fixtures/gate-a/transcripts.txt --section run-1` (and `run-2`) | `transcript-check-first-run-1.txt`, `-run-2.txt` |
| Check forms | `node tests/fixtures/gate-a-2/validate-form.ts <forms>` | `validate-<set>.txt` |
| Search, part 1 raw | `node tests/fixtures/gate-a-2/run-search.ts --forms <filled> --out tests/fixtures/gate-a-2/results/<set> --label <set> --all-pages yes` | `run-search-<set>.txt`, `<set>/` |
| Repeat | `node tests/fixtures/gate-a-2/repeat-check.ts --forms <filled>` | `repeat-<set>.txt` |
| Pool invariance | `node tests/fixtures/gate-a-2/pool-invariance.ts --forms <filled>` | `pool-invariance-<set>.txt` |
| Pairs and verdicts | `node tests/fixtures/gate-a-2/pairs.ts --results tests/fixtures/gate-a-2/results/run-1 --results .../run-2 --results .../reference` | `pairs.txt` |
| Ratio sweep | `node tests/fixtures/gate-a-2/ratio-sweep.ts --forms <filled> --out tests/fixtures/gate-a-2/results/ratio-<set> --label <set>` | `ratio-sweep-<set>.txt`, `ratio-<set>/<ratio>/` |
| Answer size and size proof, part 5 | `node tests/fixtures/gate-a-2/size-proof.ts --set run-1=<filled of run-1> --set run-2=... --set reference=...` | `size-proof.txt` |
| Second run, facts at the base | `node tests/fixtures/gate-a-2/fill-facts.ts --forms <forms> --out <filled-new> --commit 54d74f204205463c037361f6f158c52a5d0e9a67`, then `run-search.ts --forms <filled-new> --out tests/fixtures/gate-a-2/results/<set>-54d74f2 --label <set>-54d74f2 --all-pages yes`, then `facts-change.ts --old-forms <filled> --new-forms <filled-new> --old-results tests/fixtures/gate-a-2/results/<set> --new-results tests/fixtures/gate-a-2/results/<set>-54d74f2`, then `pairs.ts` on the three `-54d74f2` folders | `fill-54d74f2-<set>.txt`, `run-search-<set>-54d74f2.txt`, `<set>-54d74f2/`, `facts-change-<set>.txt`, `pairs-54d74f2.txt` |
| Forms of the two first runs | `node tests/fixtures/gate-a-2/compare-forms.ts --run tests/fixtures/gate-a/forms/run-1 --run tests/fixtures/gate-a/forms/run-2` | `compare-forms-run-1-run-2.txt` |
| Part 6: intent check | `node tests/fixtures/gate-a-2/intent-check.ts` (runs `target/debug/rex skills check --pool <pool> plugin/library`, then each intent phrase as one need) | `intent-check.txt` |
| Part 6: latency example | `node tests/fixtures/gate-a-2/latency-check.ts` | `latency-check.txt` |
| Part 6: repeat and pool invariance of the latency forms | `repeat-check.ts --forms tests/fixtures/gate-a/latency` (and with `--library-only yes`), `pool-invariance.ts --forms tests/fixtures/gate-a/latency` | `repeat-latency.txt`, `repeat-latency-library-only.txt`, `pool-invariance-latency.txt` |
| Part 6: wall time | `node tests/fixtures/gate-a-2/latency-check.ts --time 10 > tests/fixtures/gate-a-2/results/latency-time.txt` | `latency-time.txt` |

## The repeat command

`node tests/fixtures/gate-a-2/repeat-all.ts` runs every command of the table above but the wall time, up to six at a time in the order that each needs, and writes each output to its file under `results/`, with trailing white space removed from each line as the repo's commit hook would. It ends with `results/repeat-all.txt`: the exit code and sha256 of each output. Then `git status --porcelain tests/fixtures/gate-a-2` and `git diff --exit-code tests/fixtures/gate-a-2` print nothing when every output is byte for byte the committed one. `--only <output file>` reruns only that output.

`results/latency-time.txt` is the one output that the repeat command leaves out, because wall time changes from run to run.

An exit code other than 0 in `repeat-all.txt` is a finding of that measure, not a failure of the repeat: `pool-invariance.ts` exits 1 when a form's tier A or B items or their ranks change with the pool, and `ratio-sweep.ts` exits 1 when `--keyword-ratio 0.5` differs from no option.

## Running the scripts on other form sets

Every script takes any folder of filled forms, so the same commands run on the fresh sets. For a fresh set `<fresh>` with forms in `tests/fixtures/gate-a-2/forms/<fresh>/`:

1. `node tests/fixtures/gate-a-2/validate-form.ts tests/fixtures/gate-a-2/forms/<fresh>`
2. `node tests/fixtures/gate-a-2/fill-facts.ts --forms tests/fixtures/gate-a-2/forms/<fresh> --out /Users/shaun/Code/DEVELOPMENT/gate-a-pool/gate-a-2/filled/<fresh> --commit 75566e35d9aa83c2ae166ae1906a83a3b33b8dae`, and with `--commit 54d74f204205463c037361f6f158c52a5d0e9a67` into `filled-54d74f2/<fresh>` for the second run.
3. `run-search.ts`, `repeat-check.ts`, `pool-invariance.ts`, `ratio-sweep.ts`, `size-proof.ts --set <fresh>=<filled folder>`, `facts-change.ts` and `pairs.ts` as in the table.
4. Scores: `node tests/fixtures/gate-a-2/score-hits.ts --labels <labels file> --results tests/fixtures/gate-a-2/results/<fresh> --judgments tests/fixtures/gate-a/judgments.json --judgments-new tests/fixtures/gate-a-2/judgments-new.json`; `pairs.ts` takes the same two files as repeated `--judgments`.
5. Form agreement of two fresh runs: `compare-forms.ts --run tests/fixtures/gate-a-2/forms/fresh-1 --run tests/fixtures/gate-a-2/forms/fresh-2`.
6. Blindness: `transcript-check.ts --transcript <fresh transcript> --input /Users/shaun/Code/DEVELOPMENT/gate-a-2-sealed/<fresh>/input --allow /Users/shaun/Code/DEVELOPMENT/gate-a-2-sealed/<fresh>/out --allow <the writer's scratch folder> --allow <the writer's output folder>`, and `extract-brief.ts --transcript <fresh transcript> --out tests/fixtures/gate-a-2/briefs/<fresh>.txt`.
