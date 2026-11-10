//! `rex init`: installs the embedded skills and agents into the repo, points
//! agents at the `rex codebase` and `rex smells` commands through
//! `AGENTS.md`, keeps the installed files out of git through `.gitignore`,
//! and sets up pre-commit hooks (gitleaks, trivy, cargo fmt, clippy, test).

mod agents_md;
mod file_change;
mod gitignore;
mod install;
mod pre_commit;

use std::path::Path;

use super::stdout::print_line_to_stdout;
use crate::error::RexError;
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

/// Write every file from the embedded bundle into `cwd/.claude`. A file
/// already present on disk is left untouched and counted, unless it differs
/// from the bundle and `is_forced` is set, in which case it is overwritten.
///
/// Then make sure `cwd/AGENTS.md` has the rex sections, `cwd/.gitignore`
/// ignores the installed skill files, and `cwd/.pre-commit-config.yaml` is
/// there, and run `pre-commit install` when the tool is on PATH. Each file is
/// added once and an existing file is never replaced, with or without
/// `is_forced`.
///
/// None of these files is written through a symbolic link. One that is a link
/// is named on stderr and left alone, and the install carries on.
///
/// # Errors
/// [`RexError::Io`] if a parent directory or file cannot be created or written,
/// or if `AGENTS.md`, `.gitignore` or the pre-commit config cannot be read or
/// written; [`RexError::PreCommitInstall`] if `pre-commit install` fails;
/// [`RexError::Stdout`] if a progress line cannot be written to stdout.
pub fn run(cwd: &Path, options: InitOptions) -> Result<(), RexError> {
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
    match pre_commit::install_hooks(cwd)? {
        HookInstall::Installed => print_line_to_stdout("rex init: pre-commit hooks installed")?,
        HookInstall::SkippedNoGitDir => {
            print_line_to_stdout("rex init: pre-commit hooks skipped (no .git directory)")?;
        }
        HookInstall::SkippedNoTool => print_line_to_stdout(&format!(
            "rex init: pre-commit hooks skipped (pre-commit not found; install it, then run: {install})"
        ))?,
    }
    Ok(())
}

fn report_file_change(filename: &str, change: FileChange) -> Result<(), RexError> {
    match change {
        FileChange::Created => print_line_to_stdout(&format!("rex init: {filename} created")),
        FileChange::Updated => print_line_to_stdout(&format!("rex init: {filename} updated")),
        FileChange::Unchanged => Ok(()),
    }
}
