# Classify

Step 1 of `.claude/skills/orchestration-builder/SKILL.md`. That file gives the values. This file defines each value and gives fill rules.

Work type: what request asks you to deliver. Take first row that applies, from top.

| Value | Definition |
| --- | --- |
| `feature` | Adds behavior that software did not have. |
| `fix` | Corrects behavior that is wrong against its intent. |
| `refactor` | Changes code, not what code does. Includes faster or cleaner code. |
| `chore` | Changes repo, but not behavior and not code structure: documentation, tests, dependencies, build, CI, configuration, tooling. |
| `research` | Deliverable = answer: findings, comparison, recommendation, or plan. Request asks no change to product. |
| `other` | No row above applies. |

Size: how much separate work request holds. Module = one source file, or one folder of files with one purpose.

| Value | Definition |
| --- | --- |
| `small` | One outcome. You expect it to stay inside one module. |
| `medium` | One outcome across several modules, OR 2 or 3 outcomes that depend on each other. |
| `large` | Several independent outcomes, each one you can build and test alone. OR change that reaches most of codebase. |

Size + stated budget set spend limit of job. At limit, `rex gate` refuses each new sub-agent. Person raises limit with `rex limit <job> --set <dollars>`.

Consequence: worst credible result of mistake anywhere in job. Test: does revert of commit undo harm?

| Value | Definition |
| --- | --- |
| `low` | Mistake is cheap: normal checks catch it, or revert of commit undoes it. |
| `medium` | Mistake causes meaningful rework or disruption: it can reach main unseen, break other work, or be hard to find. |
| `high` | Mistake can cause damage that revert of commit does not undo: real funds, production data, security or secrets, published release. |

Languages: programming languages of source files that job adds or changes. One or more values. Data, configuration, and markup formats (Markdown, TOML, YAML, JSON) do not count.

| Value | Definition |
| --- | --- |
| `<name>` | Lower-case name, no symbols: `rust`, `go`, `cpp`, `c`, `python`, `typescript`, `javascript`, `java`, `csharp`, `swift`, `kotlin`, `shell`, `sql`. Set is open. Spelling never changes. |
| `none` | Job writes no code. |

Fill rules:

1. Classify whole request as one job, also when it holds two kinds of work.
2. Work type: take first row that applies.
3. IF in doubt between two sizes or two consequence levels, THEN take higher.
4. Write classification one time. Never edit it.
