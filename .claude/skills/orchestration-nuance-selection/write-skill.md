# Write new skill

Steps 4 + 5 of `.claude/skills/orchestration-nuance-selection/SKILL.md`. Builder does them at step 5 of `.claude/skills/orchestration-builder/decide.md`, IF decision block lists NEW SKILLS. Do them once for each row: `Name`, `Need`, `Reason`, `Gap language`.

- `JOB_DIR`: Absolute path where job branch is checked out. `.claude/skills/rex-deliver/start.md` gives it.
- Tag block: First lines of brief: `Rex-Job: <job ID>`, `Rex-Role: <role>`, then one blank line.

## Step 4 — Write skill

Session agent writes new skill.

1. Name = `Name` of row.
2. Run `git -C "$HOME_DIR" check-ignore -q .claude/skills/<name>/SKILL.md`.
3. IF it succeeds, THEN Git ignores new skill. `<root>` = `HOME_DIR`. No branch is necessary.
4. ELSE new skill = change of repo. Builder makes values of table of step 7 first (`.claude/skills/orchestration-builder/run.md`). Then builder reads `.claude/skills/rex-deliver/SKILL.md`, then `.claude/skills/rex-deliver/start.md`, for that repo. `<root>` = `JOB_DIR`.
5. Start session agent: `subagent_type` = `general-purpose`, no model. Do not call `rex oms` for it.
6. Give it no worktree isolation. Skill goes into `<root>`.
7. Give it brief below. It starts with tag block. Replace each `<…>`. Each path is absolute.

- [ ] Sandbox refuses shell command that writes below `.claude/skills/`. Protection is part of sandbox, not of settings.
- [ ] Write tool and Edit tool can write below `.claude/skills/`, in session and in sub-agent.

Brief template:

```
Rex-Job: <job ID>
Rex-Role: skill-writer

Write one new skill. Its name is <name>. The skill must make agents do this: <need>.
The reason: <why this job needs it, and why later jobs need it also>.

1. Read <HOME_DIR>/.claude/skills/rex-skill-language/SKILL.md in full. Obey it in every line.
2. Write the file <root>/.claude/skills/<name>/SKILL.md with the Write tool.
   A shell command cannot write below .claude/skills/.
3. Start the file with this header:
   ---
   name: <name>
   description: >
     <What the skill is for, in one to three full sentences. No "use when" text here.>
   when_to_use: >
     Use when <the situations in which an agent must load it>.
   disable-model-invocation: false
   user-invocable: true
   ---
4. Keep description and when_to_use at 60 words or fewer together, and the text after the header at 1,500 words or fewer. Put the text for one step or one case in a sibling .md file in the same folder, and say in SKILL.md when to read it.
5. The skill is not about one task. Put nothing in it that only this job needs.
6. <Gap skill only: The skill covers what <language> does not give by default and Rust does.
   Write each such protection as a rule. Give each code example in Rust, beside the same example in <language>.>
7. Run: rex skills check <root>/.claude/skills/<name>
8. <Only if Git ignores the new skill: Do not add the file to Git.>

Return a short report: the path of the file and the output of the check.
```

## Step 5 — Skill check

After session agent reports, run `rex skills check <root>/.claude/skills/<name>` on each new skill. Path = skill folder, or its `SKILL.md`.

- [ ] Pass: exit code 0 and line `ok <path>`.
- [ ] Finding: line `<path>: <rule>: <what is wrong>`. Exit code 1.
- [ ] DO correct skill with Edit tool UNTIL check passes OR 3 rounds done.
- [ ] IF check passes, THEN skill goes in skill list, with mark `new`.
- [ ] IF check still fails, THEN do not use skill. Tell requester in final report: path + last check output.
- [ ] Give path + check result of each new skill to decision agent, with findings of challenger. Decision agent changes skill list.
