Status: DRAFT

# C7. Skills: the header, `rex skills list` and `rex skills check`

Written by Lane 6. Read by Lanes 4, 8, 9 and 10. Lanes 8, 9 and 10 write their skills to this contract. Lane 4 builds the bundle and the installer around it. This file is complete on its own: a reader needs no other file.

Words used here:

- **Skill**: a folder that holds a file named `SKILL.md`. The folder name is the skill's name.
- **Skills folder**: the folder that holds skill folders, such as `.claude/skills/`.
- **Header**: the YAML block at the top of `SKILL.md`, between two `---` lines. Claude Code calls it frontmatter.
- **Bundle**: the skills that `rex` carries inside its binary and that `rex init` installs.
- **Skill file**: `SKILL.md` or any other `.md` file inside a skill folder.

---

## 1. Open points

### OP7 — proposed: there is no `INDEX.md`

SuperRex ships no `.claude/skills/INDEX.md`. `rex skills list` does the job the index did.

Reasons:

1. **One source.** `rex skills list` reads each skill's own header. An index is a second copy of the same facts. rex-skills needed a test only to stop the copy drifting from the folders. With no copy there is nothing to drift.
2. **The index's main reader is gone.** In rex-skills, `rex-code-lead` read the index to choose skills for each worker. SuperRex does not ship `rex-code-lead` as the process (build request §5.14), and builder stage 1 chooses skills with `rex skills list` (§5.3).
3. **It covers more.** The index listed only the bundle. `rex skills list` also lists the user's own skills, in the repo and in the user's Claude Code skills folder, which stage 1 must see.
4. **Least code.** One test, one marker file and one bundle file go away.

What changes in other lanes:

| Lane | Change |
| --- | --- |
| 8 | Do not carry `INDEX.md` over. Where a rulebook tells the reader to look in `INDEX.md`, it says `rex skills list` instead. |
| 4 | The installer tests stop using `skills/INDEX.md` as their marker file. Use `skills/rex-skill-language/SKILL.md`, which is in the repo today. The bundle test keeps its purpose (no stray file such as a local settings file is embedded) and stops asserting that `skills/INDEX.md` exists. |
| 6 | The rex-skills test `the_index_lists_every_skill_and_only_skills_that_exist` is retired with the file. Its other job, that every skill path named in the bundle exists, moves to rule B4 (section 4). |

### OP1 — proposed: the "use when" text goes in its own header key, `when_to_use`

`when_to_use` is a key Claude Code itself reads. It is not a key SuperRex invents.

Claude Code's skills page (code.claude.com/docs/en/skills, read 10 November 2026) says: "`when_to_use`: Additional context for when Claude should invoke the skill, such as trigger phrases or example requests. Appended to `description` in the skill listing and counts toward the 1,536-character cap."

Reasons:

1. **Claude Code reads the same words.** The plan's concern was that moving the text out of `description` changes what Claude Code reads when it decides to load a skill. It does not: Claude Code joins the two keys into one listing entry. The live check below confirms this.
2. **Exact for both commands.** `rex skills list` prints the key as it is. With the sentence inside `description`, `rex` would have to find it in free text, and the rulebooks write it four different ways today ("Use when", "Use this skill when", "Use whenever", "Triggers include").
3. **A check can test it.** `rex skills check` can say the key is missing. It cannot say a sentence inside `description` is missing.

Rejected:

- The sentence inside `description`, as rex-skills has it: needs a parser for free text, and cannot be checked.
- A key of SuperRex's own, or a value under `metadata`: Claude Code does not show it to the model, so the text would play no part in loading the skill.

Cost: Lane 8 moves the "Use …" sentences of each rulebook from `description` into `when_to_use` (plan item 8.5).

Limit: a skill used outside Claude Code (an upload to claude.ai, the Skills API) may have only `name`, `description`, `license`, `compatibility`, `metadata` and `allowed-tools`, and other keys give a hard error there. SuperRex skills are for Claude Code, so this does not apply to the bundle.

