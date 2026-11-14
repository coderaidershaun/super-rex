# WRITE: orchestrator only

Words (priority block, Priority line, Never line, quality, root file), Shape of block, and CAUTION about change of block are in `rex-code-priorities/SKILL.md`.

Ask user these questions one time, with other up-front questions. Give recommended answer for each, from task + code.

1. Rank these for this repo: correctness, time to ship, extensibility, performance, cost. Which 2 to 4 are important, in which order?
2. What will there be more of later? (Example: exchanges, file formats, tenants, payment providers.)
3. What must this system never do, even one time?
4. For each quality below first: what is good enough?

Then:

- [ ] Write priority block in module comment of root file.
- [ ] Read it back to user in report.
