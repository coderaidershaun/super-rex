# Build new orchestrator skill

Steps 2 + 5 of `.claude/skills/orchestration-selection/SKILL.md`, when its step 1 finds none to reuse. Uses words of that file, and words below.

- Requester: User or agent that gave request.
- Merge agreement: Answer of requester: merge work to main, yes or no.
- Integration owner: One agent that combines parallel work and tests result.
- Lead skill: Reusable file for lead. Kept in folder of its orchestrator skill.
- Orchestrator library: Folder `.rex/orchestrators/` in `HOME_DIR`. One folder for each orchestrator skill. Outside `.claude/skills/`.

## Step 2 — Method

Use smallest setup that improves delivery. Run independent work in parallel. Run dependent work in sequence. Add layer of management only IF it reduces load of coordination.

Execution methods:

| Method | Use when | Cost |
| --- | --- | --- |
| Single agent: one owner does work. | Work is small or tightly coupled. | Serial. Self-review misses own errors. Broad scope overloads agent. |
| Sequential pipeline: focused agents run in dependency order. | Each stage uses result of stage before it. OR one agent has too many concerns. | More latency. Context loss at handoffs. Weak early output spreads. |
| Fan-out / fan-in: workers run independently, integration owner combines results. | Each part can finish without output of other parts. | More total cost. Integration work. Conflicts of assumptions or writes. |
| Manager thread: one coordinator directs several workstreams. | Priorities + dependencies need one owner. | Manager = bottleneck. Old summaries. Supervision with no use. |
| Hierarchical delegation: orchestrator -> leads -> workers. | Several substantial features, each needs local coordination. | More handoffs. Hidden failures. Too much depth slows delivery. |
| Long-lived leads: domain owners stay across tasks. | Same context + responsibility return in later tasks. | Old context. Overlap of authority. State to keep current. |
| Agent swarms: many attempts search one problem space. | You can test each candidate alone. | High cost. Duplicate work. Validation becomes bottleneck. |
| Agent teams: groups try different approaches and share findings. | Workers gain from findings of other workers. | Message overhead. Groupthink. Overlap of work. |

Quality layers. Add to any execution method:

| Method | Use when | Cost |
| --- | --- | --- |
| Effort scaling: model + effort match each task. Step 4 does this. | Difficulty differs between tasks. | Wrong match. Retries + supervision can erase savings. |
| Executor-advisor: executor asks stronger or specialist agent. | Bounded blocker needs deeper reasoning. | Advice is only as good as its evidence. Each consult adds latency. |
| Pairs: producer + evaluator iterate. | Focused critique can improve result. | Shared blind spots. Subjective churn. Review loop with no limit. |
| Builder-reviewer-verifier: separate build, inspection, and test of behavior. | Complex work needs inspection + proof of behavior. | More coordination. Duplicate checks IF scopes are not clear. |

Select sequence, parallel, and depth, each one alone:

| Choice | Select when |
| --- | --- |
| Sequence | Stage uses output of other stage, OR must inspect stable result. Work breakdown: dependency. |
| Parallel | Interfaces explicit, inputs independent, no conflict on files or state. Work breakdown: group. |
| Depth (add lead) | Workstream needs local plan, delegation, or integration many times. Large task alone is no reason. |

Methods combine. Parallel leads can each run sequential workers. Sequential stage can start independent checks in parallel, then combine results.

**Levels.**

| Level | Owns | Delegates |
| --- | --- | --- |
| L1 orchestrator | Overall outcome, shared decisions, priorities, dependencies across workstreams, final integration. | Substantial workstreams to leads. Small tasks direct to workers. |
| L2 lead | One feature or domain: local plan, stage transitions, handoffs to workers, acceptance evidence. | Bounded tasks: build, research, review, verification. |
| L3 worker | One defined outcome inside its scope. | Nothing. Escalates blockers, or asks for more division of work. |

- [ ] Lead can do small task itself. Keep hierarchy shallow: add layer only IF it has concrete job.
- [ ] New lead for each separate feature. Keep lead across tasks only IF its domain context is worth its cost.

