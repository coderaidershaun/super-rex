//! `rex init`: sets a repo up in one run that is safe to repeat and never
//! replaces what the person has, with the sandbox settings first.

mod agents_md;
mod config;
mod error;
mod file_change;
mod gitignore;
mod old_copies;
mod on_path;
mod plugin;
mod pre_commit;
mod preflight;
mod safety;
mod settings;

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use super::stdout::print_line_to_stdout;
use crate::store::{Migrated, Store, StoreError, StorePaths};
pub use error::InitError;
use file_change::FileChange;
use plugin::PluginInstall;
use pre_commit::{ConfigWrite, HookInstall};

/// The sub-agent depth a master tree needs: master and builder at depth 1,
/// leads at 2, workers at 3 and their own fan-out at 4.
pub const MASTER_DEPTH: u32 = 4;

/// Set SuperRex up in a repo. Safe to run twice.
#[derive(clap::Args, Clone, Debug)]
#[command(
    after_help = "Run it from your own shell, in the root of the main checkout (the folder that holds .git). It installs the super-rex plugin at project scope through the claude command and never replaces a file or setting of yours; a kept value that weakens a safety setting is reported, and the run exits 1. Entries that ran every git command outside the sandbox (\"git *\", \"Bash(git *)\") are taken out with a notice: rex lists its job worktrees instead, so git works in the sandbox."
)]
pub struct InitOptions {
    /// Never let an agent run a command outside the sandbox.
    #[arg(long = "strict")]
    pub is_strict: bool,

    /// Keep the record of jobs in .rex/rex.db: recall, feedback, cost of record and master runs need it.
    #[arg(long = "with-storage")]
    pub is_with_storage: bool,

    /// The most levels of sub-agents a session may start [default: 4, or a higher value of yours].
    #[arg(long, value_name = "LEVELS", value_parser = clap::value_parser!(u32).range(1..=64))]
    pub depth: Option<u32>,

    /// Where the plugin's marketplace is: a folder, or a GitHub owner/repo.
    #[arg(long, value_name = "PATH_OR_REPO", default_value = plugin::DEFAULT_SOURCE)]
    pub plugin_source: String,
}

