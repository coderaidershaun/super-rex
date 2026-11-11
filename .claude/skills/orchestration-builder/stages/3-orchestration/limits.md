# Limits of Claude Code

Layers of sub-agents. Decision agent reads this when orchestration has a lead. Session that runs orchestrator skill reads it before it starts first lead.

- [ ] Default max = 3 layers of sub-agent below main session. `CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH` changes it.
- [ ] L1 -> L2 -> L3 uses 2 layers. One layer is spare.
- [ ] Agent at last layer cannot start agents. It has no Agent tool.
- [ ] Agent teams: teammate cannot start teammates. Teammate can start sub-agents.
- [ ] IF lead cannot start agents, THEN lead writes assignments. Orchestrator starts workers and gives their results to lead.
