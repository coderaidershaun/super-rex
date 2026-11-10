# Start challenger

Builder: steps 1 + 2 of `.claude/skills/orchestration-decision-challenge/SKILL.md`. You need not read that file. Words come from `.claude/skills/orchestration-builder/SKILL.md`.

## Builder — select challenger

| Job | Challenger agent |
| --- | --- |
| Size small or medium. Consequence low or medium. | `model-high-effort-high` |
| Size large, OR consequence high. | `model-xhigh-effort-high` |

- [ ] Start it with Agent tool. `subagent_type` = that grid agent.
- [ ] IF you have no Agent tool (last level of sub-agents), THEN skip challenge. Write `DECISION CHECK: skipped, no Agent tool` in work breakdown. Tell agent that started you.

## Builder — brief

Brief = text for agent with no context: full sentences. Give absolute paths.

```
Rex-Job: <job ID>
Rex-Role: decision-challenger

Outcome: Challenge the decision that the orchestration builder made for this job, before work starts. Find what is missing or wrong, most of all important skills that the decision does not give to a task or to a role.
Context: Read <absolute path of .claude/skills/orchestration-decision-challenge/SKILL.md> in full first, and do each check in its section "Checks". Run `rex skills list` to see every skill that is available. The request: <request text>. Classification: <work type, size, consequence, languages>. Preferences: <stated preferences, or none>. Lessons of similar jobs: <lesson lines of rex recall, or none>. The decision block, unchanged: <decision block>. It holds the skill list with the reason for each skill, the work breakdown with the skills of each task, the orchestrator skill (absolute path, new or reused), and the role table with the tasks, skills, and ratings of each role and the agent of `rex oms`.
Limits: You only read. Change no file, and run no command that writes. Give one set of findings, then stop.
Return: The findings in the form of the section "Output" of the skill. End the report with one state: done, partial, awaiting decision, or blocked.
```

Then send findings to decision agent: step 5 of `.claude/skills/orchestration-builder/decide.md`.
