# Write new skill

Steps 4 + 5 of `orchestration-builder/stages/1-skills/stage.md`. Builder does them at step 3 of `## Step 6 — Decide` in `orchestration-builder/decide.md`, IF decision block lists NEW SKILLS. Do them once for each row: `Name`, `Need`, `Reason`, `Gap language`.

- `JOB_DIR`: Absolute path where job branch is checked out: `worktree` of `rex_job_open`. `rex-deliver/start.md` makes it.

## Step 4 — Write skill

Grid agent of role `skill-writer` writes new skill as repo skill: `.rex/skills/<name>/`. `rex_skill` finds it there, so plan can give it to each task.

1. Name = `Name` of row. Gap skill: `rex-gap-<language>` (`## Output` of `orchestration-builder/stages/1-skills/stage.md`).
2. Run `git -C "$HOME_DIR" check-ignore -q .rex/skills/<name>/SKILL.md`.
3. IF it succeeds, THEN Git ignores new skill. `<root>` = `HOME_DIR`. No branch is necessary.
4. ELSE new skill = change of repo. Builder makes values of table of step 8 first (`orchestration-builder/run.md`). Then builder reads `rex-deliver/SKILL.md`, then `rex-deliver/start.md`, for that repo. `<root>` = `JOB_DIR`. This START comes before plan: exception to item 3 of `## Step 8 — Run` of `orchestration-builder/run.md`. Path of new skill starts at `JOB_DIR`, not at `HOME_DIR`.
5. Start grid agent that `rex oms` gives for role `skill-writer`: ratings of `## Builder roles` in `orchestration-builder/decide.md`. `subagent_type` = `super-rex:` + that grid agent.
6. Give it no worktree isolation. Skill goes into `<root>`.
7. Give it brief below. First line = `Task: skill-writer`. Replace each `<…>`. Each path is absolute. Mod gives it `rex-skill-language` (`## Fixed manifests` of `orchestration-builder/SKILL.md`).

Brief template:

```
Task: skill-writer
Outcome: Write one new skill. Its name is <name>. The skill must make agents do this: <need>. The reason: <why this job needs it, and why later jobs need it also>. The skill is not about one task, so put nothing in it that only this job needs.
Scope: You own one folder: <root>/.rex/skills/<name>/. Write no other file except your output file.
Context: The <skills> block of this brief holds the rex-skill-language skill. Obey it in every line. Write the file <root>/.rex/skills/<name>/SKILL.md with the Write tool. Start the file with this header:
   ---
   name: <name>
   description: >
     <What the skill is for, in one to three full sentences. No "use when" text here.>
   when_to_use: >
     Use when <the situations in which an agent must read it>.
   ---
Keep description and when_to_use at 60 words or fewer together, and the text after the header at 1,500 words or fewer. Put the text for one step or one case in a sibling .md file in the same folder, and say in SKILL.md when to read it. <Gap skill only: The skill covers what <language> does not give by default and Rust does. Write each such protection as a rule. Give each code example in Rust, beside the same example in <language>.>
Acceptance: `rex skills check <root>/.rex/skills/<name>` prints `ok` for the folder. Your report gives the path of the file and the output of the check.
Limits: Start no agent. Change no other skill. <Only if Git ignores the new skill: Do not add the file to Git.>
If no <skills> block is at the top of this brief, call rex_skill with no name first.
```

## Step 5 — Skill check

After skill writer reports, run `rex skills check <root>/.rex/skills/<name>` on each new skill. Path = skill folder, or its `SKILL.md`.

- [ ] Pass: exit code 0 and line `ok <path>`.
- [ ] Finding: line `<path>: <rule>: <what is wrong>`. Exit code 1.
- [ ] DO correct skill with Edit tool UNTIL check passes OR 3 rounds done.
- [ ] IF check passes, THEN skill goes in skill list, with mark `new`.
- [ ] IF check still fails, THEN do not use skill. Tell requester in final report: path + last check output.
- [ ] Give path + check result of each new skill to decision agent, with findings of challenger. Decision agent changes skill list.
