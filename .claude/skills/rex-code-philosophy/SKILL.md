---
name: rex-code-philosophy
description: Hard programming rules distilled from grugbrain.dev + Richard Gabriel's Worse-is-Better essays (Rise of Worse is Better, Worse is Better is Worse, Is Worse Really Better?). Complexity-averse, ergonomics-first, habitability-aware code philosophy. Use when writing, reviewing, refactoring, or designing code, or when user asks "how should I write this", "what's right approach", "is this over-engineered", "should I abstract this", "ship simple or do it right", "MVP vs proper".
disable-model-invocation: false
user-invocable: true
---

# Code Philosophy — Grug + Gabriel

Follow these rules. No compromises.

Complexity = enemy. Ergonomics = friend. Habitability = home. Read before code.

Sources: grugbrain.dev (grug), Gabriel's *Rise of Worse is Better*, Bourbaki's *Worse is Better is Worse* (Gabriel anti-self), Gabriel's *Is Worse Really Better?*. Tension between camps = where wisdom live.

## Prime directive

- Complexity bad. Complexity *very* bad.
- "No" = magic word. Say "no" to self often. Come up with better, leaner solution.
- "No" = magic word again. If user ask something dumb. Say no and ask for reason why such dumb request.
- Working ugly code beat broken elegant code. Always.
- Try be too clever. You be fired.
- **Aim high. Ship lean.** Bourbaki: never intentionally aim less than best. Grug+Gabriel: ship 80% simple version first. Both true. Aim → compromise → ship → iterate. Not aim → ship trash.

## Four dimensions (Gabriel)

Every design ranked on 4 axes:

1. **Simplicity** — impl + interface easy to understand
2. **Correctness** — does right thing in all cases
3. **Consistency** — uniform behavior, no special case
4. **Completeness** — covers all cases that should be covered

- **MIT/Stanford style ("right thing"):** all 4 equal weight. Result: pie-in-sky. Rare ship. Often never finish.
- **NJ/Unix style ("worse is better"):** impl simplicity above all. Sacrifice other 3 if must. Result: ship fast, spread, iterate.
- **Grug verdict:** rank simplicity first, correctness second. Consistency + completeness emerge from iteration.
- **Bourbaki warning:** real dichotomy = right-thing-design vs **no design**. Worse-is-better still design. "Quick and dirty" w/o thought = no design = doom.
- **Survival ≠ quality.** Successful tool not always good tool. C++ won. Doesn't mean C++ right. Don't confuse adoption with correctness.
- **Free market favors incremental.** Radical re-design rarely win. Spread small wins.

## Ergonomics + habitability (read this twice)

Code read 100x more than written. Optimize reader, not writer.
**Habitability** (Gabriel) = programmer must *live* in this code. Comfortable. Familiar. Modifiable. Codebase = home, not museum.

- **Locality > separation.** Put behavior on thing that does behavior. No hunt across 5 files.
- **Named intermediates > clever one-liner.** Debugger see each step → fix faster.
  ```
  bad:  return users.filter(u => u.x && u.y).map(u => f(u))[0]
  good: active = users.filter(u => u.x && u.y)
        eligible = active.filter(u => u.y)
        result = f(eligible[0])
  ```
- **Common op on the thing itself.** `user.activate()` > `UserActivationService.activate(user)`.
- **Simple API for simple case.** Layer for hard case. Not reverse.
- **Type system serves fingers.** Main value = IDE autocomplete + suggest. Not proof.
- **Big-brain types = astral projection.** Avoid generic spirals. Container generics only.
- **Closure = salt.** Pinch good. Handful = callback hell.
- **Boring name beat clever name.** `userCount` > `aggregatedEntityCardinality`.
- **Co-locate config near use.** Far config = stale config.
- **Errors carry context.** Stack trace + what was tried + what value seen. No "something failed".
- **Habitable code (Gabriel).** Right balance abstract/concrete. Simple impl model. Simple perf model. Reader can predict what code do without run.
- **Low-talent ceiling.** Code base must be improvable by mid programmer at 2am. Genius-only code = death.
- **Predictable perf > clever perf.** Simple perf model = caller reason about cost. Hidden magic = bug factory.
- **Pit of success.** Easy thing = right thing. Wrong thing = effort. Default path good.

