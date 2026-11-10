# C11. Conventions for every `rex` command

Status: DRAFT

## 1. What this file is

These rules apply to every `rex` command and to every lane that writes code or tests in this crate. They say where your code goes, how a command prints, which exit code it uses, how it words an error, how it adds an error type, and which crates you can use.

## 2. Where your code goes

The library is `super_rex` (`src/lib.rs`). The binary is `rex` (`src/main.rs`). Every module and test module below already exists as a stub. You do your work by replacing the stubs in your own paths.

| Lane | Code | Test modules, in each of `tests/integration/`, `tests/contract/`, `tests/fitness/` |
| --- | --- | --- |
| 1 Spine | The shared files (below) | `tests/integration/cli.rs` only |
| 2 Store and jobs | `src/store/`, `src/commands/job/` | `store_jobs.rs` |
| 3 Capture | `src/capture/`, `src/commands/hook.rs`, `src/commands/status.rs`, `tests/fixtures/capture/` | `capture.rs` |
| 4 Init and safety | `src/bundle.rs`, `src/commands/init/` | `init.rs` |
| 5 Codebase | `src/commands/` files `codebase.rs`, `smells.rs`, `catalog.rs`, `walk.rs`, `comments.rs`, `language.rs`, `module_comment.rs` | `codebase.rs` |
| 6 OMS and skill commands | `src/oms/`, `src/commands/oms.rs`, `src/commands/skills/` | `oms_skills.rs` |
| 7 Experience and views | `src/commands/` files `recall.rs`, `feedback.rs`, `jobs.rs`, and `watch/` | `experience.rs` |
| 8 Rulebooks and grid | `.claude/skills/rex-code-*/`, `.claude/skills/rex-skill-language/`, `.claude/agents/`, `docs/model-selection-inputs.md` | none |
| 9 Builder skills | `.claude/skills/orchestration-builder/` and its three sub-skill folders | none |
| 10 Delivery skills | `.claude/skills/rex-deliver/`, `rex-pr-review/`, `rex-feedback/` | none |

**Handler shape.** Each command module has an options struct that derives `clap::Args` and one `run` function:

```rust
pub fn run(cwd: &Path, options: RecallOptions) -> Result<(), E>
```

`E` is any error type that is `std::error::Error + Send + Sync + 'static`. `main.rs` passes `cwd` as `.`. Add flags and arguments to your options struct; `main.rs` does not change. `rex job` and `rex skills` hold their subcommands in the options struct: `JobOptions { #[command(subcommand)] pub command: JobCommand }`, with the `clap::Subcommand` enum in the same file. One exception: `catalog::run(cli: &clap::Command, options: CommandsOptions)` also takes the clap command tree, because it prints it. Part 6 shows a full handler.

**Stubs.** Every stub `run` returns `RexError::NotBuilt` until its lane replaces it. The stub of `rex hook` returns `Ok(())` instead.

**One file becomes several.** Move `<name>.rs` to `<name>/mod.rs` and declare the children there with `mod`. This works the same in `src/` and in `tests/`. Do not keep both forms.

**Shared files belong to Lane 1:** `Cargo.toml`, `Cargo.lock`, `build.rs`, `src/main.rs`, `src/lib.rs`, `src/error.rs`, `src/commands/mod.rs`, `src/commands/stdout.rs`, `src/commands/links.rs`, the `main.rs` of each test folder, `.github/`, `README.md`, and `docs/contracts/` after a freeze. To change one, write the exact change (file, old text, new text) under `REQUESTS FOR LANE 1` in your report to the orchestrator.

**The intake.** `super_rex::capture::take_in_events(working_dir: &Path)` exists and does nothing. Its signature is provisional. When contract C5 is frozen, Lane 3's first merge puts the frozen signature in place, still with a body that does nothing. Lanes that call it merge their calls after that merge.

**Code with no caller yet.** An item in your own module that has no caller yet takes `#[allow(dead_code)]` in your own file, until the caller lands. The six private helper modules of `src/commands/` (`comments`, `language`, `links`, `module_comment`, `stdout`, `walk`) already have it on their `mod` lines.

## 3. Output style

