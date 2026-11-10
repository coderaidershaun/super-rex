---
name: orchestration-selection
description: >
  This is stage 3 of the orchestration builder. It reuses an orchestrator skill
  from the library, or it builds a new one that suits the work breakdown. It
  defines the roles and rates the inputs that select a model for each role. It
  holds a summary of the methods of agent orchestration and of the seven inputs
  of model selection.
when_to_use: >
  Use when `orchestration-builder` reaches stage 3, or when an agent selects
  orchestration and models for roles.
disable-model-invocation: false
user-invocable: false
---

# Orchestration Selection

Select or build orchestrator skill that suits work breakdown. Give each role its agent.

```
1. Reuse. IF orchestrator skill in recall output suits work breakdown, THEN reuse it, do step 4, give output.
2. ELSE select method.
3. Define roles + skills of each role.
4. For each role: rate inputs, run `rex oms`, read `demand` + `conflict`. OR give role session agent.
5. Write orchestrator skill + lead skills. Run `rex skills check` on its folder.
6. Give output. Builder records it with `rex job plan`.
```

## Words

- Requester: User or agent that gave request.
- Session: Claude Code conversation that loaded builder. It has one session ID.
- Builder: Agent that obeys `orchestration-builder`.
- Job: One request that SuperRex handles. It has one job ID.
- Classification: Four fields of job: work type, size, consequence, languages.
- Preference: Duration, budget, or autonomy that user stated in request.
- Merge agreement: Answer of requester: merge work to main, yes or no.
- Skill list: Skills this job uses. Output of stage 1.
- Nuanced skill: Skill for something particular to repo or to kind of work, that no other skill covers.
- Gap skill: Nuanced skill for one language. It covers what Rust gives by default and that language does not.
- Work breakdown: Tasks of job, their dependencies, and groups that run in parallel. Output of stage 2.
- Integration owner: One agent that combines parallel work and tests result.
- Role: Named part in orchestrator skill. Example: `engineer`.
- Grid agent: One of 20 agent definitions in `.claude/agents/`. Name: `model-<tier>-effort-<level>`.
- Session agent: `general-purpose` sub-agent that you start with no model. It runs at model + effort of session.
- `HOME_DIR`: Absolute path of main checkout root of home repo. Home repo = repo that holds store (`.rex/`) and skills (`.claude/skills/`) of session. IF your shell is in Git worktree of home repo, THEN `HOME_DIR` = parent folder of `git rev-parse --path-format=absolute --git-common-dir`.
- `JOB_DIR`: Absolute path where job branch is checked out. START of `rex-deliver` gives it.
- Role table: For one job: skills, inputs, and agent of each role. Output of stage 3.
- Orchestrator skill: Reusable file. It gives roles, order, skills of each role, and checks. Not about one task.
- Lead skill: Reusable file for lead. Kept in folder of its orchestrator skill.
- Orchestrator library: Folder `.rex/orchestrators/` in `HOME_DIR`. One folder for each orchestrator skill. Outside `.claude/skills/`.
- Brief: Prompt that sub-agent gets.
- Tag block: First lines of brief: `Rex-Job: <job ID>`, `Rex-Role: <role>`, then one blank line.
- Orchestrator, lead, worker: Level 1 = session. Level 2 = sub-agent of session. Level 3 = sub-agent of lead.
- Paths: each path in this skill starts at `HOME_DIR`.

## Input

Work breakdown, skill list, classification, preferences, recall output.

## Step 1 — Reuse

Reuse before you build.

1. Candidates = `orchestrator` lines and `unused` lines of recall output.
2. `orchestrator` line: orchestrator skill that similar jobs used, with results of those jobs. `unused` line: orchestrator skill whose newest version never ran.
3. Read `SKILL.md` of each candidate IN FULL. Its path is in `path` of its line.
4. IF one suits work breakdown, THEN reuse it. You decide: weigh results of its jobs against preferences.
5. IF none suits, THEN build new one: steps 2, 3, 5. In both cases, do step 4 for each role.

