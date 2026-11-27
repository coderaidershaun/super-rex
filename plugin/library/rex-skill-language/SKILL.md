---
name: rex-skill-language
description: >
  Language for skills, prompts, and agent instructions: Simplified Technical
  English (ASD-STE100) with caveman compression. One word, one meaning. Short
  lines. IF/THEN, DO/UNTIL, checklists.
when_to_use: >
  Use when you write, rewrite, or shorten a skill, SKILL.md, system prompt, or
  agent instructions, or user says "skill language", "STE", "STE100",
  "ASD-STE100", "simplified technical English", "controlled language",
  "caveman the skill", "make this skill shorter".
metadata:
  kind: rule
  category: agent-instructions
  claims:
    artefact: [instructions]
  placement: full
  intents:
    - write a skill
    - shorten a prompt
    - write agent instructions in controlled English
---

# Skill Language

STE + caveman. One word, one meaning. Fewest words that keep the meaning.

Never change technical meaning. Never change code, commands, paths, or quoted text.

## Procedure

```
FOR EACH line:
    Apply WORDS, VERBS, CUT.
    IF line has a condition, THEN write IF … THEN … ELSE.
    IF line has a loop, THEN write DO … UNTIL.
    IF line is an important item, THEN put it in a checklist.
    IF line is in KEEP FULL, THEN write a full sentence.
    IF line tells about a risk, THEN apply SAFETY.
    IF text is a library skill, THEN also apply METADATA.
DO read text again UNTIL text obeys all rules.
Give text only. IF user asks for changes, THEN list them.
```

Not: "Before you commit, you should really make sure that all of the tests are passing, and keep fixing until they do."

Yes:

```
DO correct failures UNTIL all tests pass OR 3 rounds done.
IF all tests pass, THEN commit.
```

## STRUCTURE

- [ ] Condition: `IF <condition>, THEN <action>. ELSE <action>.`
- [ ] Loop: `DO <work> UNTIL <condition>.` Always give a limit: `OR 3 rounds done`.
- [ ] Important items (rules, gates, report contents): checklist with `- [ ]`.
- [ ] Sequence is important: numbered list.
- [ ] Parallel data: table.
- [ ] Special term: define it one time in a Words section. Then use only that term.
- [ ] One instruction per line. One topic per section.
- [ ] Max 20 words per sentence.

## CUT

- [ ] Drop articles: a, an, the.
- [ ] Drop filler: just, really, basically, simply, actually.
- [ ] Drop pleasantries and hedging.
- [ ] Fragments OK. Pattern: `Label: content`.
- [ ] Short forms OK: `+` for "and", `=` for "is", `max 3`.
- [ ] One word when one word is sufficient. Use shortest common word.
- [ ] IF a cut gives the line two meanings, THEN do not make that cut.

## KEEP FULL

Write full sentences, with articles, for these:

- [ ] Safety text: WARNING, CAUTION.
- [ ] Actions that you cannot undo: delete, git, funds, data.
- [ ] Text for a reader with no context: a prompt for a different agent.
- [ ] Text for a person: manual, procedure, runbook. Here use STE only. Do not apply CUT.

## SAFETY

- [ ] IF persons can get injuries, THEN write WARNING. IF only equipment or data can get damage, THEN write CAUTION.
- [ ] Put safety instruction before the step.
- [ ] Start with command or condition. Then give risk: "CAUTION: Do not delete a flag. A deleted flag hides a known problem."

## VERBS

- [ ] Start instruction with command verb: "Remove cover."
- [ ] IF there is a condition, THEN write it first: "IF lamp is red, THEN stop pump."
- [ ] IF voice is passive, THEN make it active.
- [ ] Use only simple present, simple past, or simple future.
- [ ] IF verb has "-ing" form or complex form ("is running", "has been set", "would open"), THEN use simple tense.
- [ ] IF noun gives action ("make an adjustment"), THEN use verb ("adjust").
- [ ] IF verb is phrasal verb ("put out"), THEN use one-word verb ("extinguish").

## WORDS

