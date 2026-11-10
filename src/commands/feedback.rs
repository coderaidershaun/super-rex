//! `rex feedback`: records what the user said about a job, the root cause and
//! the lesson.

use std::path::Path;

use crate::error::RexError;

/// Arguments of `rex feedback`.
#[derive(clap::Args)]
pub struct FeedbackOptions {}

/// # Errors
/// [`RexError::NotBuilt`] until the command is built.
pub fn run(_cwd: &Path, _options: FeedbackOptions) -> Result<(), RexError> {
    Err(RexError::NotBuilt {
        command: "feedback",
    })
}