- [ ] Reused file gives roles + skills of each role. Add skills from skill list that role uses.
- [ ] Add no rule to reused file. Rule enters orchestrator skill only through `rex-feedback`.

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

**Limits of Claude Code.**

- [ ] Default max = 3 layers of sub-agent below main session. `CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH` changes it.
- [ ] L1 -> L2 -> L3 uses 2 layers. One layer is spare. IF you are sub-agent, THEN you have fewer layers below you.
- [ ] Agent at last layer cannot start agents. It has no Agent tool.
- [ ] Agent teams: teammate cannot start teammates. Teammate can start sub-agents.
- [ ] IF lead cannot start agents, THEN lead writes assignments. Orchestrator starts workers and gives their results to lead.

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

### Plan jobs

Plan job: deliverable = plan file, not code. `## Plan jobs` in `orchestration-builder` defines it. Build its orchestrator skill for good planning, not for writing code.

Order of roles:

1. Research, IF plan depends on facts that nobody knows yet: readers in parallel, one question each. Evidence: sources, code that they read, or spike in scratch folder (`rex-code-spike`).
2. Plan writer: one role writes whole plan file. One author keeps plan coherent.
3. Plan reviewer: role `plan-reviewer`. Never plan writer. Obeys `rex-codex-adversarial-review`.
4. Revision: plan writer changes plan from verdict table. Same agent IF its context is still useful.

- [ ] Each orchestrator skill for plan job holds steps 2, 3, 4. Step 1 is optional.
- [ ] Rate plan writer: intelligence `high`, verification `low` (plan is hard to examine). Consequence = consequence of work that plan leads to.
- [ ] Rate `plan-reviewer`: intelligence `high`, verification `low`. Its challenger (Codex or fallback agent) comes from its skill, not from `rex oms`.
- [ ] Roles of plan job change no file in Git repo. They write plan file + scratch files only.
- [ ] Reuse orchestrator skill for planning IF recall output gives one.

## Step 3 — Roles

Roles, not models. Orchestrator skill says "start agent for role `engineer`". Role table says which agent that is.

For each role, define:

- [ ] Name: lower-case letters, digits, and `-` only. Examples: `engineer`, `reviewer`, `verifier`.
- [ ] Job: one primary job.
- [ ] Skills it reads, from skill list: core rules + skills for its task.
- [ ] Gap skill of each language that role writes.

Skills are files. Role reads each `.claude/skills/<name>/SKILL.md` IN FULL. Its brief gives absolute paths.

## Step 4 — Model for each role

Seven inputs. Each value = `low`, `medium`, `high`, or `none`.

| Input (flag) | Belongs to | `low` | `medium` | `high` |
| --- | --- | --- | --- | --- |
| `--duration` | Job | Finish quickly. Also `fast`. | Moderate wait is acceptable. | Longer run is acceptable. Also `long`. |
| `--budget` | Job | Keep spend low. | Moderate spend. | Higher spend when useful. |
| `--autonomy` | Job | Frequent guidance is acceptable. | Occasional help is acceptable. | Work alone. Ask only when necessary. |
| `--intelligence` | Role | Basic capability. Routine task. | Reasoning + judgment. | Advanced reasoning. Difficult task. |
| `--reliability` | Role | Occasional failures are acceptable. | Failures are uncommon. | Failures are very rare. |
| `--consequence` | Role | Mistake is minor, easy to reverse. | Mistake causes meaningful rework or disruption. | Mistake causes serious damage or loss, hard to reverse. |
| `--verification` | Role | Correctness is hard to examine. | Some review or test is necessary. | Quick, reliable checks exist. |

