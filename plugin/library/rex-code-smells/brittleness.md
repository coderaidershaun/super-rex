# Lens 3 — Brittleness signals

Method + severity: `rex-code-smells/SKILL.md`.

Brittle = looks fine, shatters on small perturbation. Signals:

| Signal | Detect | Why it bites |
|---|---|---|
| `.unwrap()`/`.expect` on external input | `grep -rn '\.unwrap()\|\.expect(' src/` — anything downstream of network/file/env/args | first unusual input = panic in prod |
| unchecked index after split/parse | `[0]`/`[1]` on `split`/`parse` results | input shape assumption nobody wrote down |
| duplicated knowledge | same magic literal 2+ places (`grep -rn '86400'` style), same rule re-encoded | change updates N−1 copies |
| ordering dependency | `init` flags, `assert!(ready)`, "call X before Y" in docs | types do not enforce sequence — one reorder away from breakage |
| data w/o control flow | field with no writer, method with no caller — both compile green | seam half-wired. Grep BOTH sides of every seam |
| stringly-typed seam | `&str` params matched against literals deep inside | typo compiles. Enum/newtype at boundary does not |
| float in state logic | `f64` key, `==` on floats, float money accumulator | rounding error drifts state — use integers or decimal type for money and counts |
| wall clock in state transition | `now()`/`Instant::now` decides state, not only stamps time | replay or test gives different result — give time as parameter |
| brittle tests | assert on `Debug` format, exact multi-line strings, HashMap iteration order, `sleep` | harmless refactor turns suite red → people ignore red |
