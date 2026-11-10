//! `rex jobs`: lists jobs for scripts and agents.

use std::path::Path;

use crate::error::RexError;

/// Arguments of `rex jobs`.
#[derive(clap::Args)]
pub struct JobsOptions {}

/// # Errors
/// [`RexError::NotBuilt`] until the command is built.
pub fn run(_cwd: &Path, _options: JobsOptions) -> Result<(), RexError> {
    Err(RexError::NotBuilt { command: "jobs" })
}
