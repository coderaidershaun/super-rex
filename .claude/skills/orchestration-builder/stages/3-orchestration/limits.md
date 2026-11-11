# Limits of Claude Code

Layers of sub-agents. Decision agent reads this when orchestration has a lead, OR orchestrator skill runs below level 1. Agent that runs orchestrator skill reads it before it starts first lead, OR IF it is sub-agent.

- [ ] Default max = 3 layers of sub-agent below main session. `CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH` changes it.
- [ ] L1 -> L2 -> L3 uses 2 layers. One layer is spare. IF orchestrator is sub-agent (builder inside open job), THEN it has fewer layers below it.
- [ ] Agent at last layer cannot start agents. It has no Agent tool.
- [ ] Agent teams: teammate cannot start teammates. Teammate can start sub-agents.
- [ ] IF lead cannot start agents, THEN lead writes assignments. Orchestrator starts workers and gives their results to lead.
