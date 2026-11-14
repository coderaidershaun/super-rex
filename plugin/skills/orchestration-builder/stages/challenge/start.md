# Start challenger

Builder: steps 1 + 2 of `orchestration-builder/stages/challenge/stage.md`. You need not read that file. Words come from `orchestration-builder/SKILL.md`.

## Builder — select challenger

- [ ] Run `rex oms` with ratings of role `decision-challenger` in `## Builder roles` of `orchestration-builder/decide.md`, + job inputs (duration, budget, autonomy) that `rex_job_open` got in `prefs`.
- [ ] Start challenger with Agent tool. `subagent_type` = `super-rex:` + grid agent of `agent=` line.
- [ ] Size of job does not change agent. Only ratings do.

## Builder — brief

Brief = text for agent with no context: full sentences. Give absolute paths. First line = `Task: decision-challenger`. Mod gives it `orchestration-builder/stages/challenge/stage.md` + `orchestration-builder/stages/1-skills/assign.md` (`## Fixed manifests` of `orchestration-builder/SKILL.md`), and its output file.

```
Task: decision-challenger
Outcome: Challenge the decision that the orchestration builder made for this job, before work starts. Find what is missing or wrong, most of all important skills that the decision does not give to a task or to a role.
Scope: The job is <job ID>. You only read. Your output file is the one file that you write.
Context: You have no Skill tool. The <skills> block of this brief holds orchestration-builder/stages/challenge/stage.md and orchestration-builder/stages/1-skills/assign.md. Do each check in the section "Checks" of stage.md. The decision agent answered the questions of the assignment table through the rex_assign tool, and the mod recorded each answer with its reason: call the rex_status tool with job <job ID> and read each line that starts with decision. Run `rex skills list` to see every skill of the repo and of the user. Read the header of a skill of the plugin library with the rex_skill tool when you must. The request: <request text>. Classification: <work type, size, consequence, languages>. Preferences: <stated preferences, or none>. Merge answer: <yes or no>. Lessons of similar jobs: <lesson lines of rex_recall, or none>. Read the decision block in full: <absolute path of output file of decision agent>. It holds the skill list with the reason for each skill, the work breakdown with the skills of each task, the overrides with their reasons, the orchestrator skill (absolute path, new or reused), and the role table with the tasks, skills, and ratings of each role and the agent of `rex oms`.
Acceptance: Your output file holds the findings in the form of the section "Output" of stage.md. Your final message gives the count of findings for each severity.
Limits: Change no file except your output file, and run no command that writes. Give one set of findings, then stop.
If no <skills> block is at the top of this brief, call rex_skill with no name first.
```

Then send path of findings file to decision agent: step 5 of `## Step 6 — Decide` in `orchestration-builder/decide.md`.
