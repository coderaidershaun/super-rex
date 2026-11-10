//! `rex job`: the only commands that change a job's stored status.

mod close;
mod create;
mod error;
mod input;
mod plan;
mod record;
mod show;

pub use close::CloseOptions;
pub use create::CreateOptions;
pub use error::JobError;
pub use plan::PlanOptions;
pub use show::ShowOptions;

use std::path::Path;

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
    Create(CreateOptions),

    /// Record the work breakdown, the orchestrator and the role table, and move the job to planned.
    Plan(PlanOptions),

    /// Record the outcome, with one delivery for each repository the job changed.
    Close(CloseOptions),

    /// Print one job's record.
    Show(ShowOptions),
}

/// # Errors
/// [`JobError`]: a usage error for a wrong command line; otherwise a failure
/// of the store, an input file or the job's state.
pub fn run(cwd: &Path, options: JobOptions) -> Result<(), JobError> {
    match options.command {
        JobCommand::Create(options) => create::run(cwd, options),
        JobCommand::Plan(options) => plan::run(cwd, options),
        JobCommand::Close(options) => close::run(cwd, options),
        JobCommand::Show(options) => show::run(cwd, options),
    }
}
