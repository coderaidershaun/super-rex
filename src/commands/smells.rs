//! `rex smells`: lists every `SMELL` flag comment.

use std::path::Path;

use crate::error::RexError;

/// Arguments of `rex smells`.
#[derive(clap::Args)]
pub struct SmellsOptions {}

/// # Errors
/// [`RexError::NotBuilt`] until the command is built.
pub fn run(_cwd: &Path, _options: SmellsOptions) -> Result<(), RexError> {
    Err(RexError::NotBuilt { command: "smells" })
}
