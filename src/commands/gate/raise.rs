//! `rex limit`: shows what a job has spent against its limit, and sets a new limit,
//! which is how a person lets a refused job start sub-agents again.

use std::path::Path;
use std::time::SystemTime;

use super::limit::{Dollars, SpendLimit};
use crate::capture::{CaptureError, Cost, job_runs};
use crate::commands::stdout::print_to_stdout;
use crate::commands::token_text::cost_usd;
use crate::error::{CommandError, RexError};
use crate::store::{JobId, Store, StoreError, StorePaths};

/// Arguments of `rex limit`.
#[derive(clap::Args)]
#[command(
    after_help = "Prints `limit_usd=`, `spent_usd=` and `source=` (`default` from the job's budget and size, or `set`). Spend is the cost of the job's sub-agents and main session at list prices. Once spend reaches the limit, `rex gate` refuses new sub-agents."
)]
pub struct LimitOptions {
    /// The job whose spend and limit to print.
    pub job: JobId,

    /// The new limit in US dollars, such as 12 or 7.50. It replaces the old one.
    #[arg(long, value_name = "DOLLARS")]
    pub set: Option<Dollars>,
}

#[derive(thiserror::Error, Debug)]
pub enum LimitError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Capture(#[from] CaptureError),
    #[error(transparent)]
    Rex(#[from] RexError),
}

impl CommandError for LimitError {}

/// Sets the limit when `--set` is given, then prints the limit and the spend.
///
/// # Errors
/// - [`StoreError::NotFound`] if the repo has no store.
/// - [`StoreError::JobNotFound`] if the job is not in the store.
/// - [`CaptureError`] if the event log or the store cannot be read.
/// - [`RexError::Stdout`] if the output cannot be written.
pub fn run(cwd: &Path, options: LimitOptions) -> Result<(), LimitError> {
    let paths = StorePaths::discover(cwd)?;
    let mut store = Store::open(&paths)?;
    if let Some(dollars) = options.set {
        SpendLimit::set(&mut store, &options.job, dollars)?;
    }
    let runs = job_runs(&mut store, &options.job, SystemTime::now())?;
    let limit = SpendLimit::of(&store, &options.job)?;
    let spent = runs.cost.unwrap_or_default();
    let limit_cost = Cost {
        nanodollars: limit.nanodollars,
        unpriced_requests: 0,
    };
    print_to_stdout(&format!(
        "limit_usd={}\nspent_usd={}\nsource={}\n",
        cost_usd(&limit_cost),
        cost_usd(&spent),
        limit.source
    ))?;
    Ok(())
}
