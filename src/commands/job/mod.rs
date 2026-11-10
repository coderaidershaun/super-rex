//! `rex job`: the only commands that change a job's stored status.

use std::path::Path;

use crate::error::RexError;

/// Arguments of `rex job`.
#[derive(clap::Args)]
pub struct JobOptions {
    #[command(subcommand)]
    pub command: JobCommand,
}

/// The commands of `rex job`.
#[derive(clap::Subcommand)]
pub enum JobCommand {
    /// Open a job with its request, classification and session ID, and print the job ID.
    Create,

    /// Record the work breakdown, the orchestrator and the role table, and move the job to planned.
    Plan,

    /// Record the outcome, with one delivery for each repository the job changed.
    Close,

    /// Print one job's record.
    Show,
}

/// # Errors
/// [`RexError::NotBuilt`] until the command is built.
pub fn run(_cwd: &Path, _options: JobOptions) -> Result<(), RexError> {
    Err(RexError::NotBuilt { command: "job" })
}