- [ ] IF word can have two meanings in line, THEN replace it.
- [ ] IF one thing has two names, THEN use one name everywhere.
- [ ] IF noun cluster has more than 3 nouns, THEN divide it with "of" or "for", or add hyphens.
- [ ] No contractions. No semicolons.
- [ ] American spelling.
- [ ] IF word is in this table, THEN replace it.

| Not approved                 | Approved  |
| ---------------------------- | --------- |
| utilize                      | use       |
| ensure, verify, check (verb) | make sure |
| should, need to, have to     | must      |
| may, might, could            | can       |
| prior to                     | before    |
| in order to                  | to        |
| begin, commence              | start     |
| perform, carry out           | do        |
| follow (a rule)              | obey      |
| enable, allow                | let       |

- [ ] IF not sure about a word, THEN use shortest common word that has one meaning.

## METADATA

Library skill: skill of `plugin/library/`, or repo skill of `.rex/skills/` (new skill, fork). A person cannot call it by slash command. Agents read it with `rex_skill`.

Header of a library skill = `name`, `description`, `when_to_use`, and one `metadata` map. Rules below apply to that map. Each rule names the check of `rex skills check` that finds a breach.

- [ ] Header holds `metadata` with `kind` and `category`. No check: a header with no map gives no finding. Make sure of it by eye.
- [ ] Use terms of `plugin/vocab/terms.json` only. IF term is wrong, THEN `rex skills check` prints the allowed terms. Use one of them. Do not invent a term.
- [ ] Other keys under `metadata`, such as `author`, are ignored. A map with none of the keys below gives no finding.
- [ ] After each edit of header or of files of skill folder, run `rex skills check <skill folder>`. DO correct UNTIL it prints `ok` OR 3 rounds done.

| Key | Rule | Check |
| --- | --- | --- |
| `kind` | Required. One term: what sort of skill. | M1 |
| `category` | Required. One term: subject of skill. | M2 |
| `claims` | Fields: `facts`, `activity`, `touches`, `quality`, `artefact`. Terms only. `facts` = map (`writes_code: true`, `languages: [ts]`). Other fields = lists. | M3 |
| `placement` | `full` or `open-when`. Default form of the skill. | M4 |
| `scope` | Condition that code checks. Key = scope key of vocabulary. Value = list of 1 or more allowed values. | M5 |
| `not_when` | One line: case in which skill does not apply. Required when `scope` has a key. | M6 |
| `intents` | 3 to 5 phrases. Each one = request that skill answers. None empty. | M7 |
| `files` | One item for each file of folder, but `SKILL.md`. Item: `path` and `role`. `when` is optional: text of the case to open the file. | M8, M9 |
| `open_when` | Item names what to open, and the case in `when`. `when` is required. Target must exist. | M10 |
| `requires`, `replaces` | Lists of names of skills that exist. | M11 |
| `max_topics` | Positive whole number: most topic files for one request. | M12 |

`files` item:

- [ ] `role`: `topic`, `language`, or `code` (code twin of a topic file). Other `role` = M8 finding.
- [ ] Role `language` or `code`: key `language` is a language term of vocabulary. M8.
- [ ] `intents` of a file: 1 to 5 phrases. M8.
- [ ] File of folder not in `files`, or item with no file: M9. Hidden files (`.DS_Store`) do not count.

`open_when` item:

- [ ] `{ skill: <name>, when: <case> }`: another skill.
- [ ] `{ file: <path>, when: <case> }`: file of this skill.
- [ ] `{ skill: <name>, file: <path>, when: <case> }`: file of another skill.
- [ ] IF body sends agent to another skill or to a file of it, THEN declare it as item `{ skill, when }` or `{ skill, file, when }`, with the same case. No check: B4 only finds a missing file, and M10 reads only declared items.

Other rules:

- [ ] Size: fields above hold max 120 words together (M13). Keys and file paths do not count. Number, `true`, `false` = 1 word each. IF over, THEN give `when` to fewer files.
- [ ] IF edit adds, removes, or renames a file of folder, THEN same edit changes `files` (M9) + each `open_when` item that names it (M10).
- [ ] `claims`: write only what skill is for. A word in the text is not a claim. No check.
- [ ] General rule that applies to code in any language: no `scope.languages`. No check.
- [ ] Each phrase of `intents` names a request that this skill answers. No check.