- stdout holds the result and nothing else. Warnings and errors go to stderr.
- Plain text. No colour, no box characters, no progress lines. Agents read the output, so every line costs tokens: give one fact one time, in its shortest true form.
- One record on one line. Where a caller picks values out, print `key=value` lines in a fixed order, as `rex oms` does. The value a caller needs most comes first (`rex oms` prints `agent=` first).
- `--json` only where a contract gives it: one JSON document on one line, keys in `snake_case`, the same facts as the text form.
- Print through `crate::commands::stdout` (`print_line_to_stdout`, `print_to_stdout`). They treat a reader that closes the pipe early, such as `head`, as success, not failure.
- Library modules outside `src/commands/` (`src/store/`, `src/capture/`, `src/oms/`, `src/bundle.rs`) do not print.

## 4. Exit codes

| Code | Meaning |
| --- | --- |
| 0 | Done, also when there was nothing to do. |
| 1 | The command failed. stderr has the reason. `run` returned an error. |
| 2 | The command line is wrong. clap prints the usage. |

A command lets clap check its values (`clap::ValueEnum`, a value parser), so it never returns 2 by hand. `rex hook` always exits 0 and prints nothing. There are no other codes: a contract that needs one asks Lane 1.

## 5. Error text

- Lower-case first letter.
- No full stop at the end.
- No `Error:` prefix; `main` adds it.
- Name the value seen and the path.
- Give the fix when it is known.

`main` adds one line of context that names the top command, and prints the chain. A stub prints this to stderr and exits 1:

```
$ rex job create
Error: rex job failed

Caused by:
    `rex job` is not built yet
```

The first line names the top command only (`rex job`, not `rex job create`).

## 6. How to add an error

Define your own `thiserror` enum in your own module. `run` returns it. `main.rs` accepts any error type, so `src/error.rs` and `src/main.rs` do not change. A new variant never goes into `src/error.rs`. Code carried over from rex-skills keeps the `RexError` variants that exist. To pass on a failure of a shared helper (such as `print_line_to_stdout`, which returns `RexError`), wrap it with `#[error(transparent)]`, as below. The names and the file path are an illustration; contract C1 sets the real ones.

```rust
// src/store/mod.rs
use std::path::PathBuf;

use crate::error::RexError;

#[derive(thiserror::Error, Debug)]
pub enum StoreError {
    #[error("no store at {}; run `rex init` in this repo first", path.display())]
    NotFound { path: PathBuf },

    #[error(transparent)]
    Rex(#[from] RexError),
}
```

```rust
// src/commands/job/mod.rs
use std::path::Path;

use crate::commands::stdout::print_line_to_stdout;
use crate::store::StoreError;

/// # Errors
/// [`StoreError::NotFound`] if the repo has no store.
pub fn run(cwd: &Path, _options: JobOptions) -> Result<(), StoreError> {
    let path = cwd.join("rex.sqlite");
    if !path.exists() {
        return Err(StoreError::NotFound { path });
    }
    print_line_to_stdout(&format!("store={}", path.display()))?;
    Ok(())
}
```

## 7. Dependencies

`Cargo.toml` holds the exact versions. Every crate has its default features off, except `serde`.

| Crate | Kind | Features | For |
| --- | --- | --- | --- |
| `anyhow` | normal | `std` only | `src/main.rs` only |
| `clap` | normal | `derive`, `error-context`, `help`, `std`, `usage` | All commands |
| `thiserror` | normal | no default features | Every error enum |
| `ignore` | normal | no default features | Lane 5 (`walk`), `RexError::Walk` |
| `include_dir` | normal | no default features | Lane 4 (`bundle`) |
| `rusqlite` | normal | `bundled` | Lane 2 (SQLite, compiled in) |
| `serde` | normal | `derive`, with default `std` | Lanes 2, 3, 4, 6, 7 (JSON) |
| `serde_json` | normal | `std`, `preserve_order` | The same. Keeps key order when `rex init` merges a settings file. |
| `yaml-rust2` | normal | no default features | Lane 6 (skill headers) |
| `ratatui` | normal | `crossterm` | Lane 7 (`rex watch`) |
| `assert_cmd` | dev | no default features | Tests that run the binary |
| `tempfile` | dev | no default features | Tests in temporary folders |
| `sha2` | dev | no default features | Lane 6 (the OMS checksum test) |

There is no time crate, no random crate and no async runtime. Try std first: `std::time::SystemTime` for times, `std::hash::RandomState` for a random part, `std::fs::File::lock` for a file lock (the minimum Rust version is 1.89 or higher). To get one more crate or feature, write the crate, the features and the use under `REQUESTS FOR LANE 1` in your report to the orchestrator.
