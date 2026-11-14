//! `rex job show`: the whole record of one job, as the plugin and the feedback
//! skill read it. Jobs are written only by `rex ingest`.

mod error;
mod record;
mod show;
mod value;

pub use error::JobError;
pub(crate) use record::{QUESTION_COLUMNS, question_row as read_question, read as read_job};
pub use show::ShowOptions;
pub(crate) use show::{json_job, json_question};

use std::path::Path;

/// Show a job.
#[derive(clap::Args)]
pub struct JobOptions {
    #[command(subcommand)]
    pub command: JobCommand,
}

/// The commands of `rex job`.
#[derive(clap::Subcommand)]
pub enum JobCommand {
    /// Print one job's record.
    Show(ShowOptions),
}

/// # Errors
/// [`JobError`]: a usage error for a wrong command line; otherwise a failure
/// of the store.
pub fn run(cwd: &Path, options: JobOptions) -> Result<(), JobError> {
    match options.command {
        JobCommand::Show(options) => show::run(cwd, options),
    }
}
