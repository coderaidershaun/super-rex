# Interface Design

IF you need alt interfaces for chosen deepening candidate, THEN use parallel sub-agent pattern. Based on "Design It Twice" (Ousterhout) — first idea unlikely best. Autonomous: no user questions, decide self.

Vocab from [LANGUAGE.md](LANGUAGE.md) — **module**, **interface**, **seam**, **adapter**, **leverage**.

## Process

### 1. Frame problem space

Write problem space for chosen candidate (goes in final report):

- Constraints any new interface must satisfy
- Deps it relies on + which category (see [DEEPENING.md](DEEPENING.md))
- Rough illustrative code sketch that grounds constraints — not proposal, only makes constraints concrete

Then immediate Step 2.

### 2. Spawn sub-agents

Spawn 3+ sub-agents parallel via Agent tool. Each = **radically different** interface for deepened module.

Prompt each w/ separate technical brief (file paths, coupling details, dep category from [DEEPENING.md](DEEPENING.md), what sits behind seam). Each agent = different design constraint:

- Agent 1: "Minimize interface — 1–3 entry points max. Maximise leverage per entry point."
- Agent 2: "Serve first quality of priority line above all. Say what it costs."
- Agent 3: "Optimise for most common caller — default case trivial."
- Agent 4 (only IF growth is not "none", or dep is category 3 or 4): "Design around interface at seam + adapters for cross-seam deps."

Include both [LANGUAGE.md](LANGUAGE.md) vocab + domain words from code in brief. Purpose: each sub-agent names things consistent w/ arch language + project domain language.

Each sub-agent outputs:

1. Interface (types, methods, params + invariants, ordering, error modes)
2. Usage example that shows how callers use it
3. What impl hides behind seam
4. Dep strategy + adapters (see [DEEPENING.md](DEEPENING.md))
5. Trade-offs — where leverage high, where thin

### 3. Compare + decide

Compare in prose. Contrast by **depth** (leverage at interface), **locality** (where change concentrates), **seam placement**.

Pick winner self. IF elements from different designs combine, THEN propose hybrid. Be opinionated.

IF close call between two strong designs, THEN dispatch one more fresh subagent (Agent tool) with both designs + trade-off. Take its recommendation as input. Then decide. Never give decision to user.
