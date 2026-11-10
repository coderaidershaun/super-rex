//! `rex jobs`: lists jobs for scripts and agents. `select.rs` holds the filter
//! and the SELECTs, `summary.rs` the summary of one job and its two output
//! forms. This file ties them to the store and the run read model. `rex watch`
//! builds its job list with [`load_summaries`], the same function, so the
//! screen and `rex jobs --json` cannot drift apart.

mod select;
mod summary;

use std::path::Path;
use std::time::SystemTime;

use super::stdout::print_line_to_stdout;
use crate::capture::{self, CaptureError, JobRuns};
use crate::error::{CommandError, RexError};
use crate::store::status::{Shown, Status, StatusError};
use crate::store::{JobId, Store, StoreError, StorePaths};

pub use select::{
    DeliveryRow, JobFilter, JobRow, has_feedback, select_deliveries, select_job_rows,
};
pub use summary::{
    DeliverySummary, EXCERPT_CHARS, JobSummary, OrchestratorRef, RunCounts, TokenSummary, jobs_json,
};

/// Arguments of `rex jobs`.
#[derive(clap::Args)]
pub struct JobsOptions {
    /// Print one JSON document instead of one line per job
    #[arg(long)]
    pub json: bool,

    /// Keep jobs whose request contains this text, ignoring ASCII case; repeat to require several
    #[arg(long, value_name = "TEXT")]
    pub search: Vec<String>,

    /// Keep jobs created on or after this UTC date, as YYYY-MM-DD
    #[arg(long, value_name = "DATE")]
    pub since: Option<String>,

    /// Keep jobs created on or before this UTC date, as YYYY-MM-DD
    #[arg(long, value_name = "DATE")]
    pub until: Option<String>,
}

#[derive(thiserror::Error, Debug)]
pub enum JobsError {
    #[error("--{flag} needs a date as YYYY-MM-DD, not {text}")]
    BadDate { flag: &'static str, text: String },

    #[error("cannot write the jobs list as JSON")]
    Json(#[source] serde_json::Error),

    #[error("job {job} has a stored status that rex does not know")]
    StoredStatus {
        job: String,
        #[source]
        source: StatusError,
    },

    #[error("job {job} has an ID that rex does not accept")]
    StoredId {
        job: String,
        #[source]
        source: crate::store::JobIdError,
    },

    #[error(transparent)]
    Store(#[from] StoreError),

    #[error(transparent)]
    Capture(#[from] CaptureError),

    #[error(transparent)]
    Rex(#[from] RexError),
}

impl CommandError for JobsError {
    fn is_usage(&self) -> bool {
        matches!(self, JobsError::BadDate { .. })
    }
}

/// # Errors
/// - [`JobsError::BadDate`] for a bad `--since` or `--until`.
/// - [`JobsError::Store`] if the store is missing or a read fails.
/// - [`JobsError::Capture`] if the runs of a job cannot be read.
/// - [`JobsError::StoredStatus`] if a job holds a status that is not one of the nine.
/// - [`JobsError::Json`], [`JobsError::Rex`] if the output cannot be written.
pub fn run(cwd: &Path, options: JobsOptions) -> Result<(), JobsError> {
    let filter = options.filter()?;
    let paths = StorePaths::discover(cwd)?;
    let mut store = Store::open(&paths)?;
    let loaded = load_summaries(&mut store, &filter, SystemTime::now())?;
    let summaries: Vec<JobSummary> = loaded.into_iter().map(|(summary, _)| summary).collect();

    if options.json {
        print_line_to_stdout(&jobs_json(&summaries)?)?;
    } else {
        for summary in &summaries {
            print_line_to_stdout(&summary.plain_line())?;
        }
    }
    Ok(())
}

/// The jobs that match `filter`, newest first, each with its summary and its
/// runs. This takes in the event log of every job it returns, through
/// `capture::job_runs`; it writes nothing else. `rex jobs` and `rex watch`
/// both call this, so they agree on the set of jobs and on each job's shown
/// status, run counts and token totals.
///
/// # Errors
/// - [`JobsError::Store`] if a read fails.
/// - [`JobsError::Capture`] if the runs of a job cannot be read.
/// - [`JobsError::StoredStatus`] if a job holds a status that is not one of the nine.
pub(crate) fn load_summaries(
    store: &mut Store,
    filter: &JobFilter,
    now: SystemTime,
) -> Result<Vec<(JobSummary, JobRuns)>, JobsError> {
    let rows = select_job_rows(store.conn(), filter)?;
    let mut loaded = Vec::with_capacity(rows.len());
    for row in rows {
        let id: JobId = row.id.parse().map_err(|source| JobsError::StoredId {
            job: row.id.clone(),
            source,
        })?;
        let stored: Status =
            row.stored_status
                .parse()
                .map_err(|source| JobsError::StoredStatus {
                    job: row.id.clone(),
                    source,
                })?;
        let runs = capture::job_runs(store, &id, now)?;
        let deliveries = select_deliveries(store.conn(), &id)?;
        let feedback = has_feedback(store.conn(), &id)?;
        let shown = Shown::of(stored, runs.open).to_string();
        let summary = JobSummary::new(row, deliveries, &runs, feedback, &shown);
        loaded.push((summary, runs));
    }
    Ok(loaded)
}

#[cfg(test)]
mod test_support {
    use rusqlite::Connection;

    use crate::capture::{JobRuns, TokenTotal};

    pub fn store() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../../store/schema.sql"))
            .unwrap();
        conn
    }

    pub fn insert(conn: &Connection, id: &str, created_at: &str, request: &str) {
        conn.execute(
            "INSERT INTO job (id, request, asked_by, class_version, work_type, size, consequence, \
             languages, merge_agreed, session_id, status, created_at, updated_at) \
             VALUES (?1, ?2, 'human', 1, 'fix', 'small', 'low', 'go,rust', 0, ?1, 'created', ?3, ?3)",
            (id, request, created_at),
        )
        .unwrap();
    }

    pub fn empty_runs() -> JobRuns {
        JobRuns {
            runs: vec![],
            open: 0,
            finished: 0,
            unconfirmed: 0,
            tokens: TokenTotal::Unknown,
            session_tokens: TokenTotal::Unknown,
            incomplete: false,
            damaged_log_lines: 0,
        }
    }
}
