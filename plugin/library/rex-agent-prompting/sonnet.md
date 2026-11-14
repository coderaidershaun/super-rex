# Tier `medium` — Sonnet 5.5

Sibling file of `rex-agent-prompting/SKILL.md`. Read when you start agent `super-rex:model-medium-effort-<level>`.

Fits: work with clear scope. Bug fix, feature from specification, document, repeated agent task. Not hardest long work.

## By level

| Level | Agent tends to | Add to brief |
| --- | --- | --- |
| `low` | Ask before it finishes. Skip check. | Finish text + check text. |
| `medium` | Ask before it finishes. | Finish text. |
| `high` | Add tests, documents, clean-up that nobody asked for. | Stop text. |
| `xhigh`, `max` | Same, + extra review rounds and reviewer agents. | Stop text + review text. |

Task that asks only for ideas, options, or plan: add ideas text, at each level.

## Brief text

Finish text:

```
Keep working until everything in this brief is done and checked. Do not stop to ask whether to continue. Stop early only for a decision or an approval that this brief does not give.
```

Stop text:

```
When the work in this brief is done and checked, stop and report. Add nothing that the brief does not ask for: no extra tests, documents, or clean-up. If you see other work that is worth doing, name it in your report.
```

Check text:

```
Before you report, run a real check of your result: run the test or the command, or read the file that you wrote. Report what you ran and what it showed. If you could not check something, say so.
```

Review text:

```
Check your work one time. Do not start extra rounds of review, and do not start reviewer sub-agents, unless this brief asks for them.
```

Ideas text:

```
This task asks for <ideas | options | a plan> only. Give them and stop. Do not start to build.
```

## Rules

- [ ] Give scope exactly: files that agent owns, and what it leaves alone.
- [ ] IF agent must use facts that can have changed, THEN tell it to search for them.

## Do not

- [ ] Old workarounds: "do not be lazy", "do not refuse", "retry the tool call".
- [ ] "Do not think". Agent then writes internal tags into its reply.
- [ ] Many progress reminders. After third, agent can read them as attack.
