---
name: rex-skill-language
description: >
  Language for skills, prompts, and agent instructions. It is Simplified
  Technical English (ASD-STE100) with caveman compression. One word has one
  meaning. Lines are short, with no articles and no filler. Conditions are
  IF/THEN, loops are DO/UNTIL, and important items are checklists. Use this
  skill when you write, rewrite, or shorten a skill, a SKILL.md, a system
  prompt, or agent instructions. Use it also when the user says "skill
  language", "STE", "STE100", "ASD-STE100", "simplified technical English",
  "controlled language", "caveman the skill", or "make this skill shorter".
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
