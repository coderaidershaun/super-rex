---
name: orchestration-work-breakdown
description: >
  Stage 2 of the orchestration builder. It divides a request into tasks, gives
  each task its skills, and finds which tasks run in parallel and which in
  sequence. Its output goes into the plan and the briefs.
when_to_use: >
  Use when the decision agent of `orchestration-builder` reaches stage 2, or
  when an agent must find what work can run in parallel.
disable-model-invocation: true
user-invocable: false
---

# Work Breakdown

Divide request into tasks. Find what depends on what, and what can run in parallel.

```
1. Read input. Look at repo.
2. Divide request into tasks. Give each task its skills.
3. Find dependencies and shared contracts.
4. Put tasks in groups: parallel or sequence.
5. Name integration owner of each group with 2 or more tasks.
6. Give WORK BREAKDOWN to builder.
IF job has one outcome, THEN write one task and go to step 6.
```

## Words

- Decision agent: Sub-agent of builder that does stages 1 to 3. Its brief gives input of each stage.
- Skill list: Skills this job uses. Output of stage 1.
- Task: One outcome of job, with evidence that shows it is complete.
- Dependency: Task T2 uses output of task T1. Then T2 depends on T1.
- Shared contract: Interface, data form, or decision that 2 or more tasks use.
- Group: Tasks that can run at same time. Group can hold one task.
- Integration owner: One agent that combines parallel work and tests result.
- Work breakdown: Tasks of job, their dependencies, and groups that run in parallel. Output of stage 2.
- Brief: Prompt that sub-agent gets.
- Orchestrator skill: Reusable file. It gives roles, order, skills of each role, and checks. Not about one task.
- Orchestrator, lead, worker: Level 1 = session. Level 2 = sub-agent of session. Level 3 = sub-agent of lead.

## Input

- [ ] From builder: request, classification, recall output, skill list.
- [ ] Look at repo: `rex codebase --with-context`. It prints tree of repo, with first sentence of opening comment of each source file.
- [ ] Use lessons of recall output and skills of skill list where they apply.

Work can be code, research, documents, or other work. Task can change no file.

## Step 1 — Tasks

- [ ] Divide request into tasks. For each task, write outcome + evidence.
- [ ] Divide around useful outcomes, not around steps.
- [ ] Keep decisions that are tightly coupled in one task.
- [ ] Evidence: tests, sources, reproducible observations, or explicit rubric.
- [ ] Skills: for each task, names from skill list that task needs: core rules for its kind of work and its language, gap skill of each language that it writes, and skills for its task. Add `<skill>/<file>.md` for each sibling file that task needs. Agent of task reads only these.
- [ ] IF job has one outcome, THEN write one task. Go to `## Output`.
- [ ] Plan job: tasks = research + plan writing. Last 2 tasks: adversarial review of plan, then revision. Evidence of review task: verdict table.
- [ ] Plan execution: start from task table of plan file. Change task only IF repo changed since plan.

## Step 2 — Dependencies

For each task, write:

- [ ] Output of other tasks that it uses.
- [ ] Files and state that it changes.
- [ ] Shared contracts that it uses.

IF 2 or more tasks use one shared contract, THEN make contract stable first, in its own task.

## Step 3 — Parallel or sequence

- [ ] IF task uses output of other task, OR must examine stable result of other task, THEN put them in sequence.
- [ ] Put tasks in parallel only IF all three are true:
  1. Interfaces between tasks are explicit.
  2. Inputs of tasks are independent.
  3. Tasks own different files and state.
- [ ] Different file names do not prove that tasks are independent.
- [ ] IF tasks change same files or state, THEN put them in sequence.
- [ ] Each parallel task that writes in Git repo works in its own Git worktree. `.claude/skills/rex-deliver/parallel-writers.md` names branch + worktree. Agent that starts parallel writers reads `.claude/skills/rex-deliver/SKILL.md`, then that file.

Groups:

1. Number groups 1, 2, 3, in run order.
2. Group 1 = tasks that depend on no task.
3. Each other task goes into first group after groups of all tasks it depends on.
4. Tasks of one group must pass parallel test with each other. IF 2 tasks fail it, THEN move one to later group.
5. Task can start when all tasks it depends on are complete.

## Step 4 — Integration

- [ ] For each group with 2 or more tasks: name one integration owner before group starts.
- [ ] Name no model for owner. Example owner: orchestrator, or integration task in next group.
- [ ] Evidence of each task is from before integration. It is not proof for combined result.
- [ ] Integration owner tests combined result.

## Output

WORK BREAKDOWN: one table, plus integration owner of each group with 2 or more tasks.

Columns, in this sequence: `Task / Outcome / Evidence / Depends on / Owns (files, state) / Group / Skills`.

Example:

| Task | Outcome | Evidence | Depends on | Owns (files, state) | Group | Skills |
| --- | --- | --- | --- | --- | --- | --- |
| T1 | Event contract stable | Contract tests pass | — | `events/` | 1 | `rex-code-ergonomics`, `rex-code-tests-contract-seams` |
| T2 | Email notices sent | Email tests pass | T1 | `email/` | 2 | `rex-code-ergonomics`, `rex-code-tdd` |
| T3 | In-app notices shown | In-app tests pass | T1 | `inapp/` | 2 | `rex-code-ergonomics`, `rex-code-tdd` |

Integration owner, group 2: orchestrator. Evidence: delivery test of combined result passes.

- [ ] Work breakdown is about this job only.
- [ ] Builder records it with `rex job plan` and puts task detail in briefs.
- [ ] It never goes into orchestrator skill.
- [ ] This stage does not select method, roles, or models. `orchestration-selection` does that. It can add skills to `Skills` of a task.
