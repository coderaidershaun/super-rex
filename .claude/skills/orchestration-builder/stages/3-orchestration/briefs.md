# Briefs and guidance

For agent that runs orchestrator skill: session at step 7 of `.claude/skills/orchestration-builder/run.md`, or builder inside open job. Uses words of `.claude/skills/orchestration-builder/SKILL.md`, and words below.

- `JOB_DIR`: Absolute path where job branch is checked out. `.claude/skills/rex-deliver/start.md` gives it.
- Tag block: First lines of brief: `Rex-Job: <job ID>`, `Rex-Role: <role>`, then one blank line.
- Lead skill: Reusable file for lead: `<role>.md` in folder of orchestrator skill.

## Briefs

Each brief starts with tag block, as its first characters. Hooks read it to record which role each run filled. This is no quality rule.

- [ ] Nothing before tag block: no heading, quote mark, or code fence. One space after each colon.
- [ ] `Rex-Job`: copy it from tag block of your own brief. Session copies `job=` that `rex job create` printed.
- [ ] `Rex-Role`: role of new sub-agent, as role table names it.
- [ ] Session agent: tag block is only source of its role.
- [ ] Do not tell agents to correct tags of brief already sent, or to send it again.
- [ ] Paths in brief are absolute. Git worktree does not hold files that Git ignores.
- [ ] Writer in Git repo: IF job works in job worktree, THEN brief gives `JOB_DIR`.
- [ ] Parallel writer: its lines come from `.claude/skills/rex-deliver/parallel-writers.md`.
- [ ] Task detail from work breakdown goes in brief.
- [ ] Skills in brief: only `Skills` of task of that agent, from its row in work breakdown. Path of skill = its `Path` in skill list. Path of sibling file `<skill>/<file>.md` = folder of that `Path` + `<file>.md`. Agent reads these first. It also reads a sibling file when a `SKILL.md` that it read sends it there.
- [ ] Agent of lead, or of role with no task (`—` in `Tasks`): skills of its role, from role table.
- [ ] Lead: brief gives rows of tasks that it leads, with their Skills. Each worker brief of lead names only skills + sibling files of that worker's task.
- [ ] Grid agent has no Skill tool. It reads only files that its brief names, by absolute path. Write that in brief: template line below.
- [ ] Lead gets path of its lead skill in brief. Lead reads it IN FULL. Plain file read, not Skill tool.
- [ ] Brief is for agent with no context: full sentences.

Template:

```
Rex-Job: <job ID>
Rex-Role: <role>

Outcome: <one deliverable, and why it matters>.
Scope: <the files and state that you own, the prerequisites, the starting revision, and the edits or failures that exist already>.
Context: <the requirements, interfaces, decisions, and locations of artifacts that apply>. <Grid agent: You have no Skill tool.> Read these files in full with the Read tool, one file for each call, before you start: <absolute paths of the skills and sibling files of your task, or of your role>. Also read a sibling file when one of these SKILL.md files sends you there.
Acceptance: <the evidence that you must show to pass>.
Limits: <the time and effort, the concurrency, and when to escalate>.
Return: Give a short verdict and references: paths and commits. Map each acceptance criterion to its evidence. List the open issues. End the report with one state: done, partial, awaiting decision, or blocked.
```

## Limits of Claude Code

Read `.claude/skills/orchestration-builder/stages/3-orchestration/limits.md` before you start first lead, OR IF you are sub-agent.

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
- [ ] IF failures repeat, OR coordination costs more than delivery, THEN plan again: send evidence of failures to decision agent with SendMessage. IF it is gone, THEN start one new decision agent (brief of `.claude/skills/orchestration-builder/decide.md`) with last decision block + evidence. Work on with corrected block. Role that `rex job plan` did not print: start it with agent of its `Agent` column. Max 2 replans for each job. After that, report state `blocked` to requester.

Recovery:

| Situation | Action |
| --- | --- |
| Stage completes | Inspect result + acceptance evidence before next stage. Send defect of build to agent that built it. Send design or scope to lead. |
| Decision needed | Decide from requirements + decisions first. Send material ambiguity to parent, with options + recommendation. Wait for answer is different from failure. |
| Work stalls | Examine progress, failures, open questions at useful intervals. Silence alone is not failure. Before you replace active writer: stop it, or isolate its writes. |
| Run resumes | Compare checkpoint with actual files. Continue incomplete work. Do not repeat complete changes. Do not overwrite unrelated edits. |
| Result changes | After repair, clean-up, merge, or contract change: old evidence is not valid. Test final integrated result again. |
