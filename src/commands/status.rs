//! `rex status`: where a job stands, kept short because agents read it.

use std::cmp::max;
use std::fmt::Write as _;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::Connection;

use crate::capture::{CaptureError, Cost, JobRuns, Run, RunState, TokenTotal, job_runs};
use crate::commands::gate::SpendLimit;
use crate::commands::stdout::print_line_to_stdout;
use crate::commands::token_text::{cost_text, split_text};
use crate::error::{CommandError, RexError};
use crate::store::status::{OPEN_STATUSES_SQL, Shown, Status};
use crate::store::{JobId, Store, StoreError, StorePaths};

/// Print where a job stands: open sub-agents and what has finished.
#[derive(clap::Args)]
#[command(
    after_help = "Exits 0 also when no job is open, with the line `no open job`. Tokens are `in` (input), `cache-w` (cache writes), `cache-r` (cache reads) and `out` (output); `hit` is the share of prompt tokens read from the cache; the dollar figure is their cost at list prices, `≥` when some requests ran on a model with no price. A `~` at the end means the counts can still grow. From 80% of the spend limit of a job that is created or planned, a `warning:` line follows the header (see `rex limit`)."
)]
pub struct StatusOptions {
    /// The job to show. Without it, every job that is created or planned.
    pub job: Option<JobId>,
}

#[derive(thiserror::Error, Debug)]
pub enum StatusError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Capture(#[from] CaptureError),
    #[error(transparent)]
    Rex(#[from] RexError),
}

impl CommandError for StatusError {}

/// Prints the runs of one job, or of every job that is created or planned.
///
/// # Errors
/// - [`StoreError::NotFound`] if the repo has no store.
/// - [`StoreError::JobNotFound`] if the job given is not in the store.
/// - [`CaptureError`] if the event log or the store cannot be read.
/// - [`RexError::Stdout`] if the output cannot be written.
pub fn run(cwd: &Path, options: StatusOptions) -> Result<(), StatusError> {
    let paths = StorePaths::discover(cwd)?;
    let mut store = Store::open(&paths)?;
    let now = SystemTime::now();

    let jobs = match options.job {
        Some(job) => vec![job],
        None => open_jobs(store.conn()).map_err(StoreError::from)?,
    };
    if jobs.is_empty() {
        print_line_to_stdout("no open job")?;
    }

    let mut damaged_log_lines = 0;
    for job in &jobs {
        let runs = job_runs(&mut store, job, now)?;
        let stored = stored_status(store.conn(), job).map_err(StoreError::from)?;
        for line in job_lines(&store, now, job, stored, &runs)? {
            print_line_to_stdout(&line)?;
        }
        damaged_log_lines = runs.damaged_log_lines;
    }
    // The count is for the whole log, so it is the same for every job.
    if damaged_log_lines > 0 {
        print_line_to_stdout(&format!("event log: {damaged_log_lines} damaged lines"))?;
    }
    Ok(())
}

/// The jobs that are created or planned, oldest first.
fn open_jobs(conn: &Connection) -> rusqlite::Result<Vec<JobId>> {
    let mut query = conn.prepare(&format!(
        "SELECT id FROM job WHERE status IN {OPEN_STATUSES_SQL} ORDER BY created_at, id"
    ))?;
    query.query_map([], |row| row.get(0))?.collect()
}

fn stored_status(conn: &Connection, job: &JobId) -> rusqlite::Result<Status> {
    conn.query_row(
        "SELECT status FROM job WHERE id = ?1",
        [job.as_str()],
        |row| row.get(0),
    )
}

/// The header of a job, the warning near its spend limit, and its sections, one
/// line each.
fn job_lines(
    store: &Store,
    now: SystemTime,
    job: &JobId,
    stored: Status,
    runs: &JobRuns,
) -> Result<Vec<String>, StatusError> {
    let role_width = column_width(runs, |run| role_text(run));
    let type_width = column_width(runs, |run| agent_type_text(run));
    let now_ms = millis(now);
    let mut lines = vec![header(job, stored, runs)];
    // The gate checks only an open job, so a closed one gets no warning.
    if stored.is_open() {
        let spent = runs.cost.unwrap_or_default();
        if let Some(warning) = SpendLimit::of(store, job)?.warning(job, &spent) {
            lines.push(warning);
        }
    }
    for (state, name) in [
        (RunState::Open, "open"),
        (RunState::Finished, "finished"),
        (RunState::Unconfirmed, "unconfirmed"),
    ] {
        let in_section: Vec<&Run> = runs.runs.iter().filter(|run| run.state == state).collect();
        if in_section.is_empty() {
            continue;
        }
        lines.push(name.to_owned());
        for run in in_section {
            let time = match state {
                RunState::Open => elapsed_since_start(store.conn(), now_ms, run)
                    .map_err(StoreError::from)?
                    .map_or_else(|| "-".to_owned(), duration_text),
                RunState::Finished => run
                    .duration_ms
                    .map_or_else(|| "-".to_owned(), duration_text),
                RunState::Unconfirmed => match runs.last_active_ms.get(&run.agent_id) {
                    Some(at) => format!(
                        "no stop event, quiet {}",
                        duration_text(now_ms.saturating_sub(*at))
                    ),
                    None => "no stop event".to_owned(),
                },
            };
            lines.push(run_line(run, &time, role_width, type_width));
        }
    }
    Ok(lines)
}

/// The job's ID and stored status, `(running)` when it applies, the three counts,
/// `incomplete` when the flag is set, and the job's tokens (runs and session).
fn header(job: &JobId, stored: Status, runs: &JobRuns) -> String {
    let mut header = format!("job {job}  {stored}");
    if Shown::of(stored, runs.open) == Shown::Running {
        header.push_str(" (running)");
    }
    let _ = write!(
        header,
        "  {} open  {} finished  {} unconfirmed",
        runs.open, runs.finished, runs.unconfirmed
    );
    if runs.incomplete {
        header.push_str("  incomplete");
    }
    if let Some(tokens) = tokens_text(runs.tokens, runs.cost) {
        let _ = write!(header, "  {tokens}");
    }
    header
}

fn column_width(runs: &JobRuns, text: impl Fn(&Run) -> &str) -> usize {
    runs.runs
        .iter()
        .fold(0, |width, run| max(width, text(run).len()))
}

fn role_text(run: &Run) -> &str {
    run.role.as_deref().unwrap_or("-")
}

fn agent_type_text(run: &Run) -> &str {
    run.agent_type.as_deref().unwrap_or("-")
}

/// Milliseconds from the run's start to `now_ms`. `None` when the start is not known.
///
/// SQLite turns the stored time text into milliseconds, so Rust parses no dates.
fn elapsed_since_start(conn: &Connection, now_ms: u64, run: &Run) -> rusqlite::Result<Option<u64>> {
    let Some(started_at) = &run.started_at else {
        return Ok(None);
    };
    let now_ms = i64::try_from(now_ms).unwrap_or(i64::MAX);
    let elapsed: Option<i64> = conn.query_row(
        "SELECT ?1 - CAST(ROUND((julianday(?2) - 2440587.5) * 86400000) AS INTEGER)",
        rusqlite::params![now_ms, started_at],
        |row| row.get(0),
    )?;
    // A start after `now` (clock skew) shows as no time passed.
    Ok(elapsed.map(|ms| u64::try_from(ms).unwrap_or(0)))
}

fn millis(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH).map_or(0, |since_epoch| {
        u64::try_from(since_epoch.as_millis()).unwrap_or(u64::MAX)
    })
}

