//! `rex watch`: a live view of jobs and their agent trees, for people.

use std::path::Path;

use crate::error::RexError;

/// Arguments of `rex watch`.
#[derive(clap::Args)]
pub struct WatchOptions {}

/// # Errors
/// [`RexError::NotBuilt`] until the command is built.
pub fn run(_cwd: &Path, _options: WatchOptions) -> Result<(), RexError> {
    Err(RexError::NotBuilt { command: "watch" })
}
