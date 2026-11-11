# 2. Rank

Words, rules + SIZE CHECK: `.rex/skills/rex-code-improve-codebase-architecture/SKILL.md`. Facts come from step 1 (`.rex/skills/rex-code-improve-codebase-architecture/explore.md`).

Write numbered list. Sort: 1. friction on main path or failed growth test (1.5). 2. leverage x locality gain. Each candidate:

| Field | Content |
| --- | --- |
| Files | Files + modules in candidate |
| Problem | Friction, with file:line evidence |
| Solution | Plain words: what changes |
| Kind | Move (no code change) OR redesign (interface or logic changes) |
| Benefits | Locality + leverage + test gain |
| Ergonomics | Result of ERGONOMICS CHECK |
| Risk | Hidden behavior, public path change, size of diff |
| Quality | Quality of priority line that candidate serves. Quality that it costs, or "none". |

Rules:

- [ ] Use domain words from code for domain. Use Words table for architecture. Say "Order intake module", not "FooBarHandler".
- [ ] Each candidate must pass ERGONOMICS CHECK. IF it fails, THEN drop it or change it.
- [ ] Each candidate must obey each rule of `rex-code-folder-management`.
- [ ] Obey USE in `rex-code-priorities`. IF candidate costs a higher quality or breaks a Never line, THEN drop it.
- [ ] Candidate fits pattern from 1.5. New pattern needs reason from priority line.
- [ ] IF growth = "none", THEN no new seam, trait, or generic for growth.
- [ ] IF candidate conflicts with warning comment or fitness function, THEN list it only when friction is large. Mark it: "contradicts warning at src/feed.rs:42, open again because …".
- [ ] One run = one kind. Join move candidates into one run. Never mix move + redesign.
- [ ] IF scope has move candidates + redesign candidates, THEN do move first. Clean tree makes redesign small.
- [ ] Pick top candidate yourself. IF tie between top 2 or 3, THEN get second opinion, then decide.
- [ ] Keep list of candidates not taken, each with one-line reason. It goes in report.

IF lens mode, THEN stop here + report.

## ERGONOMICS CHECK

Each candidate must answer all. IF one answer is bad, THEN think again before you propose it.

| Question | Good answer |
| --- | --- |
| Is caller code shorter after? | Shorter |
| How many files does reader open for one concept after? | Fewer |
| Does each name match word in code or domain word from task? | Yes |
| Is autocomplete list at interface easier to read? | Yes |
| Does error reach caller with context? | Yes |
| Can reader name jobs of crate from tree alone? | Yes |
