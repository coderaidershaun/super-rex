//! `rex status`: where a job stands, kept short because agents read it.

use std::path::Path;

use crate::error::RexError;

/// Arguments of `rex status`.
#[derive(clap::Args)]
pub struct StatusOptions {}

/// # Errors
/// [`RexError::NotBuilt`] until the command is built.
pub fn run(_cwd: &Path, _options: StatusOptions) -> Result<(), RexError> {
    Err(RexError::NotBuilt { command: "status" })
}
