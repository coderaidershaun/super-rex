# Limits of Claude Code

Layers of sub-agents. Decision agent reads this when orchestration has a lead. Builder that runs orchestrator skill reads it before it starts first lead.

- [ ] Depth: sub-agent of main session = depth 1. Sub-agent of an agent at depth `n` = depth `n + 1`.
- [ ] `rex init` sets `CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH` = 4. Agent at last depth cannot start agents. It has no Agent tool.
- [ ] Depth rule of mod: a spawn at depth 4 passes only IF agent at depth 1 above it is builder of a master tree. ELSE mod refuses it: `depth <n> is for master trees only; start this work one layer up`. Mod refuses also IF its check fails.
- [ ] Builder in main session: leads = depth 1, workers = depth 2, sub-agents of workers = depth 3.
- [ ] Builder that master started: builder = depth 1, leads = depth 2, workers = depth 3, sub-agents of workers = depth 4.
- [ ] So both builders get same layers: L1 builder -> L2 lead -> L3 worker, + one spare layer for a sub-agent of a worker (example: fallback reviewer of `rex-codex-adversarial-review`).
- [ ] Agent teams: teammate cannot start teammates. Teammate can start sub-agents.
- [ ] IF lead cannot start agents, THEN lead writes assignments. Orchestrator starts workers and gives their results to lead.
