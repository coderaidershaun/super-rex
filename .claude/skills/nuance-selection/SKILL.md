---
name: nuance-selection
description: >
  This is stage 1 of the orchestration builder. It lists the skills that are
  available and selects the skills that a job uses. It writes a new skill only
  when a three-part test passes. The usual result is no new skill. Use this
  skill when `orchestration-builder` reaches stage 1, or when an agent must
  select skills for a piece of work.
disable-model-invocation: false
user-invocable: true
# PENDING(C7): form of the "use when" line and the header keys.
---

# Nuance Selection

Settle which skills this job uses. Write new skill only IF test of step 3 passes. Default result: no new skill.

```
1. List skills.
2. Find needs of job. IF skill in list covers need, THEN add that skill to skill list.
3. Test each need that no skill covers. IF test fails, THEN write no skill.
4. IF test passes, THEN session agent writes new skill.
5. Do skill check on each new skill.
6. Give SKILL LIST to builder.
```

## Words

- Skill: Folder with one `SKILL.md`. Its header gives name, description, and "use when" line.
- Session: Claude Code conversation that loaded builder. It has one session ID.
- Builder: Agent that obeys `orchestration-builder`.
- Requester: User or agent that gave request.
- Job: One request that SuperRex handles. It has one job ID.
- Classification: Four fields of job: work type, size, consequence, languages.
- Skill list: Skills this job uses. Output of stage 1.
- Nuanced skill: Skill for something particular to repo or to kind of work, that no other skill covers.
- Gap skill: Nuanced skill for one language. It covers what Rust gives by default and that language does not.
- Session agent: `general-purpose` sub-agent that you start with no model. It runs at model + effort of session.
- Main checkout: Repo folder that holds `.claude/skills/` and `.rex/`. IF you work in Git worktree, THEN main checkout = parent folder of `git rev-parse --path-format=absolute --git-common-dir`.
- Brief: Prompt that sub-agent gets.
- Tag lines: First lines of brief. They give job ID and role.
- Paths: each path in this skill starts at main checkout.

## Input

From builder: request, classification, recall output, job ID.

## Step 1 — List skills

Run `rex skills list`. `PENDING(C7)`: output form.

- [ ] It prints name, description, and "use when" line of each skill: in repo and in user skill folder of Claude Code.
- [ ] Decide from these lines. Do not open each file.
- [ ] Open skill file only IF its lines are not sufficient.

## Step 2 — Find needs

From request, classification, lessons of recall output, and look at repo: find what this job needs that changes what agents do.

- [ ] For each need: find skill in list that covers it.
- [ ] IF skill covers need, THEN add that skill to skill list. Write nothing new.
- [ ] IF no skill covers need, THEN do step 3 for it.
- [ ] For each language in classification that is not `rust`: do `## Gap skill`.

Examples of need: ergonomics are very important in this repo. Repo is very large, and architecture must get attention after work. Skill in list can cover each one already. Then add nothing new.

Work can be code, research, or other work. Pre-installed rulebooks are for code. For other work, user supplies skills of that field.

## Step 3 — Test

New skill only IF all three are true:

- [ ] It changes what agents do.
- [ ] No skill in list covers it.
- [ ] It is important after this job also.

IF one is false, THEN write no skill. Content that only this job needs goes in brief.
IF in doubt, THEN write no skill. Add skill only IF it is necessary.
Never add rule to skill that exists. Only `rex-feedback` does that.

## Step 4 — Write skill

Session agent writes new skill. It runs at model + effort of session.

1. Select name: short lower-case words with hyphens. Name = folder name. No skill in list of step 1 has that name.
2. Start session agent: `subagent_type` = `general-purpose`, no model. Do not call `rex oms` for it.
3. Give it no worktree isolation. Skill goes into main checkout. Agent in worktree cannot write there.
4. Give it brief below. Replace each `<…>`. `<main checkout>` = absolute path.

- [ ] Sandbox refuses shell command that writes below `.claude/skills/`. Protection is part of sandbox, not of settings.
- [ ] Write tool and Edit tool can write below `.claude/skills/`, in session and in sub-agent.
- [ ] New skill stays out of Git, like all files that SuperRex adds. `PENDING(C6)`: whether ignore entries cover new skill.

Brief template. `PENDING(C4)`: form of tag lines. `PENDING(C7)`: header keys and "use when" line.

```
<tag line: job ID = <job ID>>
<tag line: role = skill-writer>

Write one new skill. Its name is <name>. The skill must make agents do this: <need>.
The reason: <why this job needs it, and why later jobs need it also>.

1. Read <main checkout>/.claude/skills/rex-skill-language/SKILL.md in full. Obey it in every line.
2. Write the file <main checkout>/.claude/skills/<name>/SKILL.md with the Write tool.
   A shell command cannot write below .claude/skills/.
3. Start the file with this header:
   ---
   name: <name>
   description: >
     <What the skill does, in one to three full sentences.>
     Use this skill when <the situation in which an agent must load it>.
   disable-model-invocation: false
   user-invocable: true
   ---
4. The skill is not about one task. Put nothing in it that only this job needs.
5. <Gap skill only: The skill covers what <language> does not give by default and Rust does.
   Write each such protection as a rule. Give each code example in Rust, beside the same example in <language>.>
6. Run: rex skills check <main checkout>/.claude/skills/<name>/SKILL.md
7. Do not add the file to Git.

Return a short report: the path of the file and the output of the check.
```

## Gap skill

IF job writes code in language that is not `rust`, THEN find skill in list for gaps of that language.

- [ ] IF skill covers gaps of that language, THEN add it to skill list.
- [ ] ELSE gap skill = need. It must pass test of step 3 also.
- [ ] Content: each protection that Rust gives by default and that language does not give, as rule that gives same protection.
- [ ] Code examples: Rust (reference) beside that language.
- [ ] Go: error handling. C++: `const` correctness and memory safety, no chance of segmentation fault.

## Step 5 — Skill check

After session agent reports, run `rex skills check <main checkout>/.claude/skills/<name>/SKILL.md` on each new skill. `PENDING(C7)`: argument form and rules of check.

- [ ] DO correct skill with Edit tool UNTIL check passes OR 3 rounds done.
- [ ] IF check passes, THEN add skill to skill list, with mark `new`.
- [ ] IF check still fails, THEN do not use skill. Tell requester in final report: path + last check output.

## Output

SKILL LIST: one table. Columns, in this sequence: `Skill / Path / Why this job needs it`.

- [ ] Mark each new skill in `Skill` column: `<name>` (`new`).
- [ ] Default: no row is `new`.
- [ ] Builder records skill list with plan.
