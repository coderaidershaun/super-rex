# Tier `low` — Haiku 5.5

Sibling file of `rex-agent-prompting/SKILL.md`. Read when you start agent `super-rex:model-low-effort-<level>`.

Fits: classification, extraction, search in files, log reading, summary, short tool task. Not complex code work.

## By level

| Level | Add to brief |
| --- | --- |
| `low` | Check text. Finish text IF task has more than a few steps. |
| `medium` | Check text. Finish text IF task is long. |
| `high` | Nothing. Use this level when agent must obey each rule of brief exactly. |
| `xhigh`, `max` | Nothing. Tell your parent: tier `medium` at lower level does this work better for same cost. |

Each level: IF agent can search web, THEN add date text.

## Brief text

Finish text:

```
Work until every item of the task is done. A partial result with a list of what is left is not a finished task. Stop before the end only if you need a decision or an approval that this brief does not give.
```

Check text:

```
Before you report, run a real check of your result: run the test or the command, or read the file that you wrote. Report what you ran and what it showed. If you could not check something, say so.
```

Date text:

```
Today's date is <date>. Your training data ends well before this date, so search for facts that can have changed.
```

## Rules

- [ ] Small brief, exact words. Agent does what brief says, not what you meant.
- [ ] One job for each agent. Give list of steps IF order is important.
- [ ] Give form of answer: fields, or one example in `<example>` tags.
- [ ] Give paths of files to read. Do not paste long text: price of each token rises five times above 100,000 tokens of prompt.

## Do not

- [ ] "Always search". Agent then searches when it has answer already.
- [ ] Same brief again after refusal. See `## Refusal` in `SKILL.md`.
- [ ] Design decisions, or judgment where mistake is hard to see. Give those to higher tier.