**Live check, 10 November 2026.** Claude Code 2.1.322 on macOS, in a new throwaway Git repo beside `super-rex` (removed afterwards). Each experiment was a fresh headless session: `claude -p --dangerously-skip-permissions --model sonnet --effort low --output-format stream-json --verbose "<prompt>"`. Three skills were set up. In `lane6-probe-a` the trigger word was only in a folded `when_to_use: >` value. In `lane6-probe-c` it was only in a one-line quoted `when_to_use` value. In `lane6-probe-b`, the control, it was in `description` and there was no `when_to_use`.

| # | What | Result | Evidence |
| --- | --- | --- | --- |
| E1 | The model quotes its skill listing | Confirmed | `lane6-probe-a: Prints a fixed test token for a tool check. - Use when the user says the word "quokkafrost". Use it also when the user asks for "the lane six probe".` The folded value arrives as one line, after the description and the separator ` - `. |
| E2 | Trigger word from folded `when_to_use` only | Confirmed | `"name":"Skill","input":{"skill":"lane6-probe-a"}`, result `LANE6-PROBE-A-LOADED` |
| E3 | Trigger word from one-line `when_to_use` only | Confirmed | `"input":{"skill":"lane6-probe-c"}`, result `LANE6-PROBE-C-LOADED` |
| E4 | Control, trigger word in `description` | Confirmed | `"input":{"skill":"lane6-probe-b"}`, result `LANE6-PROBE-B-LOADED` |
| E5 | Warnings or errors | None | Every standard error log was empty. Every init event listed all three skills. |

Verdict: Claude Code accepts `when_to_use`, shows it to the model, and loads a skill on it alone.

---

## 2. The skill header

### 2.1 The form

```yaml
---
name: rex-code-tdd
description: >
  Test-driven development with a red-green-refactor loop. The working method
  for writing new code and fixing bugs.
when_to_use: >
  Use when you build a feature or fix a bug test first. Use it also when the
  user says "TDD", "red-green-refactor" or "test first".
---
```

- Line 1 of `SKILL.md` is exactly `---`. The next line that is exactly `---` closes the header. Lines end in LF. A CR before the LF is accepted.
- `description` says what the skill is for. It holds no "use when" text.
- `when_to_use` says when to load the skill: the situations, then the words a user says. It starts with `Use when`.
- Write long values as folded scalars (`>`). A plain one-line value must not hold ` #` (rule H6).
- Every skill written or changed in this project is written in `rex-skill-language`. No command checks the language.

### 2.2 Keys

| Key | Required | Value |
| --- | --- | --- |
| `name` | Yes | The folder name. Lowercase letters and digits in groups joined by single hyphens, at most 64 characters. |
| `description` | Yes | What the skill is for. Not empty. |
| `when_to_use` | Yes | When to load the skill. Starts with `Use when`. |
| Any other key | No | Only a key from the list in rule H8, such as `disable-model-invocation` or `user-invocable`. |

### 2.3 How `rex` reads a text value

The text of `description` or `when_to_use` is the string YAML gives, with each run of white space (spaces, tabs, line ends) made one space, and with no space at either end. Both commands use this text. It is always one line.

---

## 3. `rex skills list`

```
rex skills list [--json]
```

Prints each skill in the two places Claude Code loads skills from at the start of a session in this repo, so that an agent can choose skills without opening their files.

### 3.1 Where it looks

1. **`repo`**: `<repo>/.claude/skills/`. `<repo>` is the nearest folder, from the current folder up, that holds a `.git` folder or a `.git` file (a worktree has a file). IF there is none, THEN `<repo>` is the current folder.
2. **`user`**: `$HOME/.claude/skills/`. IF `HOME` is not set, THEN this place is skipped.

- A skill is each entry in one of these folders that is a folder, or a symbolic link to a folder, and holds a `SKILL.md`. Other entries are skipped.
- IF the two places are the same folder, THEN its skills are listed once, as `user`.
- A place that does not exist is skipped.
- Nested `.claude/skills/` folders below the repo, plugin skills and organisation-managed skills are not listed.