**Limits of Claude Code.** Read `.claude/skills/orchestration-selection/limits.md` before you add a lead, or IF orchestrator skill runs below level 1.

**Load of each agent.**

- [ ] One coherent primary job for each agent. Size it to capability of agent and difficulty of task.
- [ ] Separate concerns that later pass can examine. Example: agent that built work gives it to fresh specialists for security, performance, edge cases.
- [ ] Run parallel reviews only against stable result. Combine findings before fixes.
- [ ] New agent for different responsibility. Same agent for clarifications + repairs while its context is useful.
- [ ] IF repairs stall, THEN make task narrower, get diagnosis, or change agent.

### Inputs shape method

Seven inputs select models, and they shape method also.

- Example: role with consequence `high` and verification `low` (hard to examine). Add reviewer or verifier role.
- Example: `rex oms` output shows `demand` higher than `agent`: cap applied. Answer with orchestration, for example add verifier role.

Fact: duration caps effort only. How long job takes depends most on how much runs in parallel.

## Step 5 — Write orchestrator

Only IF step 1 found none. Write it yourself. First read `.claude/skills/rex-skill-language/SKILL.md` IN FULL. Write in that language.

Contents:

- [ ] Roles: name + job of each.
- [ ] Order, and what runs in parallel.
- [ ] Skills that each role reads.
- [ ] Checks: how agents examine work.
- [ ] One rule (below).
- [ ] Tag block: each brief starts with it (`.claude/skills/orchestration-selection/briefs.md`).
- [ ] Each role that starts agents: what it does when it has no Agent tool. Facts: `.claude/skills/orchestration-selection/limits.md`.

Form: like any skill. Header, then `## Words`, flow block, sections. Header:

```
---
name: <name>
description: >
  <What this orchestrator skill does, in full sentences.>
when_to_use: >
  Use when orchestration builder selects this orchestrator skill for a job.
---
```

- [ ] `<name>` = folder name: lower-case letters and digits in groups joined by single hyphens.
- [ ] `description`: `rex recall` prints it for orchestrator skill that never ran.
- [ ] `when_to_use`: write only line above. Write no text on what kind of job it fits.
- [ ] Size: max 1,500 words after header of `SKILL.md`. Max 60 words in `description` + `when_to_use`, so max 49 in `description`. Text for one role or one case goes in sibling file in same folder. `SKILL.md` says when to read it.
- [ ] Not about one task. Nothing of this request goes in. Task detail goes in briefs.
- [ ] No model names, no grid agents. Roles only.
- [ ] No other fixed rule. Design it on its merits: no fixed count of rounds, proof form, or test budget, unless this design wants one.
- [ ] Write file at `.rex/orchestrators/<name>/SKILL.md`.
- [ ] Lead skill: IF lead has its own procedure, THEN write file `<role>.md` in same folder. Lead gets its path in brief.

Then run `rex skills check "$HOME_DIR/.rex/orchestrators/<name>"`. DO correct files UNTIL exit code 0 OR 3 rounds done. IF it still fails, THEN keep files and give findings to requester in final report: put them under `REPORT` of decision block.

### One rule

Every orchestrator skill carries this rule, in these words:

```
- [ ] Merge agreement: IF job record or brief gives it, THEN use it. ELSE ask requester: merge work to main, yes or no.
```

Builder settles merge agreement before job opens. So requester gets question one time.

## Orchestrator library

- [ ] Where: `.rex/orchestrators/` in `HOME_DIR`. Outside `.claude/skills/`. Not in Git.
- [ ] Each folder in library = one orchestrator skill: its `SKILL.md` + its lead skills. Never give lead skill its own folder.
- [ ] `rex job plan` and `rex recall` index library and give version of each orchestrator skill. No agent writes version.
- [ ] Store holds results of jobs that each orchestrator skill ran.
- [ ] Claude Code does not list this folder. So library costs no context in other sessions.
- [ ] Use: session reads orchestrator skill IN FULL and obeys it. Plain file read, not Skill tool.
- [ ] Fit: write no text on what orchestrator skill fits. Store reads fit from jobs that orchestrator skill ran, and their results.
