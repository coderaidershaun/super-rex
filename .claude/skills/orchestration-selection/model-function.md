# How `rex oms` selects

Step 4 of `.claude/skills/orchestration-selection/SKILL.md`. Read IF you must know why `rex oms` gave its output.

How function acts (function version 1). Stakes = higher of reliability and consequence.

- [ ] Tier starts at intelligence. Stakes `high` adds one step.
- [ ] Effort starts at stakes. Verification `low` adds one level, `high` removes one. Autonomy `high` adds one, `low` removes one.
- [ ] Budget caps tier + effort. Duration caps effort only. Caps never raise anything.
- [ ] Stakes `high`: result is never below `model-medium-effort-high`. `conflict` line names cap that floor overrode.
- [ ] All inputs `none` gives `model-medium-effort-medium`.

IF `version` line of output is not `1`, THEN trust output, not these lines.

Example:

```
$ rex oms --budget low --intelligence high --autonomy high \
          --reliability high --consequence high --verification low
agent=model-medium-effort-high
demand=model-xhigh-effort-max
rules=T1,T2,E1,E2,E4,C1,C2,F1
conflict=budget
inputs=-LHHHHL
version=1
```