### 3.2 Output

One block of six `key=value` lines for each skill, with one empty line between blocks. Keys always come in this order:

```
$ rex skills list
name=rex-code-tdd
description=Test-driven development with a red-green-refactor loop. The working method for writing new code and fixing bugs.
when_to_use=Use when you build a feature or fix a bug test first. Use it also when the user says "TDD", "red-green-refactor" or "test first".
source=repo
shadowed=no
path=/home/user/project/.claude/skills/rex-code-tdd/SKILL.md

name=rust-review-notes
description=How this team writes review notes.
when_to_use=
source=user
shadowed=no
path=/home/user/.claude/skills/rust-review-notes/SKILL.md
```

| Key | Value |
| --- | --- |
| `name` | The header's `name` when it is a string that is not empty. Else the folder name. |
| `description` | The text of `description` (section 2.3). Empty when it is missing or is not a string. |
| `when_to_use` | The text of `when_to_use`. Empty when it is missing or is not a string. |
| `source` | `repo` or `user`. |
| `shadowed` | `yes` for a `repo` skill whose `name` is also the `name` of a `user` skill. Claude Code runs the user's skill in that case. Else `no`. |
| `path` | The absolute path of the `SKILL.md`. |

- Order: all `repo` skills by `name`, then all `user` skills by `name`. Names are compared as bytes.
- A skill whose header is missing or is not valid YAML is still listed, with an empty `description` and `when_to_use`. `rex skills list` never fails because of one skill. `rex skills check` finds the problem.
- No skills: no output, exit code 0.

`--json` prints the same as one JSON array on one line. `shadowed` is a boolean. Every other value is a string.

```json
[{"name":"rex-code-tdd","description":"Test-driven development with a red-green-refactor loop. The working method for writing new code and fixing bugs.","when_to_use":"Use when you build a feature or fix a bug test first. Use it also when the user says \"TDD\", \"red-green-refactor\" or \"test first\".","source":"repo","shadowed":false,"path":"/home/user/project/.claude/skills/rex-code-tdd/SKILL.md"}]
```

### 3.3 Exit codes

| Code | When |
| --- | --- |
| 0 | The list was printed. |
| 1 | A place exists but cannot be read. The cause goes to standard error. |
| 2 | Usage error, such as an unknown flag. |

---

## 4. `rex skills check`

```
rex skills check <path>...
```

Checks one or more skills against the rules below. Builder stage 1 runs it on each skill it writes. Lanes 8, 9 and 10 run it on theirs.

### 4.1 Input

- Each `<path>` is a `SKILL.md` file or a skill folder. For a folder, `rex` checks `<folder>/SKILL.md`.
- The skill folder is the folder that holds that `SKILL.md`. The skills folder is the folder above it.
- The rules for the header (H) apply to `SKILL.md`. Rules B1 and B2 apply to `SKILL.md`. Rules B3 to B5 apply to every skill file of the skill: `SKILL.md` and every other `.md` file in the skill folder and its sub-folders.
- **Known skills**: the folders in the same skills folder that hold a `SKILL.md`, together with the skills in `rex`'s own bundle.

### 4.2 The rules

Rule IDs are permanent. One is never used again for a different meaning.

