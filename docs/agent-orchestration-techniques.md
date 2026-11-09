---
title: Agent Orchestration Lookup Guide
version: "2.1"
updated: "2026-11-09"
purpose: Choose and combine agent structures, execution order, model effort, and verification.
---

# Agent Orchestration Lookup Guide

**Use the smallest setup that improves delivery. Parallelize independent work, sequence dependent work, and add management layers only when they reduce coordination load.**

These patterns are building blocks. Choose who owns the work, when agents run, what capability they need, and how results are checked. Different branches of one project can use different combinations.

## 1. Methods at a glance

### Execution and coordination

| Method | Choose when / example | Advantages | Disadvantages |
| --- | --- | --- | --- |
| **Single agent:** one owner executes the task. | Work is small or tightly coupled. Example: refactor an authentication interface and its callers. | Shared context; clear ownership; minimal overhead. | Serial bottleneck; self-review blind spots; broad scope can overload the agent. |
| **Sequential pipeline:** focused agents run in dependency order. | Each stage needs the previous result, or one agent has too many concerns. Example: researcher → builder → specialist reviewer → verifier. | Narrow remits; fresh scrutiny; explicit handoffs. | Added latency; context can be lost; weak early output propagates. |
| **Fan-out / fan-in:** workers run independently; one owner combines results. | Work can finish without sibling outputs. Example: compare five libraries against one rubric, then synthesize. | Faster independent work; broader coverage; focused contexts. | More total cost; integration effort; conflicting assumptions or writes. |
| **Manager thread:** one coordinator directs several workstreams. | Priorities and dependencies need one owner. Example: coordinate implementation, documentation, and release checks. | One user interface; visible ownership; coordinated delivery. | Manager bottleneck; stale summaries; unnecessary supervision. |
| **Hierarchical delegation:** orchestrator → leads/managers → workers. | Several substantial features each need local coordination. Example: five feature leads, each choosing its own workers and schedule. | Distributes coordination; local ownership; different workflows per branch. | More handoffs; hidden failures; excessive depth can slow delivery. |
| **Long-lived leads:** domain owners persist across tasks. | Context and responsibility recur. Example: service leads oversee a multi-release migration. | Continuity; less repeated onboarding; durable ownership. | Stale context; overlapping authority; state maintenance. |
| **Agent swarms:** many attempts explore a search space. | Candidates are independently testable. Example: search algorithm variants against fixed correctness tests and benchmarks. | Broad exploration; diverse attempts; effort can follow promising results. | High cost; duplication; validation can become the bottleneck. |
| **Agent teams:** coordinated groups pursue distinct approaches. | Workers benefit from exchanging discoveries. Example: teams investigate different failure hypotheses and share reproducible findings. | Local coordination; shared discoveries; distinct strategies. | Messaging overhead; groupthink; overlapping work. |

### Model allocation and quality layers

Add these to any suitable execution structure.

| Method | Choose when / example | Advantages | Disadvantages |
| --- | --- | --- | --- |
| **Effort scaling:** match model capability and reasoning effort to each task. | Difficulty varies. Example: routine documentation workers plus a stronger migration planner. | Targets expensive reasoning; can reduce total cost. | Misrouting; retries and supervision can erase savings. |
| **Executor–advisor:** an executor consults a stronger or specialist agent. | A bounded blocker needs deeper reasoning. Example: supply traces and a reproduction for a deadlock diagnosis. | Preserves execution context; buys help only where needed. | Advice depends on supplied evidence; repeated consultation adds latency. |
| **Pairs:** producer and evaluator iterate. | A focused critique can improve the artifact. Example: a writer and evidence reviewer refine a research brief. | Independent challenge; actionable correction loop. | Shared blind spots; subjective churn; unbounded review loops. |
| **Builder–reviewer–verifier:** separate implementation, inspection, and behavior checks. | Complex work needs both code scrutiny and demonstrated behavior. Example: build checkout logic, review retry handling, exercise payment journeys. | Focused responsibilities; evidence beyond code review. | Extra coordination; duplicated checks unless scopes are clear. |

