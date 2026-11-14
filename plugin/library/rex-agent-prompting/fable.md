# Tier `xhigh` — Fable 5.2

Sibling file of `rex-agent-prompting/SKILL.md`. Read when you start agent `super-rex:model-xhigh-effort-<level>`.

Fits: hardest long work. Large migration or refactor, architecture, lead of many agents, deep research. Costs most.

## By level

| Level | Add to brief |
| --- | --- |
| `low`, `medium` | Finish text. It halves runs that end with open work. Cost: about 30% more output tokens. |
| `high`, `xhigh`, `max` | Nothing. |

Each level: report text, IF brief gives no report form. Owner text only IF you want agent to merge duplicate code.

## Brief text

Finish text:

```
Do not end your turn while work that this brief asks for is still open. Work through ordinary obstacles such as errors, timeouts, and empty results: find the cause and try another way. Stop before the end only when you need a decision or an approval that this brief does not give, or when something was blocked on purpose. Ask for approval before each action that this brief says needs approval.
```

Report text:

```
Write your report for a reader who has none of your context. Put the result first, then the evidence. Use plain words and full sentences.
```

Owner text. Cost: 70 to 80% more output tokens, and changes that brief did not ask for:

```
You are the owner and long-term maintainer of this code. Where you find duplicate code in the files of your scope, merge it. List each such change in your report.
```

## Rules

- [ ] Give outcome + acceptance, not steps. Agent finds method. Steps make it worse.
- [ ] Hard or unclear problem is fine. Give it whole, with what you know.
- [ ] Name each action that needs approval. Name routine actions that need none.
- [ ] Narrow stop conditions: "stop if the migration fails", not "stop at first error".
- [ ] Review diff of agent, not only its summary. Summary can leave out small changes.

## Do not

- [ ] "Never X" for thing that is sometimes necessary. Agent obeys it always. Write "ask before X".
- [ ] Reminder to check or test its work. Agent does that itself.
- [ ] Hard style limit on prose, such as max words in each sentence. Prose gets worse. Limit on length of whole report is fine.
- [ ] "No refactor" together with owner text. Owner text wins in most runs. Leave owner text out.
- [ ] Standing instruction to delegate. Say what delegation costs, and let agent decide.
- [ ] Security testing or research biology. Agent refuses. See `## Refusal` in `SKILL.md`.
