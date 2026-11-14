# On request

`## On request` of `orchestration-builder/SKILL.md`. Uses its words and rules.

- [ ] IF requester asks for review of pull request, THEN start fresh grid agent, role `pr-reviewer`: `rex oms` gives it from ratings of `## Builder roles` in `orchestration-builder/decide.md`. It wrote no part of change. First add its task: `rex_job_plan` with `amend: true`, same `orchestrator` + `breakdown`, `tasks` = `[{ id: "pr-review", role: "pr-reviewer", skills: ["rex-pr-review"], open_when: [] }]`, `roles` = its ratings. `subagent_type` = `super-rex:` + that grid agent. Brief: template of `orchestration-builder/stages/3-orchestration/briefs.md`, first line `Task: pr-review`, with `PR` or `BASE` + `HEAD`, and `JOB`. Give its verdict to requester.
- [ ] IF requester asks for feedback on job, THEN read `rex-feedback/SKILL.md` IN FULL and obey it.
