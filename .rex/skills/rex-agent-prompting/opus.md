# Tier `high` — Opus 5.5

Sibling file of `.rex/skills/rex-agent-prompting/SKILL.md`. Read when you start agent `model-high-effort-<level>`.

Fits: complex code and knowledge work, code review, work with images, default when unsure.

## By level

| Level | Add to brief |
| --- | --- |
| `low` | Stops text. Trivial task with one answer: direct text also. |
| `medium` | Stops text. |
| `high` | Stops text IF task is long. |
| `xhigh`, `max` | Nothing. |

Agent that works for many minutes with no person: stops text at each level.

## Brief text

Stops text. Keep only stops that fit task:

```
This run has no person to answer you. Do not stop to ask whether to continue, to report progress, to offer options, or because the task is large. Stop only when the acceptance evidence passes, or when you need a decision or an approval that this brief does not give.
```

Direct text:

```
Answer directly without deliberating.
```

Time line, IF agent has time limit. Send it in message to running agent also:

```
elapsed <seconds>s / <limit>s
```

## Rules

- [ ] Define done as evidence: "the tests pass", "the command prints X".
- [ ] Final message of agent with no evidence = progress report, not done. Send one message that names open items. Max 2 such messages, then report `partial`.
- [ ] Agent obeys named stops. "Do not stop early" alone does little. Name each stop.
- [ ] Work in many applications or many folders: tell agent to look at all of them before it acts.
- [ ] User interface work: name each pattern that you do not want. "No generic look" does nothing.

## Do not

- [ ] Stops text when person reads each step and answers.
- [ ] "Treat that answer as done" for long analysis. Agent must examine its earlier work again.
- [ ] Direct text at `medium` or above. It works only at `low`. Make sure quality holds.
