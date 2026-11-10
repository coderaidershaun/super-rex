---
name: rex-code-failure-policy
description: >
  Hard rules for what a system does after an error: each error variant gets one
  failure class (HALT, RESET, RETRY, REJECT), the class lives in code on the
  error type, and one place at the top of the system acts on it. Silent wrong
  state always halts. Retries are bounded and safe to repeat. Sister to
  rex-code-error-writing, which says how errors are written. This skill says
  what happens next.
when_to_use: >
  Use when code has outside inputs, long-running state, retries, reconnects, or
  a main loop. Use it also when the user says "what happens when this fails",
  "should this stop the system", "retry or fail", "fatal error", "kill switch",
  "halt", or "make error handling consistent".
disable-model-invocation: false
user-invocable: true
---

# Failure Policy — Hard Rules

`rex-code-error-writing` = how error is written. This skill = what system does next.

Same error, same reaction, everywhere in repo. Reaction is decided one time, in code, not at each call site.

Sister skills:

- `rex-code-error-writing` — typed errors. Read first.
- `rex-code-priorities` — Never lines. Each one = HALT.
- `rex-code-tests-fitness-functions` — guard for each HALT that protects state.
- `rex-code-validation-plan` — expected trace for each failure scenario.

## Words

| Term | Meaning |
| --- | --- |
| Failure class | One of HALT, RESET, RETRY, REJECT. |
| HALT | Stop all new work. Keep state for a person to read. |
| RESET | Throw away state of one part. Build it again from trusted source. |
| RETRY | Do same action again, after delay, limited times. |
| REJECT | Refuse this one input. Tell caller. Continue. |
| Bug | Invariant that code itself broke. Panic. Not failure class. |
| Supervisor | One place at top of system that reads failure class and acts. |
| Trusted source | Data that is known correct: fresh snapshot, database, file on disk. |
| Safe to repeat | Second run of action gives same result as one run. |

## Pick class

First row that applies wins.

| Fact about error | Class |
| --- | --- |
| It breaks Never line of priority block | HALT |
| State can now be wrong and nothing downstream can tell | HALT |
| State of one part is wrong, trusted source can rebuild it | RESET |
| Outside world failed for short time: timeout, 5xx, rate limit, dropped link | RETRY |
| This one input is bad: parse error, failed validation, unknown id | REJECT |
| Code broke its own invariant | panic (bug) |

- [ ] Not sure between two classes? Pick class higher in table.
- [ ] RETRY limit reached → error becomes class of next row up that applies. Usually RESET or HALT.
- [ ] RESET fails 2 times in a row → HALT.

## Class lives on error type

Policy in code. Not in doc. Compiler makes each new variant pick class.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureClass { Halt, Reset, Retry, Reject }

impl SyncError {
    pub fn failure_class(&self) -> FailureClass {
        match self {
            SyncError::ChecksumMismatch { .. } => FailureClass::Halt,
            SyncError::SequenceGap { .. } => FailureClass::Reset,
            SyncError::Timeout { .. } => FailureClass::Retry,
            SyncError::Malformed { .. } => FailureClass::Reject,
        }
    }
}
```

- [ ] One `FailureClass` enum for whole crate. It lives in shared data module.
- [ ] Each error enum that reaches supervisor has `failure_class`.
- [ ] `match` has no `_ =>` arm. New variant must not get class by accident.
- [ ] Error that wraps other error gives class of inner error, unless outer has reason to raise it. Never lower it.
- [ ] `anyhow` in `main` only prints. Supervisor acts on typed error before that.

## One supervisor

- [ ] One place reads `failure_class` and acts: main loop, task root, or request entry.
- [ ] Lower code returns error with `?`. It does not retry, reset, exit, or log-and-continue on its own.
- [ ] Exception: retry of one outside call can live in wrapper of that call. Still obeys RETRY rules.
- [ ] No `std::process::exit` and no panic for HALT in lower code.
- [ ] No `let _ =` on `Result`. No error that only gets logged.

## HALT

- [ ] Stop new work first. Then do safe stop of open work that plan defines.
- [ ] Keep state. Do not clean, do not restart, do not retry.
- [ ] Write one loud line: what failed, values seen, what was stopped.
- [ ] Only a person starts system again. No auto-restart after HALT.
- [ ] Plan defines "safe stop" for this repo. Example: cancel open orders, finish write in progress, close files.

CAUTION: Do not turn a HALT into a RETRY or a log line to make a test pass. A HALT guards state that can be wrong with no other sign.

## RESET

- [ ] Reset smallest part that holds bad state. Not whole system.
- [ ] Part gives no output until rebuild is complete.
- [ ] Rebuild from trusted source only. Never from own old state.
- [ ] Count resets. Limit in config.

## RETRY

- [ ] Max tries in config. No endless loop. No bare `loop` around call.
- [ ] Delay grows each try. Add random part. Obey `Retry-After` when server gives it.
- [ ] Only retry action that is safe to repeat. IF not safe, THEN add unique key so receiver drops duplicate. ELSE no retry.
- [ ] Each try has time limit.
- [ ] After failure with unknown result (timeout on send): find out what happened before you send again.

CAUTION: Do not retry an action that moves funds or sends a message to a person unless it is safe to repeat. A blind retry can do the action two times.

## REJECT

- [ ] Return typed error to caller with values seen.
- [ ] No state change from rejected input. Check before first write.
- [ ] Count rejects by source. Many rejects from one source in short time = RESET or HALT of that source. Limit in config.

## In plan

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

## Audit sweep

```bash
grep -rn 'let _ = ' src/                       # swallowed Result?
grep -rnE '_ => *FailureClass' src/            # catch-all class
grep -rn 'process::exit' src/                  # exit outside main
grep -rnE '^\s*loop \{' -A6 src/ | grep -E 'retry|send|connect|fetch'   # retry with no limit?
grep -rnE 'error!\(|warn!\(' -A2 src/ | grep -E 'continue|Ok\(\(\)\)'  # log + continue
```

Each hit = candidate. Read it before you report it.

## Checklist

- [ ] Each error variant that reaches supervisor has one failure class
- [ ] `failure_class` match has no catch-all arm
- [ ] One supervisor acts. Lower code only returns errors
- [ ] Each Never line maps to HALT
- [ ] HALT keeps state, stops new work, needs a person to restart
- [ ] RESET rebuilds from trusted source
- [ ] RETRY has max tries, growing delay, and action is safe to repeat
- [ ] REJECT changes no state
- [ ] Failure table in plan, with guard for each row
