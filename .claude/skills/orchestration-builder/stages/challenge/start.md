# Start challenger

Builder: steps 1 + 2 of `.claude/skills/orchestration-builder/stages/challenge/stage.md`. You need not read that file. Words come from `.claude/skills/orchestration-builder/SKILL.md`.

## Builder — select challenger

- [ ] Run `rex oms` with ratings of role `decision-challenger` in `## Builder roles` of `.claude/skills/orchestration-builder/decide.md`, + job inputs (duration, budget, autonomy) that `rex job create` got.
- [ ] Start challenger with Agent tool. `subagent_type` = grid agent of `agent=` line.
- [ ] Size of job does not change agent. Only ratings do.

## Builder — brief

Brief = text for agent with no context: full sentences. Give absolute paths.

```
Rex-Job: <job ID>
Rex-Role: decision-challenger

Outcome: Challenge the decision that the orchestration builder made for this job, before work starts. Find what is missing or wrong, most of all important skills that the decision does not give to a task or to a role.
Context: You have no Skill tool. With the Read tool, read <absolute path of .claude/skills/orchestration-builder/stages/challenge/stage.md> in full first, then <absolute path of .claude/skills/orchestration-builder/stages/1-skills/assign.md>, and do each check in the section "Checks". Run `rex skills list` to see every skill that is available. The request: <request text>. Classification: <work type, size, consequence, languages>. Preferences: <stated preferences, or none>. Merge agreement: <yes or no, for each repo>. Lessons of similar jobs: <lesson lines of rex recall, or none>. Read the decision block in full: <absolute path of output file of decision agent>. It holds the skill list with the reason for each skill, the work breakdown with the skills of each task, the orchestrator skill (absolute path, new or reused), and the role table with the tasks, skills, and ratings of each role and the agent of `rex oms`.
Limits: You only read. Change no file except your output file, and run no command that writes. Give one set of findings, then stop.
Return: Write the findings in the form of the section "Output" of the skill to <HOME_DIR>/.rex/plans/<job ID>/decision-challenger.md. Your final message has at most 2,000 characters: the path of that file, the count of findings for each severity, and one state: done, partial, awaiting decision, or blocked.
```

Then send path of findings file to decision agent: step 5 of `.claude/skills/orchestration-builder/decide.md`.
