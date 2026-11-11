---
name: rex-code-failure-policy
description: >
  Hard rules for what a system does after an error: each variant gets one
  failure class (HALT, RESET, RETRY, REJECT).
when_to_use: >
  Use when code has outside inputs, long-running state, retries, reconnects,
  a main loop, or the user says "what happens when this fails", "should this
  stop the system", "retry or fail", "fatal error", "kill switch", "halt",
  "make error handling consistent".
---

# Failure Policy — Hard Rules

Other language: apply intent of each rule in idiom of that language. IF language lacks Rust feature that a rule relies on, THEN obey gap skill of that language: skill for what Rust gives by default and that language does not.

`rex-code-error-writing` = how error is written. This skill = what system does next.

Same error, same reaction, everywhere in repo. Reaction is decided one time, in code, not at each call site.

Sister skills:

- `rex-code-error-writing` — typed errors. Read first.
- `rex-code-priorities` — Never lines. Each one = HALT.
- `rex-code-tests-fitness-functions` — guard for each HALT that protects state.
- `rex-code-validation-plan` — expected trace for each failure scenario.

## Files of this skill

| Read | When |
| --- | --- |
| `.rex/skills/rex-code-failure-policy/failure-table.md` | IF you write or review plan for feature with outside input or long-lived state, THEN read it. Failure table in plan. |
| `.rex/skills/rex-code-failure-policy/audit.md` | IF you audit code for failure handling, THEN read it. Grep sweep. |

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
