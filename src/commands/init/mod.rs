//! `rex init`: sets SuperRex up in a repo.

use std::path::Path;

use crate::error::RexError;

/// Arguments of `rex init`.
#[derive(clap::Args)]
pub struct InitOptions {}

/// # Errors
/// [`RexError::NotBuilt`] until the command is built.
pub fn run(_cwd: &Path, _options: InitOptions) -> Result<(), RexError> {
    Err(RexError::NotBuilt { command: "init" })
}
