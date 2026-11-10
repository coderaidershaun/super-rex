//! `rex init`: checks that the repo and Claude Code are fit for SuperRex,
//! then installs the embedded skills and agents into the repo, points agents
//! at the `rex codebase` and `rex smells` commands through `AGENTS.md`, keeps
//! the installed files out of git through `.gitignore`, and sets up
//! pre-commit hooks (gitleaks, trivy, cargo fmt, clippy, test).

mod agents_md;
mod error;
mod file_change;
mod gitignore;
mod install;
mod on_path;
mod pre_commit;
mod preflight;

use std::io::{self, Write};
use std::path::Path;

use super::stdout::print_line_to_stdout;
pub use error::InitError;
use file_change::FileChange;
use install::{install_bundle, summary};
use pre_commit::{ConfigWrite, HookInstall};

/// What `rex init` installs and whether it replaces files it wrote before.
#[derive(clap::Args, Clone, Copy, Debug)]
pub struct InitOptions {
    /// Overwrite installed skill and agent files that differ from this version of rex.
    #[arg(long = "force")]
    pub is_forced: bool,
}

/// Check that `cwd` is the root of a Git repository and that a recent
/// enough Claude Code is on PATH. Nothing is written when a check fails.
///
/// Then write every file from the embedded bundle into `cwd/.claude`. A file
/// already present on disk is left untouched and counted, unless it differs
/// from the bundle and `is_forced` is set, in which case it is overwritten.
/// Then make sure `cwd/AGENTS.md` has the rex sections, `cwd/.gitignore`
/// ignores the installed files, and `cwd/.pre-commit-config.yaml` is there,
/// and run `pre-commit install` when the tool is on PATH. Each file is added
/// once and an existing file is never replaced, with or without `is_forced`.
///
/// None of these files is written through a symbolic link. One that is a link
/// is named on stderr and left alone, and the install carries on.
///
/// A failure that leaves the rest of the install possible, such as a failed
/// `pre-commit install`, does not stop it. Each one is printed on stderr at
/// the end, in the order they happened.
///
/// # Errors
/// A preflight variant of [`InitError`] if a check fails before anything is
/// written; [`InitError::Incomplete`] if any step failed, each one named on
/// stderr first.
pub fn run(cwd: &Path, options: InitOptions) -> Result<(), InitError> {
    let mut failures = preflight::check(cwd)?;
    if let Err(stop) = install(cwd, options, &mut failures) {
        failures.push(stop);
    }
    finish(&failures)
}

/// Run each install step in order. A failure that the later steps do not
/// depend on is added to `failures`, and the install goes on.
///
/// # Errors
/// The first failure that stops the install: a file that cannot be read or
/// written, or a line that cannot be printed.
fn install(
    cwd: &Path,
    options: InitOptions,
    failures: &mut Vec<InitError>,
) -> Result<(), InitError> {
    let InitOptions { is_forced } = options;

    let counts = install_bundle(&cwd.join(".claude"), is_forced)?;
    print_line_to_stdout(&summary(counts))?;

    report_file_change(
        agents_md::AGENTS_FILENAME,
        agents_md::ensure_rex_sections(cwd)?,
    )?;
    report_file_change(
        gitignore::GITIGNORE_FILENAME,
        gitignore::ensure_ignore_block(cwd)?,
    )?;

    let config = pre_commit::CONFIG_FILENAME;
    match pre_commit::ensure_config(cwd)? {
        ConfigWrite::Created => {
            print_line_to_stdout(&format!("rex init: {config} created"))?;
            pre_commit::warn_about_missing_hook_tools();
        }
        ConfigWrite::AlreadyWritten => pre_commit::warn_about_missing_hook_tools(),
        ConfigWrite::SkippedExists => {
            print_line_to_stdout(&format!("rex init: {config} skipped (already exists)"))?;
        }
        // The link was already named on stderr, so stdout stays quiet.
        ConfigWrite::SkippedLink => {}
    }

    let install = pre_commit::INSTALL_COMMAND;
    match pre_commit::install_hooks(cwd) {
        Ok(HookInstall::Installed) => print_line_to_stdout("rex init: pre-commit hooks installed")?,
        Ok(HookInstall::SkippedNoTool) => print_line_to_stdout(&format!(
            "rex init: pre-commit hooks skipped (pre-commit not found; install it, then run: {install})"
        ))?,
        Err(failed @ InitError::PreCommitInstall { .. }) => failures.push(failed),
        Err(stop) => return Err(stop),
    }
    Ok(())
}

fn report_file_change(filename: &str, change: FileChange) -> Result<(), InitError> {
    match change {
        FileChange::Created => print_line_to_stdout(&format!("rex init: {filename} created"))?,
        FileChange::Updated => print_line_to_stdout(&format!("rex init: {filename} updated"))?,
        FileChange::Unchanged => {}
    }
    Ok(())
}

/// Name each failure on stderr, one line each, and turn them into one error
/// for the exit code.
fn finish(failures: &[InitError]) -> Result<(), InitError> {
    if failures.is_empty() {
        return Ok(());
    }
    let mut stderr = io::stderr().lock();
    for failure in failures {
        // A failure that cannot be shown must not hide the ones after it.
        let _ = writeln!(stderr, "rex init: failed: {}", failure.with_sources());
    }
    Err(InitError::Incomplete {
        count: failures.len(),
    })
}
