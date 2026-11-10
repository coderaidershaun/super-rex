//! `rex oms`: prints the grid agent for a set of inputs.

use std::path::Path;

use crate::error::RexError;

/// Arguments of `rex oms`.
#[derive(clap::Args)]
pub struct OmsOptions {}

/// # Errors
/// [`RexError::NotBuilt`] until the command is built.
pub fn run(_cwd: &Path, _options: OmsOptions) -> Result<(), RexError> {
    Err(RexError::NotBuilt { command: "oms" })
}