- [ ] `none` = input is not relevant. Function ignores it. Leave flag out: same as `none`.
- [ ] Job inputs: give `rex oms` same job inputs that `rex job create` got. These are only preferences that user stated.
- [ ] Role inputs: rate each role. Role consequence starts at consequence in classification. It can be lower for role that cannot cause that harm.
- [ ] Higher budget or longer duration permits more spend or time. It does not require it.

Who selects agent:

- [ ] Job with plan: `rex job plan` selects agent of each role from same inputs. Builder starts each role with agent that `rex job plan` printed.
- [ ] Run `rex oms` for each role in this step. Read `demand` + `conflict` before you fix roles: job gets one plan only.
- [ ] Builder inside open job has no plan. It starts each role with agent of `agent` line of `rex oms`.

How function acts (function version 1). Stakes = higher of reliability and consequence.

- [ ] Tier starts at intelligence. Stakes `high` adds one step.
- [ ] Effort starts at stakes. Verification `low` adds one level, `high` removes one. Autonomy `high` adds one, `low` removes one.
- [ ] Budget caps tier + effort. Duration caps effort only. Caps never raise anything.
- [ ] Stakes `high`: result is never below `model-medium-effort-high`. `conflict` line names cap that floor overrode.
- [ ] All inputs `none` gives `model-medium-effort-medium`.

IF `version` line of output is not `1`, THEN trust output, not these lines.

Run:

```
rex oms [--duration <v>] [--budget <v>] [--intelligence <v>] [--autonomy <v>] \
        [--reliability <v>] [--consequence <v>] [--verification <v>]
```

Output = six `key=value` lines, in fixed order. Write `agent` in role table.

| Key | Value |
| --- | --- |
| `agent` | Grid agent to use. |
| `demand` | Pair that role needed before caps. Same as `agent` IF no cap applied. |
| `rules` | Rules that fired, comma-separated. |
| `conflict` | `none`, `budget`, `duration`, or `budget,duration`. |
| `inputs` | Seven inputs as seven characters: duration, budget, intelligence, autonomy, reliability, consequence, verification. `L`, `M`, `H`, or `-` for `none`. |
| `version` | Function version, whole number. |

Example:

```
$ rex oms --budget low --intelligence high --autonomy high \
          --reliability high --consequence high --verification low
agent=model-medium-effort-high
demand=model-xhigh-effort-max
rules=T1,T2,E1,E2,E4,C1,C2,F1
conflict=budget
inputs=-LHHHHL
version=1
```

Grid: tiers `low`, `medium`, `high`, `xhigh`. Levels `low`, `medium`, `high`, `xhigh`, `max`. Grid agent runs at model + effort of its definition.

- [ ] Orchestrator = session. It runs at model + effort of session. Run no `rex oms` for it.
- [ ] Role that must run as session does: run no `rex oms`. Write `session agent` in `Agent` column, and `—` in each input column. In plan file, it is `"session": true`.

## Step 5 — Write orchestrator

Only IF step 1 found none. Write it yourself. First read `.claude/skills/rex-skill-language/SKILL.md` IN FULL. Write in that language.

Contents:

- [ ] Roles: name + job of each.
- [ ] Order, and what runs in parallel.
- [ ] Skills that each role reads.
- [ ] Checks: how agents examine work.
- [ ] One rule (below).
- [ ] Tag block: each brief starts with it (`## Briefs`).

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
- [ ] Not about one task. Nothing of this request goes in. Task detail goes in briefs.
- [ ] No model names, no grid agents. Roles only.
- [ ] No other fixed rule. Design it on its merits: no fixed count of rounds, proof form, or test budget, unless this design wants one.
- [ ] Write file at `.rex/orchestrators/<name>/SKILL.md`.
- [ ] Lead skill: IF lead has its own procedure, THEN write file `<role>.md` in same folder. Lead gets its path in brief.

Then run `rex skills check "$HOME_DIR/.rex/orchestrators/<name>"`. DO correct files UNTIL exit code 0 OR 3 rounds done. IF it still fails, THEN keep files and give findings to requester in final report.

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
- [ ] Lead gets path of its lead skill in brief. Lead reads it same way.
- [ ] Fit: write no text on what orchestrator skill fits. Store reads fit from jobs that orchestrator skill ran, and their results.