/// One run: the id, role, agent type, `time`, then `bg`, `<parent` and tokens where they apply.
fn run_line(run: &Run, time: &str, role_width: usize, type_width: usize) -> String {
    let mut line = format!(
        "  {}  {:role_width$}  {:type_width$}  {time}",
        run.agent_id,
        role_text(run),
        agent_type_text(run),
    );
    if run.background == Some(true) {
        line.push_str("  bg");
    }
    if let Some(parent) = &run.parent_agent_id {
        let _ = write!(line, "  <{parent}");
    }
    if let Some(tokens) = tokens_text(run.tokens, run.cost) {
        let _ = write!(line, "  {tokens}");
    }
    line
}

/// `41s`, `2m05s` or `1h02m`.
fn duration_text(ms: u64) -> String {
    let seconds = ms / 1000;
    match seconds {
        0..60 => format!("{seconds}s"),
        60..3600 => format!("{}m{:02}s", seconds / 60, seconds % 60),
        _ => format!("{}h{:02}m", seconds / 3600, seconds % 3600 / 60),
    }
}

/// `in 30  cache-w 78.4k  cache-r 762.6k  out 13.4k  hit 90%  $0.42`, with `~` at
/// the end while the total may still grow. `None` when the total is not known.
fn tokens_text(total: TokenTotal, cost: Option<Cost>) -> Option<String> {
    let (counts, mark) = match total {
        TokenTotal::Unknown => return None,
        TokenTotal::Provisional(counts) => (counts, "~"),
        TokenTotal::Final(counts) => (counts, ""),
    };
    let cost = cost.map_or_else(String::new, |cost| format!("  {}", cost_text(&cost)));
    Some(format!("{}{cost}{mark}", split_text(&counts)))
}
