//! `rex codebase`: prints a tree outline of the working directory.

use std::path::Path;

use crate::error::RexError;

/// Arguments of `rex codebase`.
#[derive(clap::Args)]
pub struct CodebaseOptions {}

/// # Errors
/// [`RexError::NotBuilt`] until the command is built.
pub fn run(_cwd: &Path, _options: CodebaseOptions) -> Result<(), RexError> {
    Err(RexError::NotBuilt {
        command: "codebase",
    })
}