## Briefs

Each brief starts with tag block, as its first characters. Hooks read it to record which role each run filled. This is no quality rule.

- [ ] Nothing before tag block: no heading, quote mark, or code fence. One space after each colon.
- [ ] `Rex-Job`: copy it from tag block of your own brief. Session copies `job=` that `rex job create` printed.
- [ ] `Rex-Role`: role of new sub-agent, as role table names it.
- [ ] Session agent: tag block is only source of its role.
- [ ] Do not tell agents to correct tags of brief already sent, or to send it again.
- [ ] Paths in brief are absolute. Git worktree does not hold files that Git ignores.
- [ ] Writer in Git repo: IF job works in job worktree, THEN brief gives `JOB_DIR`.
- [ ] Parallel writer: its lines come from section "Parallel writers" of `rex-deliver`.
- [ ] Task detail from work breakdown goes in brief.
- [ ] Brief is for agent with no context: full sentences.

Template:

```
Rex-Job: <job ID>
Rex-Role: <role>

Outcome: <one deliverable, and why it matters>.
Scope: <the files and state that you own, the prerequisites, the starting revision, and the edits or failures that exist already>.
Context: <the requirements, interfaces, decisions, and locations of artifacts that apply>. Read these skills in full before you start: <absolute paths>.
Acceptance: <the evidence that you must show to pass>.
Limits: <the time and effort, the concurrency, and when to escalate>.
Return: Give a short verdict and references: paths and commits. Map each acceptance criterion to its evidence. List the open issues. End the report with one state: done, partial, awaiting decision, or blocked.
```

## Guidance

Guidance, not rules. Orchestrator can do different.

- [ ] Short reports. Sub-agent returns short verdict + references (paths, commits). No long report, no chatter.
- [ ] Report ends with one of four states: `done`, `partial`, `awaiting decision`, `blocked`.
- [ ] "How is it going?" from person or agent: answer from `REX_STORE="$HOME_DIR/.rex" rex status <job-id>`. Sub-agent can run it also.
- [ ] `rex status` prints one header line for job: ID, status, counts `open`, `finished`, `unconfirmed`. Then one line for each run: agent ID, role, agent type, duration, tokens.
- [ ] `rex status` with no job ID: each open job.
- [ ] Agreement of agents is not proof. Use tests, sources, observations you can repeat, or explicit rubric.
- [ ] Select verification by uncertainty + consequence, not by complexity only. Stop when acceptance passes.
- [ ] Put limit on reviews, retries, and search.
- [ ] IF failures repeat, OR coordination costs more than delivery, THEN plan again.

Recovery:

| Situation | Action |
| --- | --- |
| Stage completes | Inspect result + acceptance evidence before next stage. Send defect of build to agent that built it. Send design or scope to lead. |
| Decision needed | Decide from requirements + decisions first. Send material ambiguity to parent, with options + recommendation. Wait for answer is different from failure. |
| Work stalls | Examine progress, failures, open questions at useful intervals. Silence alone is not failure. Before you replace active writer: stop it, or isolate its writes. |
| Run resumes | Compare checkpoint with actual files. Continue incomplete work. Do not repeat complete changes. Do not overwrite unrelated edits. |
| Result changes | After repair, clean-up, merge, or contract change: old evidence is not valid. Test final integrated result again. |

## Output

Paired challenger checks both first (`orchestration-decision-challenge`). Then builder records both with `rex job plan`.

```
ORCHESTRATOR: <name> | <path> | new OR reused
ROLE TABLE:
| Role | Skills | Intelligence | Reliability | Consequence | Verification | Agent |
| <role> | <paths> | <v> | <v> | <v> | <v> | <grid agent> OR session agent |
```

Role with `session agent` has `—` in each input.
