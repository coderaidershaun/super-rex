# Assignment table

One home of which skills each task gets. Table is data of plugin: file `assign/table.json`. Mod tool `rex_assign` applies it. Code decides each rule. Decision agent answers only questions that need judgement. Code places each skill. Mod records each answer + each skill, with source + reason. Stage 1 finds needs beyond table. Stage 2 + 3 copy `Skills` + `Open when` of each task from answer of `rex_assign`. Challenger checks recorded answers.

- [ ] Never place skill of table by hand. Call `rex_assign`.
- [ ] Never read `assign/table.json` by path. Question lines of `rex_assign` give each row + criterion that needs answer.

## Words

- Row: one kind of work. It gives skills + sibling files, with priority + placement.
- Criterion: one fact about task. Row fits task IF one criterion of row is true (OR).
- Rule: criterion that code decides from facts of task (`writes_code`, `langs`, `role`). Nobody answers it.
- Question: one line `question.<n>=task:<id> row:<row> criterion:<id> <text>`. Criterion `each-step`: does task need row in each step, or only in one case.
- Answer: `yes` or `no` + one-line reason. Recorded once.
- Placement: read first = `Skills`. Open when = `Open when`, with case.
- Source: `rule` (table rule), `decision-agent` (answer of decision agent), `override` (correction with reason, in `rex_job_plan`).
- Overlay: `.rex/assign.json` of `HOME_DIR`. It changes rows for this repo. Only `rex-feedback` edits it.

## Each task that writes code

Rule rows. Task with `writes_code` = true gets, read first:

| Skill | Why |
| --- | --- |
| `rex-code-philosophy` | Complexity, habitability. |
| `rex-code-commenting` | Comments, and module comment of each file. |
| `rex-code-ergonomics` | Names, types, signatures. |
| `rex-code-error-writing` | Typed errors that carry context. |

Plus gap skill `rex-gap-<language>` of each language in `langs` that is not `rust` (`## Gap skill` of `orchestration-builder/stages/1-skills/stage.md`).

- [ ] Rulebook text is about Rust. Agent applies intent of each rule in idiom of its language.
- [ ] No answer removes these skills. Code gives them in each case.

## Call rex_assign

Decision agent. Work breakdown of stage 2 has its tasks. Load tool with ToolSearch first: `rex_assign` waits behind it.

1. Call `rex_assign` with `tasks` + `roles`.
2. Read `questions=<n>` + each `question.<n>=` line.
3. Answer each question in `answers`: `{ question, answer, reason }`. `question` = `task:<id> row:<row> criterion:<id>` text of its line.
4. Call `rex_assign` again with same `tasks` + `roles` + `answers`. DO steps 2 to 4 UNTIL `questions=0` OR 3 rounds done.
5. Copy result into work breakdown (`## Result`).

`tasks`: one row for each task of work breakdown.

| Field | Value |
| --- | --- |
| `id` | `Task` of work breakdown. Lead, or role with `—` in `Tasks`: role name. |
| `role` | Role of `ROLE TABLE` that does task. |
| `writes_code` | `true` IF task writes or changes code. ELSE `false`. |
| `langs` | Languages of its code, as `rex codebase --only` names them (`rust`, `ts`). Leave out: languages of job. |

`roles`: one row for each role of `ROLE TABLE`: `{ role, intelligence, reliability, consequence, verification }`. Stage 3 gives ratings. First call before stage 3: `roles` = `[]`. Last call: all roles.

- [ ] Facts of task are facts, not judgement. IF task writes one line of code, THEN `writes_code` = `true`.
- [ ] Answer `yes` only IF task does that work. Reason = what in task does it: file, step, or outcome.
- [ ] Answer `no` with reason also. Challenger reads each reason.
- [ ] `each-step`: `yes` IF each step of task needs row. `no` IF task reaches row only in one case. Case = criterion text.
- [ ] Answered question stays. Second answer to it changes nothing: mod gives stored answer. Correction: `## Overrides`.

## Result

Last call gives:

```
questions=0
task.<id>.skills=<skill>,<skill>,…
task.<id>.open_when.<skill or skill/file.md>=<case>
source.<task>.<skill or skill/file.md>=<rule, decision-agent, or override>
role.<role>=super-rex:<grid agent>
```

- [ ] `Skills` of task = list of `task.<id>.skills`, as written. `none` = `—`.
- [ ] `Open when` of task = each `task.<id>.open_when.` line: `<item> (<case>)`.
- [ ] `Why` of SKILL LIST: source of skill, + reason of answer.
- [ ] `role.` lines: agent of each role. `rex_job_plan` runs `rex oms` again with same ratings.
- [ ] Max 6 read-first skills: code moves rest to `Open when`, lowest priority first.
- [ ] Sibling file: always `Open when`, with its case.
- [ ] `rex_job_plan` takes these skills, not `skills` + `open_when` of its own `tasks`.

## Overrides

Skill that table does not give, OR answer that challenger proves wrong:

- [ ] Write one row in `OVERRIDES` of decision block: subject `task:<id>`, item `skill` (read first), `open_when.<skill>` OR `file.<skill>/<file>.md` (value = case), value, reason.
- [ ] `Why` of SKILL LIST starts `Not in table:` for skill that table does not give.
- [ ] Builder gives `OVERRIDES` to `rex_job_plan` as `overrides`. Mod records each with source `override`.
- [ ] Rule-required skill stays. IF task must not read it, THEN put reason under `REPORT`.

## Not in table

A person starts these, OR builder reads their files by path: `orchestration-builder`, `rex-deliver`, `rex-feedback`, `rex-grill-me`. No task gets them.

`rex-rules`: router for session with no builder. It sends that session to this table. No task gets it.