## 2. Choose sequence, parallelism, and depth separately

- **Sequence** stages when one consumes another's output or needs a stable artifact to inspect.
- **Parallelize** branches when they have explicit interfaces, independent inputs, and nonconflicting ownership of files and state.
- **Add a lead** when a workstream needs repeated local planning, delegation, or integration. A large task alone does not require another layer.
- **Combine freely:** parallel feature leads can each run sequential workers; a sequential stage can fan out independent checks and then combine the results.

Different filenames or tickets do not prove independence. Map deliverables and dependencies, stabilize shared contracts, and schedule work as its prerequisites clear. Isolate writes through owned paths or worktrees; serialize conflicting changes. Name one integration owner before launching parallel builders.

### Three-level ownership

| Level | Owns | Delegates |
| --- | --- | --- |
| **L1 — Orchestrator** | Overall outcome, shared decisions, priorities, cross-feature dependencies, final integration. | Substantial workstreams to leads; small tasks directly to workers. |
| **L2 — Lead / manager** | One feature or domain, local plan, stage transitions, worker handoffs, acceptance evidence. | Bounded implementation, research, review, or verification tasks. |
| **L3 — Worker** | One defined outcome within the assigned scope. | Escalates blockers or requests further decomposition when justified and supported. |

A lead can perform a small task directly. Keep branches shallow unless another layer has a concrete job. Use fresh leads for separate features; retain a lead across tasks when ongoing domain context earns its cost.

### Runtime check

Confirm nested spawning, available tools, concurrency, and context handoff behavior before relying on a hierarchy. If nesting is unavailable, the root can launch workers from lead-written assignments and route results back to the leads.

