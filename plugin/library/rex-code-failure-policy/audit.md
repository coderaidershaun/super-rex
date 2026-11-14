# Audit sweep

Rules that each hit can break are in `rex-code-failure-policy/SKILL.md`.

```bash
grep -rn 'let _ = ' src/                       # swallowed Result?
grep -rnE '_ => *FailureClass' src/            # catch-all class
grep -rn 'process::exit' src/                  # exit outside main
grep -rnE '^\s*loop \{' -A6 src/ | grep -E 'retry|send|connect|fetch'   # retry with no limit?
grep -rnE 'error!\(|warn!\(' -A2 src/ | grep -E 'continue|Ok\(\(\)\)'  # log + continue
```

Each hit = candidate. Read it before you report it.