/// Set up the repo at `cwd`. The checks come first: `cwd` must be the root of
/// a Git repository, and a recent enough Claude Code must be on PATH. Nothing
/// is written when one fails.
///
/// Then, in this order: the safety settings, the deny rules, the job
/// worktrees in `<parent>/<name>.worktrees/` and the sub-agent depth in
/// `.claude/settings.local.json`, with the hook entries of rex 0.7 and the
/// stopgap entries that ran git outside the sandbox taken out;
/// `.rex/config.json`; the plugin, through `claude plugin`; once it is in, the
/// removal of each unchanged copy that rex 0.4 to 0.7 put in `.rex/skills/`,
/// `.claude/skills/` or `.claude/agents/`; the rex sections of `AGENTS.md`; the block in `.gitignore`; with storage on, the
/// store in `.rex`; the pre-commit config, and its hooks when `pre-commit` is
/// on PATH. The settings come first, so a step that stops the install never
/// leaves the plugin in place without the sandbox.
///
/// Nothing of the person's is replaced, and nothing is written through a
/// symbolic link. A value of the person's that weakens a safety setting is
/// kept, and it fails the run. `is_strict` writes the setting that keeps every
/// command inside the sandbox.
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
    if let Err(stop) = install(cwd, &options, &mut failures) {
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
    options: &InitOptions,
    failures: &mut Vec<InitError>,
) -> Result<(), InitError> {
    let (change, depth) = settings::ensure_settings(cwd, options, failures)?;
    report_file_change(settings::SETTINGS_FILENAME, change)?;

    let mut to_commit = Vec::new();

    let wanted = config::Wanted {
        is_storage_on: options.is_with_storage,
        depth,
    };
    let is_storage_on = match config::ensure_config(cwd, wanted) {
        Ok((change, is_storage_on)) => {
            report_file_change(config::CONFIG_FILENAME, change)?;
            if change != FileChange::Unchanged {
                to_commit.push(config::CONFIG_FILENAME);
            }
            is_storage_on
        }
        Err(failure) => {
            failures.push(failure);
            options.is_with_storage
        }
    };

    let is_plugin_in = match plugin::install(cwd, &options.plugin_source) {
        Ok(PluginInstall::Installed) => {
            print_line_to_stdout(&format!(
                "rex init: plugin {} installed at project scope from {}",
                plugin::PLUGIN,
                options.plugin_source
            ))?;
            true
        }
        Ok(PluginInstall::AlreadyInstalled) => {
            print_line_to_stdout(&format!(
                "rex init: plugin {} skipped (already installed at project scope)",
                plugin::PLUGIN
            ))?;
            true
        }
        Err(failure) => {
            failures.push(failure);
            false
        }
    };

    // Only once the plugin is in, so a repo whose install failed keeps the
    // old copies of the library until the plugin's library replaces them.
    if is_plugin_in {
        old_copies::remove_old_copies(cwd)?;
    }

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

    if is_storage_on {
        match set_up_store(cwd) {
            Ok(change) => report_file_change(".rex/rex.db", change)?,
            Err(failure) => failures.push(failure),
        }
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
///
/// A store of rex 0.7 (version 1) is first copied to a backup beside it and
/// then migrated by [`Store::migrate`], the code of `rex migrate`. That
/// migration carries every row over (`asked_by` human or agent becomes origin
/// composer or sdk) in one transaction, so it needs no choice of the person's,
/// with one exception: a row that names a row that is not there. Then nothing
/// is written, the backup goes, and the failure names the command to run.
fn set_up_store(cwd: &Path) -> Result<FileChange, InitError> {
    let store_failed = |source| InitError::Store { source };
    let paths = StorePaths::locate(cwd).map_err(store_failed)?;
    let version = Store::version_on_disk(&paths).map_err(store_failed)?;
    if version == Some(1) {
        migrate_version_one(&paths)?;
    }
    drop(Store::create(&paths).map_err(store_failed)?);
    if version.is_some() {
        Ok(FileChange::Unchanged)
    } else {
        Ok(FileChange::Created)
    }
}

/// Back up the version 1 store at `paths`, migrate it, and say so. On any
/// failure the store is as it was, and a backup this run made is removed.
fn migrate_version_one(paths: &StorePaths) -> Result<(), InitError> {
    let backup = free_backup_path(&paths.db);
    Store::back_up(paths, &backup).map_err(|source| InitError::Store { source })?;
    let migrated = Store::migrate(paths).map_err(|source| match source {
        StoreError::BrokenReferences { count, .. } => InitError::StoreNeedsChoice { count },
        source => InitError::Store { source },
    });
    let jobs = match migrated {
        Ok(Migrated::FromVersionOne { jobs }) => jobs,
        // Another rex migrated it in the meantime: the backup is still of version 1.
        Ok(Migrated::AlreadyCurrent) => 0,
        Err(failure) => {
            let _ = fs::remove_file(&backup);
            return Err(failure);
        }
    };
    let name = backup
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    print_line_to_stdout(&format!(
        "rex init: .rex/rex.db migrated from version 1 to 2 ({jobs} {}), as rex migrate does; the version 1 store is kept in .rex/{name}",
        if jobs == 1 { "job" } else { "jobs" }
    ))?;
    Ok(())
}

/// `rex.db.v1-backup` beside `db`, or the first of `rex.db.v1-backup-2`,
/// `-3` and on that is free, so an earlier backup is never replaced.
fn free_backup_path(db: &Path) -> PathBuf {
    let first = db.with_file_name("rex.db.v1-backup");
    if !first.exists() {
        return first;
    }
    // With all of them taken, the copy fails on the first, and says so.
    (2..1000)
        .map(|n| db.with_file_name(format!("rex.db.v1-backup-{n}")))
        .find(|path| !path.exists())
        .unwrap_or(first)
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
