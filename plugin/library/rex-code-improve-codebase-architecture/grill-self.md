# 3. Grill self

Words + rules: `rex-code-improve-codebase-architecture/SKILL.md`. Chosen candidate comes from step 2 (`rex-code-improve-codebase-architecture/rank.md`).

Walk design tree for chosen candidate alone. Write each answer. Answers become brief for step 4.

- [ ] Constraints: what must not change? Public paths, saved formats, messages, prompts, tests.
- [ ] Dependencies: category from `rex-code-improve-codebase-architecture/DEEPENING.md` for each one.
- [ ] Shape: interface of each module after change. What is behind each seam?
- [ ] State: each piece of state has one owner module after change.
- [ ] Tree: target tree, with one line of purpose for each file. Make sure AUDIT (`rex-code-folder-management/audit.md`) gives zero defects for it.
- [ ] Names: each new name is word that code uses, or domain word from side effects below. No invented word.
- [ ] Tests: which tests stay, which move with code, which go. Default: add zero tests for move.
- [ ] Hidden behavior: each item + how Proof covers it.
- [ ] Next work: each import from 1.1 is at interface after change.
- [ ] Weakest part: which part is forced by rule and not by real joint? Say so in brief.

IF kind = move, THEN write these also. Format: `rex-code-folder-management`, section MOVE (`rex-code-folder-management/move.md`).

- [ ] Move map.
- [ ] Closed list of edits that are not moves.
- [ ] Public path table, before + after.
- [ ] Step order where build is green after each step.

IF kind = redesign, THEN obey `rex-code-improve-codebase-architecture/INTERFACE-DESIGN.md`: 3+ different interfaces, compare, pick.

Side effects. Do each one now, not later:

| Condition | Action |
| --- | --- |
| New module name = concept that code does not name | Take domain word from task. Put it in brief as module name + module comment (`rex-code-commenting`). |
| Vague term gets sharp during design | Use sharp term in brief for each new name. |
| Candidate rejected for load-bearing reason that next explorer needs | Put reason in brief as permanent warning comment for builder to write where it matters (`rex-code-commenting`, whitelist 2). Skip reasons that are obvious or short-lived. |
| Stuck on trade-off | Second opinion. Not user. |
