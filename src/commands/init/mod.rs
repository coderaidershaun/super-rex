//! `rex init`: sets a repo up in one run that is safe to repeat and never
//! replaces what the person has, with the sandbox settings first.

mod agents_md;
mod error;
mod file_change;
mod gitignore;
mod install;
mod on_path;
mod pre_commit;
mod preflight;
mod safety;
mod settings;

use std::io::{self, Write};
use std::path::Path;

use super::stdout::print_line_to_stdout;
use crate::store::{Store, StoreError, StorePaths};
pub use error::InitError;
use file_change::FileChange;
use install::{install_bundle, summary};
use pre_commit::{ConfigWrite, HookInstall};

/// What `rex init` installs and whether it replaces files it wrote before.
#[derive(clap::Args, Clone, Copy, Debug)]
#[command(
    after_help = "Run it from your own shell, in the root of the main checkout (the folder that holds .git). It never replaces a file or setting of yours; a kept value that weakens a safety setting is reported, and the run exits 1."
)]
pub struct InitOptions {
    /// Overwrite installed skill, agent and library files that differ from this version of rex.
    #[arg(long = "force")]
    pub is_forced: bool,

    /// Leave out the hooks that record sub-agent runs (no telemetry).
    #[arg(long = "no-hooks")]
    pub is_without_hooks: bool,

    /// Never let an agent run a command outside the sandbox.
    #[arg(long = "strict")]
    pub is_strict: bool,
}

/// Set up the repo at `cwd`. The checks come first: `cwd` must be the root of
/// a Git repository, and a recent enough Claude Code must be on PATH. Nothing
/// is written when one fails.
///
/// Then, in this order: the safety settings and, unless `is_without_hooks`,
/// the hooks in `.claude/settings.local.json`; the skills and agents in
/// `.claude` and the skill library in `.rex/skills`; the rex sections of
/// `AGENTS.md`; the block in `.gitignore`; the store in `.rex`; the
/// pre-commit config, and its hooks when `pre-commit` is
/// on PATH. The settings come first, so a step that stops the install never
/// leaves the skills and agents in place without the sandbox.
///
/// Nothing of the person's is replaced, and nothing is written through a
/// symbolic link. `is_forced` overwrites only the skill, agent and library files that
/// differ from the bundle. A value of the person's that weakens a safety
/// setting is kept, and it fails the run. `is_strict` writes the setting that
/// keeps every command inside the sandbox.
///
/// A failure that leaves the rest of the install possible, such as a settings
/// file rex cannot merge, does not stop it. Each one is printed on stderr at
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
    let change = settings::ensure_settings(cwd, options, failures)?;
    report_file_change(settings::SETTINGS_FILENAME, change)?;

    let counts = install_bundle(cwd, options.is_forced)?;
    print_line_to_stdout(&summary(counts))?;

    let mut to_commit = Vec::new();

    let agents_md = agents_md::AGENTS_FILENAME;
    let change = agents_md::ensure_rex_sections(cwd)?;
    report_file_change(agents_md, change)?;
    if change != FileChange::Unchanged {
        to_commit.push(agents_md);
    }

    let gitignore = gitignore::GITIGNORE_FILENAME;
    let change = gitignore::ensure_ignore_block(cwd)?;
    report_file_change(gitignore, change)?;
    if change != FileChange::Unchanged {
        to_commit.push(gitignore);
    }

    match set_up_store(cwd) {
        Ok(change) => report_file_change(".rex/", change)?,
        Err(source) => failures.push(InitError::Store { source }),
    }

    set_up_pre_commit(cwd, &mut to_commit, failures)?;

    if let Some(line) = commit_line(&to_commit) {
        print_line_to_stdout(&line)?;
    }
    Ok(())
}

/// Write the pre-commit config when it is absent, adding it to `to_commit`,
/// and turn it into git hooks when `pre-commit` is on PATH. A failed
/// `pre-commit install` is added to `failures`.
///
/// # Errors
/// The failure that stops the install: the config cannot be read or
/// written, `pre-commit` cannot be started, or a line cannot be printed.
fn set_up_pre_commit(
    cwd: &Path,
    to_commit: &mut Vec<&str>,
    failures: &mut Vec<InitError>,
) -> Result<(), InitError> {
    let config = pre_commit::CONFIG_FILENAME;
    match pre_commit::ensure_config(cwd)? {
        ConfigWrite::Created => {
            print_line_to_stdout(&format!("rex init: {config} created"))?;
            to_commit.push(config);
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

/// The reminder to commit `files`, or `None` when there is nothing to
/// commit. A job works in the main checkout only when it is clean, so these
/// files must not stay uncommitted.
fn commit_line(files: &[&str]) -> Option<String> {
    let named = match files {
        [] => return None,
        [one] => (*one).to_string(),
        [first @ .., last] => format!("{} and {last}", first.join(", ")),
    };
    Some(format!(
        "rex init: commit {named} (git add + git commit); a job works in the main checkout only when it is clean"
    ))
}

/// Make the store and its orchestrator folder, or open the store when it is
/// already there. Opening a store of this version changes no file in it, so
/// a second run leaves `.rex/` as it was.
fn set_up_store(cwd: &Path) -> Result<FileChange, StoreError> {
    let paths = StorePaths::locate(cwd)?;
    // The skill library is installed into `.rex/` before this step, so the
    // folder alone does not tell whether the store is new.
    let was_there = paths.db.exists();
    drop(Store::create(&paths)?);
    if was_there {
        Ok(FileChange::Unchanged)
    } else {
        Ok(FileChange::Created)
    }
}

/// Print a warning, a skipped path or a failure on stderr. A line that cannot
/// be shown must not stop the install or hide the lines after it, so a failed
/// write is ignored.
fn print_line_to_stderr(line: &str) {
    let _ = writeln!(io::stderr().lock(), "{line}");
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
    for failure in failures {
        print_line_to_stderr(&format!("rex init: failed: {}", failure.with_sources()));
    }
    Err(InitError::Incomplete {
        count: failures.len(),
    })
}
