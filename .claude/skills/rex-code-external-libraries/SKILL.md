---
name: rex-code-external-libraries
description: >
  Hard rules for adding external Rust crates: latest stable version from
  crates.io, only needed features (default-features = false), and docs.rs for
  an unfamiliar crate.
when_to_use: >
  Use when you add or upgrade a crate, choose features, or pick a version, or
  the user asks "add crate X", "what version", "which features", "is this crate
  idiomatic".
disable-model-invocation: false
user-invocable: true
---

# Rust External Libraries — Hard Rules

These rules HARD. No compromise. Apply every time crate enter `Cargo.toml`.

Sister skills:
- `rex-code-philosophy` — complexity-averse. Crate = imported complexity
- `rex-code-ergonomics` — newtypes around domain primitives
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
- **Pre-release version.** Try it in spike (`rex-code-spike`): scratch dir outside repo. Never in source tree.
- **Default features when subset suffices.** Audit + trim.
- **Two TLS backends linked.** Pick one.
- **`git = "..."` dep w/o pinned `rev`.** Floating git = surprise breakage. Pin SHA or use crates.io.
- **Unused crate.** Dead deps rot. Remove it. `cargo machete` finds them: run it before commit IF installed. IF repo runs checks as git hooks, THEN add it to pre-push.
- **New crate for one fn that std already has.** Use std first.
- **Crate type in your `pub` API** when crate is not part of your contract. Its major bump becomes yours. Convert to own type at edge.

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
