---
name: rex-code-error-writing
description: Hard rules for Rust error handling. thiserror in libs (default), anyhow in bins (occasionally). Typed errors with context, no Box<dyn Error>, no .unwrap() in prod. Use when writing error types, designing Result<T,E> APIs, choosing thiserror vs anyhow, wrapping errors, writing error messages, or user asks "what error type", "how should I handle this error", "thiserror or anyhow", "error variants".
disable-model-invocation: false
user-invocable: true
---

# Rust Error Writing — Hard Rules

These rules HARD. No compromise. Every Rust error in this repo follow them.

Sister skills: `rex-code-philosophy` (errors carry context = habitability), `rex-code-ergonomics` (Result patterns), `rex-code-commenting` (doc errors on pub fn).

## Iron rule

**Library crate / shared module → `thiserror`. Binary entrypoint → `anyhow`. Default thiserror.**

Why split:
- **thiserror** = typed enum errors. Caller pattern-match. Public API contract. Zero runtime cost.
- **anyhow** = boxed dynamic error w/ context. Caller doesn't care. Just bubble up + print.
- **Lib give caller choice.** Bin make end-user choice (just exit).

Wrong default = `Box<dyn Error>` in lib API. Forbidden. Caller can't match. Can't recover. Can't compose.

## thiserror — the default

### Pattern: enum per module/concept

```rust
#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("config file not found: {path}")]
    NotFound { path: PathBuf },

    #[error("config file unreadable at {path}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("config parse failed at {path} line {line}")]
    Parse {
        path: PathBuf,
        line: usize,
        #[source]
        source: serde_json::Error,
    },

    #[error("config invalid: {field} = {value:?}, expected {expected}")]
    Invalid {
        field: &'static str,
        value: String,
        expected: &'static str,
    },
}
```

### Rules

- **One error type per concept.** `ConfigError`, `AuthError`, `DbError`. Not one mega `AppError` shared.
- **Variant name = what failed.** `NotFound`, `Parse`, `Timeout`. Not `Error1`, `BadInput`.
- **`#[source]` for chain.** Underlying error wrapped. `?`-chain build w/ `From`.
- **Carry values seen.** Path, field name, value, expected. No "something failed".
- **`#[error("...")]` = user-readable.** Not log dump. Reads like sentence.
- **No `pub` on inner fields you don't want stable.** Use accessor fns.

### Free `?` via thiserror From

```rust
#[derive(thiserror::Error, Debug)]
pub enum FetchError {
    #[error("network request failed")]
    Network(#[from] reqwest::Error),  // ? from reqwest auto-wraps

    #[error("invalid utf8 in response")]
    Encoding(#[from] std::str::Utf8Error),
}
```

`#[from]` = derive `From<inner>`. `?` operator works. No manual `.map_err`.

**Caveat:** `#[from]` only works if variant has *exactly one* unnamed field. Multiple → use `#[source]` + manual `.map_err(...)?`.

### Transparent wrap

```rust
#[derive(thiserror::Error, Debug)]
pub enum DbError {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),  // forward Display + source

    #[error("migration failed: {name}")]
    Migration { name: String, #[source] source: sqlx::migrate::MigrateError },
}
```

`transparent` = pass through to inner. Use when wrapping w/o adding info.

## anyhow — bins + scripts only

### When OK

- `main()` in binary
- One-off CLI tool
- Test helpers
- Migration scripts
- Glue code where caller will only print + exit

### Pattern

```rust
use anyhow::{Context, Result, bail};

fn main() -> Result<()> {
    let cfg = load_config("./config.toml")
        .context("failed to load config at startup")?;

    if cfg.workers == 0 {
        bail!("workers must be > 0, got 0");
    }

    run(cfg).context("worker pool exited with error")?;
    Ok(())
}
```

### Rules

- **`.context("...")` on every `?`.** Build trace. No bare `?` in bin.
- **`bail!` for early Err.** Not `return Err(anyhow!(...))`.
- **`anyhow!("msg")` for ad-hoc.** No need to define type.
- **Never `anyhow::Error` in lib pub API.** Caller stuck.
- **Downcast rare.** If you reach for `.downcast_ref::<SpecificError>()` often → switch to thiserror.

## Error messages — habitability

Per `rex-code-philosophy`: errors carry context. Reader 2am, paged, must fix.

### Hard rules

- **Concrete value seen.** "expected u32, got -1" > "invalid input".
- **What was tried.** "failed to read /etc/foo.toml" > "read failed".
- **Actionable next step (when known).** "missing field `port`. Add `port = 8080` to [server] section." > "missing field".
- **No "Error:" prefix.** Caller add it. You write the noun.
- **No trailing period.** Display chain re-format. Period collide.
- **Lowercase first letter.** Composes in chains. Per Rust API guidelines.

```rust
// bad
#[error("Error: Failed to do thing.")]
Whatever,

// good
#[error("failed to bind tcp listener on {addr}")]
Bind { addr: SocketAddr },
```

### Error chain reads top-down

```
failed to start server
  caused by: failed to bind tcp listener on 0.0.0.0:8080
  caused by: address already in use (os error 98)
```

Each layer add specific context. Don't repeat parent.

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
- **Swallow w/ `let _ = ...`.** Log it or handle. Never silent drop.
- **`println!` instead of return.** Library not own stdout.
- **Catch-all `Other(String)` variant.** Forces stringly-typed match. Define variant or wrap concrete.

## Common patterns

### Map external error to your variant

```rust
fs::read_to_string(&path)
    .map_err(|source| ConfigError::Read { path: path.clone(), source })?;
```

### Multi-source error (no `#[from]`)

```rust
serde_json::from_str(&s)
    .map_err(|source| ConfigError::Parse {
        path: path.clone(),
        line: 0,  // serde doesn't expose, document
        source,
    })?;
```

### Option → Result

```rust
let port = cfg.port.ok_or(ConfigError::Invalid {
    field: "port",
    value: "<missing>".into(),
    expected: "u16",
})?;
```

### Collect Results

```rust
let parsed: Result<Vec<_>, _> = lines.iter().map(parse_line).collect();
let parsed = parsed?;  // first err short-circuits
```

## Migration: anyhow → thiserror

Common path: prototype w/ anyhow → harden into thiserror as API stabilizes.

1. Identify error sources (each `.context(...)` = candidate variant).
2. Define enum w/ `thiserror`.
3. Replace `anyhow::Error` return type w/ your enum.
4. Add `#[from]` for upstream errors.
5. Caller updates: now can match.

Don't migrate `main.rs` itself. anyhow stays there.

## Doc comments on errors

Per `rex-code-commenting`:

```rust
/// Load and parse config from disk.
///
/// # Errors
/// - [`ConfigError::NotFound`] if `path` does not exist
/// - [`ConfigError::Read`] for IO errors during read
/// - [`ConfigError::Parse`] for malformed TOML/JSON
pub fn load(path: &Path) -> Result<Config, ConfigError>
```

`# Errors` section = required on pub fns returning Result. Lints catch missing.

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

Fail any → fix before push.
