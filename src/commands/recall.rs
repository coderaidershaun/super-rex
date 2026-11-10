//! `rex recall`: lessons from similar past jobs, and the orchestrators those
//! jobs used.

use std::path::Path;

use crate::error::RexError;

/// Arguments of `rex recall`.
#[derive(clap::Args)]
pub struct RecallOptions {}

/// # Errors
/// [`RexError::NotBuilt`] until the command is built.
pub fn run(_cwd: &Path, _options: RecallOptions) -> Result<(), RexError> {
    Err(RexError::NotBuilt { command: "recall" })
}
