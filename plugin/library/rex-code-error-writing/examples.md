# Rust error examples

Rules for each example are in `rex-code-error-writing/SKILL.md`, under the same heading.

## thiserror — default

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

## anyhow — bins + scripts only

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

## Doc comments on errors

```rust
/// Load and parse config from disk.
///
/// # Errors
/// - [`ConfigError::NotFound`] if `path` does not exist
/// - [`ConfigError::Read`] for IO errors during read
/// - [`ConfigError::Parse`] for malformed TOML/JSON
pub fn load(path: &Path) -> Result<Config, ConfigError>
```