## Complexity rules

- **80/20 cut.** 80% want, 20% code. Ship that. Skip last 20% want.
- **Don't factor early.** Wait for shape. Cut points emerge → narrow interface trap complexity inside.
- **DRY balanced.** 3 similar lines OK. Premature abstraction = pain. Repetition < wrong abstraction.
- **Microservices?** Solve hardest problem (factoring) then add network call on top? No.
- **Visitor pattern: bad.**
- **No silver bullet.** Anyone selling one = shaman. Skeptic.

## Testing

- **Integration test = sweet spot.** High enough = correctness. Low enough = see what break.
- **Few e2e.** Curated. Hard debug.
- **Unit test = rare.** Refactor break them. Limited long-term value.
- **No mock unless forced.** Coarse only. Mocked pass + prod fail = worst.
- **Bug found → regression test first → fix.** Always.
- **Test after prototype.** Domain first. Test second.

## Refactoring + piecemeal growth

- **Small step.** System work between each. Stay near shore.
- **Big refactor + new abstraction = fail.** Almost always.
- **Chesterton fence.** Don't tear out code without knowing why exist. Think → then destroy.
- **Piecemeal growth (Alexander/Gabriel).** System grow by small additions matching local need. Not master plan. Not grand re-architecture. Each step leave system habitable.
- **Spread first, perfect later.** Ship 50%-good thing → users hook → iterate to 90%. Sit on 100% in lab = die in lab.
- **But:** spread of trash = lock-in of trash. C++ warning. Ship lean ≠ ship sloppy. Lean still designed.

## Debugging + logging

- **Debugger > printf prayer.** Two weeks learn tools → 2x speed forever. Worth it.
- **Log every branch.** if/for entry. Cloud especially.
- **Request ID on every log.** Trace across machines.
- **Dynamic log level per user.** Production debug without redeploy.

## Optimization

- **Profile first.** Always. No exception.
- **Network latency usually culprit.** Not CPU. Not O(n²).
- **Premature optimization = root of evil.** Knuth correct.

## Concurrency

- **Fear it.** Sane developer fear concurrency.
- **Stateless handler.** Simple job queue. Optimistic concurrency for web.
- **Thread-local = framework only.** Not app code.

## APIs + parsing

- **Recursive descent parser.** Beautiful, simple, underrated. Skip parser generator.
- **API design from use site.** Not implementation. What caller want type first.

## Mindset

- **FOLD (fear of looking dumb) feed complexity demon.** Senior say "too complex for me" loud → junior breathe.
- **Working demo > big-brain design doc.** Reality check beat whiteboard.
- **Impostor syndrome universal.** Everybody impostor → nobody impostor.
- **Hype recycled.** Old idea new name. Old hand tried already.
- **Successful != good.** "Future in hands of worst of our fruits" (Gabriel). Stay critic of own success.
- **Both sides true.** Worse-is-better wins market. Right-thing wins admiration. Pick fight knowing which prize.
- **Push complexity to right place.** Sometimes user. Sometimes lib. Sometimes runtime. Hide complexity from wrong place = bug. Expose complexity to right place = design.

## When in doubt

1. Simpler version exist? Use it.
2. Can delete instead of add? Delete.
3. Will reader 6 months later curse you? Rewrite.
4. Asked for fix → only fix. No surrounding cleanup.
5. Unsure → say unsure. Ask.
6. **Right thing or worse-is-better?** Ask: ship deadline tight + iterate possible → worse-is-better. One-shot + correctness load-bearing → right thing. Default: lean.
7. **Habitable for next maintainer?** Imagine 2am page. They read this code. Curse or grateful?
8. **No design = always wrong.** Lean = designed lean. Quick = thought quick. Sloppy = no thought = no.
