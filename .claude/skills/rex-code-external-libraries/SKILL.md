---
name: rex-code-external-libraries
description: This skill gives hard rules for adding external Rust crates. Find the latest stable version on crates.io before you add a crate. Activate only the features that the code needs (default-features = false). When a crate is unfamiliar, make sure of its syntax on docs.rs. Use it when you add or upgrade a crate, choose features, or pick a version. Use it also when the user asks "add crate X", "what version", "which features", "is this crate idiomatic".
disable-model-invocation: false
user-invocable: true
---

# Rust External Libraries — Hard Rules

These rules HARD. No compromise. Apply every time crate enter `Cargo.toml`.

Sister skills:
- `rex-code-philosophy` — complexity-averse. Crate = imported complexity
- `rex-code-ergonomics` — wrap external types in newtypes at boundary
- `rex-code-error-writing` — `#[from]` external errors via thiserror

## Iron rule

**Pin latest stable. Minimal features. IF crate unfamiliar, THEN make sure of syntax.**

Three checks per add. No skip:

1. **Latest stable** — query crates.io. Use exact version compatible w/ rust-toolchain.
2. **Minimal features** — `default-features = false`. Add only features that code uses.
3. **Familiar?** — IF yes, THEN add. ELSE fetch docs.rs first. Confirm API shape.

## Step 1 — find latest stable

```bash
cargo search <crate> --limit 1
# or
curl -s https://crates.io/api/v1/crates/<crate> | jq -r '.crate.max_stable_version'
```

Rules:
- **Use `max_stable_version`.** Skip yanked. Skip pre-release (`-alpha`, `-rc`, `-beta`) unless user explicit.
- **Match MSRV.** `cat rust-toolchain.toml` first. IF crate's MSRV > project MSRV, THEN pick older release or bump toolchain (ask user).
- **Look for yanked.** `cargo info <crate>` shows yanked. Never use yanked version.
- **No `*` or `latest`.** Caret semver only: `"1.4"` (= `^1.4`).

## Step 2 — features

Default features = bloat trap. Crate authors activate everything for ergonomics.

```toml
# bad — pulls every feature including unused TLS, blocking, json, multipart...
reqwest = "0.12"

# good — only what fn actually call
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json"] }
```

Rules:
- **`default-features = false`** unless code uses every default.
- **List features explicitly.** Reader see exactly what crate adds.
- **One TLS impl per build.** `rustls-tls` XOR `native-tls`. Mix = link conflict.
- **Async runtime feature must match project.** `tokio` features → `tokio` runtime. `async-std` features → `async-std`. No mix.
- **`derive` feature opt-in.** `serde = { version = "1", features = ["derive"] }`. IF no `#[derive(Serialize)]`, THEN skip.

Find feature list:
```bash
cargo info <crate>          # shows features
# or docs.rs/<crate>/<version>/<crate>/#features
```

## Step 3 — unfamiliar crate: make sure of API

**Familiar bar:** used crate at this major version w/in same project, or std-tier (`serde`, `tokio`, `anyhow`, `thiserror`, `clap`, `reqwest`).

IF unfamiliar, THEN do these steps before you write code:

1. **Fetch docs.rs.** `https://docs.rs/<crate>/<version>/<crate>/`
2. **Read module root + `examples/`.** Confirm public API shape.
3. **Look for breaking changes.** IF major version jump (1.x → 2.x), THEN read CHANGELOG. API often shifts silently.
4. **Run smallest example.** Throwaway `cargo new`. Confirm it compiles + runs as expected.

IF still unsure, THEN ask user. Do not guess.

## Add command

```bash
cargo add <crate>@<version> --no-default-features --features <f1>,<f2>
# dev-only
cargo add --dev <crate>@<version> ...
# build script
cargo add --build <crate>@<version> ...
```

`cargo add` writes `Cargo.toml` correctly. Do not hand-edit unless workspace inheritance.

## Workspace inheritance

IF multi-crate repo, THEN pin in workspace root:

```toml
# workspace Cargo.toml
[workspace.dependencies]
serde = { version = "1.0.215", default-features = false, features = ["derive"] }

# member Cargo.toml
[dependencies]
serde = { workspace = true }
```

One version. One feature set. No drift between members.

## Forbidden — fix on sight

- **`*` or unbounded version.** Pin caret semver.
- **Yanked version.** Bump.
- **Pre-release in main branch.** Spike branch only, w/ ADR.
- **Default features when subset suffices.** Audit + trim.
- **Two TLS backends linked.** Pick one.
- **`git = "..."` dep w/o pinned `rev`.** Floating git = surprise breakage. Pin SHA or use crates.io.
- **Crate that someone added "to try" and never removed.** Dead deps rot. `cargo machete` periodically.
- **New crate for one fn that std already has.** Use std first.

## Anti-patterns

| Bad | Why | Fix |
|-----|-----|-----|
| `chrono = "0.4"` w/ defaults | Pulls `oldtime`, `wasmbind`, `clock` unused | `default-features = false`. IF needed, THEN add `["serde", "std"]` |
| `tokio = { version = "1", features = ["full"] }` | `full` = every runtime feature. Bloats compile + binary | List actual features: `["rt-multi-thread", "macros", "net"]` |
| Hand-edit `Cargo.toml` w/ guessed version | Version can be missing or yanked | `cargo add` resolves to real |
| Add crate in `[dependencies]` for tests only | Ships in release binary | Move to `[dev-dependencies]` |
| Pin `git = "https://..."` no `rev` | HEAD moves silently | `rev = "abc123..."` or use crates.io |
| Skip docs.rs check on unfamiliar crate | Compile errors / wrong API / runtime surprise | Read docs first |

## Ordering
[dependencies] items: alphabetical order.

## Checklist before commit

- [ ] Version = latest stable (or justified pin)
- [ ] Not yanked
- [ ] MSRV compatible w/ `rust-toolchain.toml`
- [ ] `default-features = false` unless every default used
- [ ] Feature list matches actual usage
- [ ] One TLS impl across project
- [ ] Workspace inheritance used in multi-crate repos
- [ ] Dev-only crates in `[dev-dependencies]`
- [ ] Docs.rs read (unfamiliar crate)
- [ ] `cargo build` + `cargo test` green after add
- [ ] No `git` dep without pinned `rev`

IF any fail, THEN fix before push.