| Rule | Passes when | Why |
| --- | --- | --- |
| H1 | Line 1 of `SKILL.md` is exactly `---`, and a later line that is exactly `---` closes the header. | Claude Code reads a header only when `---` is the first line. |
| H2 | The header is valid YAML, and it is a mapping. | Claude Code loads a skill with a broken header with no fields set, and says nothing. |
| H3 | `name` is a string equal to the folder name, made of lowercase letters and digits in groups joined by single hyphens, at most 64 characters. | A name that is not the folder name is hard to find, and the bundle tests and `rex init` work by folder. |
| H4 | `description` is a string, not empty, and holds none of `Use when`, `Use this skill when`, `Use it when`, `Use it also when`. | Claude Code chooses skills by this text. The "use when" text belongs in `when_to_use`, not twice. |
| H5 | `when_to_use` is a string, not empty, and its text starts with `Use when`. | Builder stage 1 chooses skills by this line (§5.3, §5.14). |
| H6 | When `description` or `when_to_use` is a plain value on its key's own line (it does not start with `>`, `\|`, `'` or `"`), YAML gives back exactly the written text. | In a plain YAML value, ` #` starts a comment. `Inline #[cfg(test)]` would cut the text short, and the skill stops loading when a person asks for it. |
| H7 | The text of `description`, plus 3, plus the text of `when_to_use` is at most 1,536 characters. The 3 is the ` - ` that Claude Code puts between them. | Claude Code cuts the listing entry at 1,536 characters. |
| H8 | Every key in the header is one of: `name`, `description`, `when_to_use`, `argument-hint`, `arguments`, `disable-model-invocation`, `user-invocable`, `allowed-tools`, `disallowed-tools`, `model`, `effort`, `context`, `agent`, `background`, `hooks`, `paths`, `shell`, `metadata`, `license`, `compatibility`. | Claude Code ignores an unknown key and says nothing, so a misspelt key such as `when-to-use` is lost with no sign. The list is the one on Claude Code's skills page, read 10 November 2026. |
| B1 | `SKILL.md` has text after the header. | An empty skill gives the agent nothing. |
| B2 | `SKILL.md` has at most 500 lines. | Claude Code's advice: a loaded skill stays in context, so long material goes into reference files. It also matches the crate's 500-line rule. |
| B3 | Each Markdown link `](<target>)` whose target, without its `#` part, ends in `.md` and holds no `://`, points to a file that exists inside the skills folder. The target is read relative to the folder of the file that holds the link. | A link that goes nowhere, or out of the skills folder, breaks once the skill is installed in another repo. |
| B4 | Each text `.claude/skills/<name>/SKILL.md` names a known skill. `<name>` is the lowercase letters and hyphens after `.claude/skills/`. | An agent told to read a skill that is not there goes on without it. |
| B5 | Each `rex-…` word is a known skill, a name pattern, or one of the other names (section 4.3). | It finds a skill named but not shipped, such as a utility left out, or a misspelt name. |

### 4.3 The `rex-…` word rule (B5)

The rex-skills bundle test failed on any `rex-…` word that was not a shipped skill. Skills from Lanes 8, 9 and 10 must name things that are not skills: the predecessor `rex-skills`, and the Rex 2.0 skills that SuperRex does not ship. The rule stays, with two changes.

**A word.** `rex-` and the lowercase letters after it, where a hyphen is part of the word only when a lowercase letter follows it. The character before `rex-` is not a letter, a digit, `-` or `_`. So `the rex-code-tdd skill` gives `rex-code-tdd`, and `.rex/` and `Rex-Job:` give no word.

**It passes when one of these holds:**

1. It is a known skill.
2. It is a name pattern: the word is followed at once by `-*`, as in `rex-code-*`, and at least one known skill starts with the word and a hyphen.
3. It is one of the other names. Version 1 has three:

| Word | What it names |
| --- | --- |
| `rex-skills` | The Rex 2.0 tool and its repo, which SuperRex supersedes. |
| `rex-code-lead` | A Rex 2.0 skill. Not shipped (§5.14). |
| `rex-code-orchestrator` | A Rex 2.0 skill. Not shipped (§5.14). |

A word in this list may appear in text. A path to it, such as `.claude/skills/rex-code-lead/SKILL.md`, still fails rule B4, because the skill is not there. A new word in the list is a change to this contract.

### 4.4 Output and exit codes

