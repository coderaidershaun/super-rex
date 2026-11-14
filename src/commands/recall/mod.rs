//! `rex recall`: lessons from similar past jobs, and the orchestrators and cost
//! of those jobs. It records what it found on the job, so the builder need not.

mod report;
mod similar;

use std::path::Path;

use rusqlite::{Transaction, params};

use super::skills;
use super::stdout::print_line_to_stdout;
use crate::error::{CommandError, FailureClass, RexError};
use crate::store::orchestrator;
use crate::store::{JobId, Store, StoreError, StorePaths};

use report::{Report, plain_lines};
use similar::{asked_job, groups, lessons, unused};

/// Print lessons from similar past jobs and the orchestrators they used.
#[derive(clap::Args)]
#[command(
    after_help = "Writes what it prints into the job's record while the job is created (recorded=yes); in any other status it writes nothing (recorded=no). It also indexes the orchestrators in .rex/orchestrators."
)]
pub struct RecallOptions {
    /// The job to recall experience for
    #[arg(value_name = "JOB", value_parser = clap::value_parser!(JobId))]
    pub job: JobId,

    /// Print one JSON document instead of lines
    #[arg(long)]
    pub json: bool,
}

/// Failure of `rex recall`.
#[derive(thiserror::Error, Debug)]
pub enum RecallError {
    #[error("cannot write the recall as JSON")]
    Json(#[source] serde_json::Error),

    #[error(transparent)]
    Store(#[from] StoreError),

    #[error(transparent)]
    Rex(#[from] RexError),
}

impl CommandError for RecallError {
    fn failure_class(&self) -> FailureClass {
        match self {
            RecallError::Json(_) => FailureClass::Reject,
            RecallError::Store(error) => error.failure_class(),
            RecallError::Rex(error) => error.failure_class(),
        }
    }

    fn exit_code(&self) -> u8 {
        match self {
            RecallError::Store(error) => error.exit_code(),
            _ => {
                if self.is_usage() {
                    2
                } else {
                    1
                }
            }
        }
    }
}

/// # Errors
/// - [`RecallError::Store`] for a missing store, an unknown job, or a failed
///   read or write, including an orchestrator folder that cannot be read.
/// - [`RecallError::Json`] if the document cannot be written.
/// - [`RecallError::Rex`] if the output cannot be written.
pub fn run(cwd: &Path, options: RecallOptions) -> Result<(), RecallError> {
    let paths = StorePaths::discover(cwd)?;
    let mut store = Store::open(&paths)?;
    let (report, json) = store.write(|tx| recall(tx, &paths, &options.job))?;

    if options.json {
        print_line_to_stdout(&json)?;
    } else {
        for line in plain_lines(&report) {
            print_line_to_stdout(&line)?;
        }
    }
    Ok(())
}

/// Indexes the orchestrator folders, builds the report, and stores it on the
/// job while the job is `created`. The index is written before it is read, so
/// a folder added a minute ago shows at once. A job in any other status still
/// gets a fresh index; only storing the report is skipped.
fn recall(
    tx: &Transaction<'_>,
    paths: &StorePaths,
    job: &JobId,
) -> Result<(Report, String), RecallError> {
    let asked = asked_job(tx, paths, job)?;
    if paths.orchestrators.is_dir() {
        orchestrator::index_all(tx, &paths.orchestrators, &skills::describe)?;
    }

    let mut report = Report {
        schema: 1,
        job: job.to_string(),
        recorded: true,
        lessons: lessons(tx, job, &asked)?,
        orchestrators: groups(tx, paths, job, &asked)?,
        unused: unused(tx, paths)?,
    };
    let stored = to_json(&report)?;
    let changed = tx
        .execute(
            "UPDATE job SET experience_recalled = ?1,
                            updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?2 AND status = 'created'",
            params![stored, job.as_str()],
        )
        .map_err(StoreError::from)?;
    if changed == 1 {
        return Ok((report, stored));
    }
    report.recorded = false;
    let json = to_json(&report)?;
    Ok((report, json))
}

fn to_json(report: &Report) -> Result<String, RecallError> {
    serde_json::to_string(report).map_err(RecallError::Json)
}
