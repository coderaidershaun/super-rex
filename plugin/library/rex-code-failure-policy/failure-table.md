# Failure table in plan

Words (failure class, HALT, RESET, RETRY, REJECT, supervisor, trusted source) and class rules are in `rex-code-failure-policy/SKILL.md`.

Lead adds failure table to `plan.md` for each feature with outside input or long-lived state.

| Error variant | Class | Supervisor action | Guard |
| --- | --- | --- | --- |
| `SyncError::ChecksumMismatch` | HALT | stop intake, keep state, alert | fitness: rebuilt state = source |
| `SyncError::SequenceGap` | RESET | drop cache, fetch fresh snapshot | contract test |
| `SyncError::Timeout` | RETRY | max 5, delay 200 ms to 10 s | integration test |
| `SyncError::Malformed` | REJECT | drop message, count by source | unit, then delete |

- [ ] Each HALT row has fitness function or reason why not.
- [ ] Each row with RESET, RETRY, or HALT has scenario in `rex-code-validation-plan`.
- [ ] Advisor reads table before code. Missing variant = finding.