- For each skill with no finding: one line `ok <path>`.
- For each finding: one line `<path>: <rule>: <what is wrong>`. `<path>` is the file the finding is in: the skill folder as the user wrote it, then the file's path inside that folder.
- Findings for one skill come in rule order (H1 to H8, then B1 to B5), then in the order they appear in the file.
- IF rule H1 or H2 fails, THEN rules H3 to H8 are not run for that skill.
- All output goes to standard output, except usage errors.

```
$ rex skills check .claude/skills/rex-code-tdd .claude/skills/rex-deliver/SKILL.md
ok .claude/skills/rex-code-tdd
.claude/skills/rex-deliver/SKILL.md: H5: when_to_use is missing
.claude/skills/rex-deliver/SKILL.md: B5: `rex-pr-lessons` is not a known skill or another known name
```

| Code | When |
| --- | --- |
| 0 | Every skill passed. |
| 1 | At least one finding. |
| 2 | Usage error: no path, a path that does not exist or cannot be read, or a file that is not named `SKILL.md`. The cause goes to standard error. |

The exact words of each finding are not part of this contract. The rule ID is.

---

## 5. The bundle tests

These run in `cargo test`, over the skills embedded in the binary, with the same checker as `rex skills check`. Known skills here are the bundle's skills. Each rex-skills test is kept or retired as follows.

| rex-skills test | In SuperRex |
| --- | --- |
| `every_skill_has_a_name_that_is_its_folder_and_a_description` (must never be deleted) | Kept. It now runs every rule of section 4.2 on every bundled skill, and still fails on an empty bundle. |
| `a_plain_description_is_not_shortened_by_yaml` (must never be deleted) | Kept, as rule H6. It now covers `when_to_use` too. |
| `the_index_lists_every_skill_and_only_skills_that_exist` | Retired with `INDEX.md` (OP7). Its path check becomes rule B4. |
| `every_skill_name_used_in_the_bundle_is_a_skill_that_exists` | Kept, as rule B5, with the changes of section 4.3. |
| `every_markdown_link_to_a_file_in_the_bundle_resolves` | Kept, as rule B3. |

One more test, after Lanes 2 and 9 have merged: **the builder lists exactly the classification values that `rex job create` accepts.** For Lane 9, the form is three lines, each once, in any skill file of `orchestration-builder`:

```
Work type values: `feature`, `fix`, ...
Size values: `small`, ...
Consequence values: `low`, ...
```

- Each line starts with its label. A list mark (`- `) before the label is allowed.
- The test takes the words in backticks on each line and compares them, as a set, with the values that Lane 2's argument parser accepts for `--type`, `--size` and `--consequence`. A value missing from either side fails the test.
- The values above are examples. The real ones are Lane 2's.

---

## 6. What each lane does with this contract

| Lane | What |
| --- | --- |
| 4 | Embed the skills folder without `INDEX.md`. Change the installer's marker file and the bundle test as in OP7. Nothing that `rex init` writes, such as the `AGENTS.md` sections, mentions `INDEX.md`. |
| 8 | Do not carry `INDEX.md`. In each rulebook header, move the "Use …" sentences from `description` into `when_to_use`, written to start with `Use when` (8.5). Remove ties to `INDEX.md`. Every rulebook passes `rex skills check`. |
| 9 | Write the four builder skills in the form of section 2. Stage 1 lists skills with `rex skills list` and checks each new skill with `rex skills check`. The skill that stage 1 writes uses the same header. Add the three value lines of section 5 to `orchestration-builder`. |
| 10 | Write the three delivery skills in the form of section 2. They pass `rex skills check`. |

Until Lane 6 has built the commands, Lanes 8, 9 and 10 follow this file by hand.

---

## 7. Not covered

- The language of a skill (`rex-skill-language`) is not checked by a command.
- Agent definitions in `.claude/agents/` are not checked by `rex skills check`.
- The list of keys in rule H8 is the one Claude Code documents today. A key added later by Claude Code needs a change to this contract and a `rex` release.
- The ` - ` between `description` and `when_to_use` in Claude Code's listing was seen in the live check. Claude Code's docs do not state it.
