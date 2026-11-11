---
name: rex-code-error-writing
description: >
  Hard rules for Rust error handling: thiserror in libs (default), anyhow in
  bins. Errors are typed and carry context. No Box<dyn Error>, no .unwrap() in
  prod.
when_to_use: >
  Use when you write error types, design Result<T,E> APIs, wrap errors, or
  write error messages, or the user asks "what error type", "how should I
  handle this error", "thiserror or anyhow", "error variants".
---

# Rust Error Writing — Hard Rules

Other language: apply intent of each rule in idiom of that language. IF language lacks Rust feature that a rule relies on, THEN obey gap skill of that language: skill for what Rust gives by default and that language does not.

These rules HARD. No compromise. Every Rust error in this repo obey them.

Sister skills: `rex-code-philosophy` (errors carry context = habitability), `rex-code-ergonomics` (Result patterns), `rex-code-commenting` (doc errors on pub fn), `rex-code-failure-policy` (what system does after error).

IF you write error enum, `main`, error message, or error mapping, THEN read `.rex/skills/rex-code-error-writing/examples.md`. It has code for each section, under same heading.

## Iron rule

**Library crate / shared module → `thiserror`. Binary entrypoint → `anyhow`. Default thiserror.**

Why split:
- **thiserror** = typed enum errors. Caller pattern-match. Public API contract. Zero runtime cost.
- **anyhow** = boxed dynamic error w/ context. Caller does not care. Propagate + print.
- **Lib give caller choice.** Bin make end-user choice (only exit).

Wrong default = `Box<dyn Error>` in lib API. Forbidden. Caller cannot match. Cannot recover. Cannot compose.

## thiserror — default

Pattern: one enum per module/concept. Example `ConfigError`: `.rex/skills/rex-code-error-writing/examples.md`.

### Rules

- **One error type per concept.** `ConfigError`, `AuthError`, `DbError`. Not one mega `AppError` shared.
- **Variant name = what failed.** `NotFound`, `Parse`, `Timeout`. Not `Error1`, `BadInput`.
- **`#[source]` for chain.** Wrap underlying error. `?`-chain build w/ `From`.
- **Carry values seen.** Path, field name, value, expected. No "something failed".
- **`#[error("...")]` = user-readable.** Not log dump. Reads like sentence.
- **No `pub` on inner fields that you do not want stable.** Use accessor fns.

### Free `?` via thiserror From

`#[from]` = derive `From<inner>`. `?` operator works. No manual `.map_err`.

**Caveat:** `#[from]` works only IF variant has *exactly one* unnamed field. IF multiple, THEN use `#[source]` + manual `.map_err(...)?`.

### Transparent wrap

`#[error(transparent)]`: `transparent` = pass through to inner. Use when you wrap w/o new info.

## anyhow — bins + scripts only

### When OK

- `main()` in binary
- One-off CLI tool
- Test helpers
- Migration scripts
- Glue code where caller will only print + exit

### Rules

- **`.context("...")` on every `?`.** Build trace. No bare `?` in bin.
- **`bail!` for early Err.** Not `return Err(anyhow!(...))`.
- **`anyhow!("msg")` for ad-hoc.** No type definition necessary.
- **Never `anyhow::Error` in lib pub API.** Caller stuck.
- **Downcast rare.** IF you often use `.downcast_ref::<SpecificError>()`, THEN switch to thiserror.

## Error messages — habitability

Per `rex-code-philosophy`: errors carry context. Reader 2am, paged, must fix.

### Hard rules

- **Concrete value seen.** "expected u32, got -1" > "invalid input".
- **What code tried.** "failed to read /etc/foo.toml" > "read failed".
- **Actionable next step (when known).** "missing field `port`. Add `port = 8080` to [server] section." > "missing field".
- **No "Error:" prefix.** Caller add it. You write the noun.
- **No trailing period.** Display chain re-format. Period collide.
- **Lowercase first letter.** Composes in chains. Per Rust API guidelines.

Bad: `#[error("Error: Failed to do thing.")]`. Good: `#[error("failed to bind tcp listener on {addr}")]`.

### Error chain reads top-down

```
failed to start server
  caused by: failed to bind tcp listener on 0.0.0.0:8080
  caused by: address already in use (os error 98)
```

Each layer add specific context. Do not repeat parent.

## panic vs Result — decision

| Situation | Tool |
|-----------|------|
| Bug — invariant violated | `panic!` / `unreachable!` / `debug_assert!` |
| Test setup | `.unwrap()` |
| Bad caller input (lib) | `Result::Err(YourError)` |
| Missing file / network / parse | `Result::Err` |
| Initialization that must succeed in `main` | `.expect("WHY this can't fail")` |
| Index `arr[i]` known in bounds | `arr[i]` (not `.get(i).unwrap()`) |

`.unwrap()` outside test = code smell. `.expect("...")` w/ WHY message OK at boundaries.

## Forbidden — fix on sight

- **`Box<dyn Error>` in lib pub API.** Use thiserror enum.
- **`unwrap()` in non-test code.** Use `?` or `expect("WHY")`.
- **`anyhow::Error` in lib pub fn return.** Bin only.
- **String-only errors (`Result<T, String>`).** No source chain. No match.
- **Stringly-typed match.** `if e.to_string().contains("not found")`. Use variant.
- **Swallow w/ `let _ = ...`.** Return it with `?`, or handle it. Log alone is not handle: supervisor acts on its failure class (`rex-code-failure-policy`). Never silent drop.
- **`println!` instead of return.** Library not own stdout.
- **Catch-all `Other(String)` variant.** Forces stringly-typed match. Define variant or wrap concrete.

## Common patterns

IF you map external error to your variant, build multi-source error with no `#[from]`, turn `Option` into `Result`, or collect Results, THEN read `.rex/skills/rex-code-error-writing/examples.md`, section "Common patterns".

## Migration: anyhow → thiserror

Common path: prototype w/ anyhow → harden into thiserror as API stabilizes.

1. Identify error sources (each `.context(...)` = candidate variant).
2. Define enum w/ `thiserror`.
3. Replace `anyhow::Error` return type w/ your enum.
4. Add `#[from]` for upstream errors.
5. Caller updates: now can match.

Do not migrate `main.rs` itself. anyhow stays there.

## Doc comments on errors

Per `rex-code-commenting`. Example: `.rex/skills/rex-code-error-writing/examples.md`, section "Doc comments on errors".

`# Errors` section = required on pub fns that return Result. Lints catch missing.

## Checklist

- [ ] Lib code uses `thiserror`, not `anyhow`, not `Box<dyn Error>`
- [ ] Bin/main uses `anyhow` w/ `.context()` on every `?`
- [ ] Each variant carries values seen + path/source
- [ ] `#[error(...)]` lowercase, no period, concrete
- [ ] `#[source]` or `#[from]` for chain
- [ ] No `.unwrap()` outside `#[cfg(test)]`
- [ ] No `Result<T, String>` anywhere
- [ ] `# Errors` doc section on pub fns
- [ ] Variant names = what failed, not generic
- [ ] No `Other(String)` catch-all

IF any fail, THEN fix before push.