**Claude Code, checked 9 November 2026:** native subagents can spawn subagents, subject to tool access and the configured depth limit. L1 orchestrator → L2 lead → L3 worker uses **two subagent layers below the main session**. Its separate *agent teams* feature does not permit teammates to spawn their own teammates; teammates can have subagents. See the [subagent nesting documentation](https://code.claude.com/docs/en/sub-agents#let-subagents-spawn-their-own-subagents) and [team limitations](https://code.claude.com/docs/en/agent-teams#limitations).

## 3. Concrete combinations

### A. Complex implementation: focused passes in sequence

**Use when:** one builder would otherwise need to juggle difficult implementation, security analysis, performance analysis, and exhaustive validation at once.

Reuse an adequate existing plan. Otherwise, the lead makes one and consults an advisor for consequential design uncertainty.

1. **Builder:** implement the agreed behavior and baseline checks.
2. **Fresh specialist reviewer:** inspect the artifact for a bounded concern, such as authorization boundaries or retry correctness. Use separate passes for materially different concerns when useful.
3. **Original builder:** resolve actionable findings while retaining implementation context.
4. **Verifier:** exercise acceptance cases, failure paths, and relevant regressions on the repaired artifact. Return failures to the builder for correction and targeted rechecking.

Keep essential correctness and security requirements in the builder's brief. Later passes add coverage. An optional architecture or cleanup worker may address demonstrated friction before final verification, preserving behavior and required coverage. No change is a valid result. Skip roles that add little value.

### B. Large project: five feature leads in parallel

**Setup:** the orchestrator establishes and checks shared foundations: authentication, data contracts, and design conventions. It names an integration owner, then launches five leads for features that can proceed independently against those contracts.

| L2 feature lead | Example local workflow |
| --- | --- |
| Saved views | One builder, then a focused reviewer. |
| CSV export | Builder, then verifier for formatting and large datasets. |
| Bulk import | Format researcher → builder → verifier for invalid input and retries. |
| Notifications | Parallel email and in-app workers against a fixed event contract, then a combined delivery check. |
| Audit history | Builder → integrity reviewer → behavior verifier. |

Leads run concurrently; their L3 workers run sequentially or in parallel as local dependencies allow. Each lead returns an integrated feature and evidence. The orchestrator combines the features and checks cross-feature journeys. Budget for leads and workers together; queue branches and reserve worker capacity when waiting leads still occupy concurrency slots.

If a feature needs another's unfinished output, sequence that dependency. Revisit the partition when shared contracts change.

### C. Two managers: parallel delivery and staged preparation

**Setup:** an orchestrator launches a **product manager** and a **migration manager** for a platform upgrade.

- The product manager delegates independent UI and API workers after fixing their interface, then integrates their outputs.
- The migration manager sequences a source-data analyst, migration builder, and rollback verifier.
- Both managers progress concurrently against an agreed target schema. Their results join at one integration checkpoint; the orchestrator verifies the upgraded application against migrated data.

This adds a useful management layer when each branch has several tasks and local decisions. For one small feature, collapse it to a builder and focused follow-up check.

## 4. Keep each agent's thinking load manageable

**Give each agent one coherent primary job.** Size it to the model's demonstrated capability, reasoning budget, context capacity, and task difficulty.

- Supply core project rules plus task-relevant skills, interfaces, and evidence. Link deeper material for selective reading. Include what exists and what comes next as context; build only the assigned outcome.
- Separate concerns that can be checked afterward. A builder tackling complex code can hand off to fresh specialists for accessibility, security, performance, or edge cases.
- Sequence those specialists when they need a repaired result from the prior pass. Parallelize independent reviews only against a stable artifact, then consolidate findings before fixes.
- Keep tightly coupled implementation decisions together. Split around useful deliverables, not arbitrary steps that repeatedly discard context.
- Use fresh agents for distinct responsibilities; resume the same agent for clarifications and repairs while its context remains useful. If repairs stall, narrow the assignment, seek diagnosis, or change capability.

## 5. Delegate, track, and recover

For substantial delegation, provide:

> **Outcome:** one deliverable and why it matters.
> **Scope:** owned files/state, prerequisites, starting revision, and existing edits or failures.
> **Context:** relevant requirements, interfaces, decisions, and artifact locations.
> **Acceptance:** observable evidence required to pass.
> **Limits:** time/effort, concurrency, and when to escalate.
> **Return:** done/partial/awaiting decision/blocked; artifact references, criterion-to-evidence mapping, and unresolved issues.

For long runs, keep one compact record of the plan, owners, decisions, current stage, artifact versions, evidence, and next action. Record assumptions separately from requirements and propagate changed decisions to affected branches. A starting baseline does not make every later change yours.

| Situation | Coordinator action |
| --- | --- |
| **Stage completes** | Inspect the artifact and acceptance evidence before advancing. Route implementation defects to the builder; revisit design or scope with the lead. |
| **Decision needed** | Resolve from existing requirements and decisions first. Escalate material ambiguity to the parent with options and a recommendation. Distinguish waiting for an answer from execution failure. |
| **Work stalls** | Check progress, runtime failures, and unanswered questions at useful intervals. Silence alone is not failure. Before replacing an active writer, stop it or isolate its writes. |
| **Run resumes** | Reconcile the checkpoint with actual artifacts; resume incomplete work without replaying completed changes or overwriting unrelated edits. |
| **Artifact changes** | Invalidate affected evidence after repairs, cleanup, merges, or contract changes. Recheck the final integrated artifact; earlier passes may no longer establish correctness. |

Agent agreement is not proof: use tests, sources, reproducible observations, or an explicit rubric. Choose verification by uncertainty and consequences as well as complexity. Stop when acceptance passes; bound reviews, retries, and search. Replan when failures repeat or coordination outweighs delivery. Report unresolved and deferred work explicitly.

## Provenance

Condensed from the supplied guide, retaining its ten patterns and adding explicit sequential pipelines and hierarchical delegation. Original taxonomy attribution: Ray Amjad, [OpenAI and Anthropic Agree on What Comes After Loops](https://www.youtube.com/watch?v=yrRpnx_iugY). Selection rules and examples are practical synthesis, not benchmark claims. Runtime-specific statements are linked to official documentation above.
