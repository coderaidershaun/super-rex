//! The one place where a job's stored status changes after it is created, and
//! where an open job is defined, so that every command follows the same table.

use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ValueRef};
use rusqlite::{Connection, OptionalExtension, Transaction, params};

use super::error::{Accepts, StoreError};
use super::job_id::JobId;

/// A status that is stored. `running` is not one: see [`Shown`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Status {
    Created,
    Planned,
    /// Closed, and waiting on something outside the session, such as a
    /// person's approval. No status leads back to an open job, and new
    /// sub-agents no longer join a closed one, so a question to the user in the
    /// same conversation leaves the job open instead.
    Blocked,
    Failed,
    Delivered,
    Accepted,
    Rejected,
    Analyzed,
    Cancelled,
    /// Closed because its session ended before the job did and nothing came back.
    Lost,
}

/// The statuses of an open job, as the list of an SQL `IN`. An open job still
/// takes new sub-agents, and an owner has at most one, which the unique index
/// of the schema enforces with the same list.
pub(crate) const OPEN_STATUSES_SQL: &str = "('created', 'planned')";

/// The statuses that closing a job can give.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Closed {
    Delivered,
    Blocked,
    Failed,
    Cancelled,
    Lost,
}

/// The two feedback verdicts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Verdict {
    Accepted,
    Rejected,
}

/// What happened to a job. Opening a job is not an event: `rex ingest`
/// inserts the row and its first history row itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Event {
    Plan,
    Close(Closed),
    /// `has_analysis` is true when the feedback gives at least one cause and a lesson.
    Feedback {
        verdict: Verdict,
        has_analysis: bool,
    },
}

/// The stored status before and after an event. Both are the same when the
/// status stays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Change {
    pub previous: Status,
    pub current: Status,
}

/// The status a person or agent sees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shown {
    Stored(Status),
    Running,
}

/// Text that is not one of the nine stored statuses.
#[derive(thiserror::Error, Debug)]
#[error(
    "{text} is not a stored job status; one of created, planned, blocked, failed, delivered, accepted, rejected, analyzed, cancelled, lost"
)]
pub struct StatusError {
    text: String,
}

/// Moves the job by the transition table, now, and writes one history row per
/// step. See [`apply_at`].
///
/// # Errors
/// As [`apply_at`].
pub fn apply(tx: &Transaction<'_>, job: &JobId, event: Event) -> Result<Change, StoreError> {
    let now: String = tx.query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ', 'now')", [], |row| {
        row.get(0)
    })?;
    apply_at(tx, job, event, &now)
}

/// Moves the job by the transition table and writes one history row per step,
/// all at `at`, a time in the store's form. A close of an open or blocked job
/// always writes a row, even `blocked` to `blocked`; a close that names the
/// status the job was closed with already writes nothing, so a close sent twice
/// is one. A feedback that leaves the status writes nothing. Run it inside the
/// caller's [`crate::store::Store::write`], so the caller's own rows and the
/// status change are one transaction.
///
/// # Errors
/// - [`StoreError::JobNotFound`] if there is no such job.
/// - [`StoreError::Refused`] for a plan or close the table does not allow.
/// - [`StoreError::NoOutcomeYet`] for feedback on a `created` or `planned` job.
/// - [`StoreError::AnalysisDue`] for feedback on a `rejected` job that gives no analysis.
/// - [`StoreError::Sqlite`] if a read or write fails.
///
/// A refused event writes nothing.
pub fn apply_at(
    tx: &Transaction<'_>,
    job: &JobId,
    event: Event,
    at: &str,
) -> Result<Change, StoreError> {
    let previous = stored_status(tx, job)?;
    let steps = steps(job, previous, event)?;
    let Some(&current) = steps.last() else {
        return Ok(Change {
            previous,
            current: previous,
        });
    };

    let now = at;
    let mut from = previous;
    for &to in &steps {
        tx.execute(
            "INSERT INTO job_transition (job_id, from_status, to_status, event, at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![job.as_str(), from.as_str(), to.as_str(), event.name(), now],
        )?;
        from = to;
    }
    tx.execute(
        "UPDATE job SET status = ?1, updated_at = ?2 WHERE id = ?3",
        params![current.as_str(), now, job.as_str()],
    )?;
    Ok(Change { previous, current })
}

/// The statuses the job steps through, in order. Empty when the status stays
/// and nothing is written.
fn steps(job: &JobId, previous: Status, event: Event) -> Result<Vec<Status>, StoreError> {
    use Status::*;
    let refused = |event: &'static str, accepts: Accepts| StoreError::Refused {
        job: job.to_string(),
        status: previous.to_string(),
        event,
        accepts,
    };
    match event {
        // A job is planned once, so a nested builder that plans by mistake fails
        // instead of replacing its parent's plan. A change of plan is an amendment.
        Event::Plan => match previous {
            Created => Ok(vec![Planned]),
            _ => Err(refused("job.plan", Accepts::CreatedJob)),
        },
        Event::Close(closed) => match (previous, closed) {
            (Created | Planned | Blocked, closed) => Ok(vec![closed.into()]),
            (previous, closed) if previous == closed.into() => Ok(vec![]),
            _ => Err(refused("job.close", Accepts::OpenOrBlockedJob)),
        },
        Event::Feedback {
            verdict,
            has_analysis,
        } => match (previous, verdict, has_analysis) {
            (Created | Planned, _, _) => Err(StoreError::NoOutcomeYet {
                job: job.to_string(),
                status: previous.to_string(),
            }),
            (Delivered, Verdict::Accepted, _) => Ok(vec![Accepted]),
            (Accepted, Verdict::Accepted, _) => Ok(vec![]),
            (Delivered | Accepted, Verdict::Rejected, false) => Ok(vec![Rejected]),
            (Delivered | Accepted, Verdict::Rejected, true) => Ok(vec![Rejected, Analyzed]),
            (Rejected, Verdict::Rejected, true) => Ok(vec![Analyzed]),
            (Rejected, _, _) => Err(StoreError::AnalysisDue {
                job: job.to_string(),
            }),
            // Feedback cannot move a blocked job, so its later close still
            // records the rest; the others already say how the work ended.
            (Blocked | Failed | Cancelled | Analyzed | Lost, _, _) => Ok(vec![]),
        },
    }
}

/// The open job of the main loop of `session_id` and its status, if it has
/// one. It takes a connection, so it also runs inside a [`crate::store::Store::write`].
///
/// # Errors
/// The read fails, or the stored ID or status is not one rex writes.
pub(crate) fn open_job(
    conn: &Connection,
    session_id: &str,
) -> rusqlite::Result<Option<(JobId, Status)>> {
    conn.query_row(
        &format!(
            "SELECT id, status FROM job
             WHERE session_id = ?1 AND builder_agent_id = 'main' AND status IN {OPEN_STATUSES_SQL}"
        ),
        [session_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .optional()
}

fn stored_status(tx: &Transaction<'_>, job: &JobId) -> Result<Status, StoreError> {
    let status: Option<Status> = tx
        .query_row(
            "SELECT status FROM job WHERE id = ?1",
            [job.as_str()],
            |row| row.get(0),
        )
        .optional()?;
    status.ok_or_else(|| StoreError::JobNotFound {
        job: job.to_string(),
        store: store_dir(tx),
    })
}

/// The `.rex` folder of the open database, for error texts.
fn store_dir(tx: &Transaction<'_>) -> PathBuf {
    let db = tx.path().map(Path::new).unwrap_or(Path::new(""));
    db.parent().unwrap_or(db).to_path_buf()
}

impl Status {
    const ALL: [Status; 10] = [
        Status::Created,
        Status::Planned,
        Status::Blocked,
        Status::Failed,
        Status::Delivered,
        Status::Accepted,
        Status::Rejected,
        Status::Analyzed,
        Status::Cancelled,
        Status::Lost,
    ];

    /// True for a job that is created or planned, and so still open.
    pub fn is_open(self) -> bool {
        matches!(self, Status::Created | Status::Planned)
    }

    fn as_str(self) -> &'static str {
        match self {
            Status::Created => "created",
            Status::Planned => "planned",
            Status::Blocked => "blocked",
            Status::Failed => "failed",
            Status::Delivered => "delivered",
            Status::Accepted => "accepted",
            Status::Rejected => "rejected",
            Status::Analyzed => "analyzed",
            Status::Cancelled => "cancelled",
            Status::Lost => "lost",
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Status {
    type Err = StatusError;

    /// # Errors
    /// [`StatusError`] unless `text` is one of the ten lower-case names.
    fn from_str(text: &str) -> Result<Status, StatusError> {
        Status::ALL
            .into_iter()
            .find(|status| status.as_str() == text)
            .ok_or_else(|| StatusError {
                text: text.to_owned(),
            })
    }
}

/// A stored text that is not one of the ten statuses fails the read of its column.
impl FromSql for Status {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Status> {
        let status = value.as_str()?.parse();
        status.map_err(|error: StatusError| FromSqlError::Other(Box::new(error)))
    }
}

impl Verdict {
    /// The text stored in `feedback.verdict`: `accepted` or `rejected`.
    pub fn as_str(self) -> &'static str {
        match self {
            Verdict::Accepted => "accepted",
            Verdict::Rejected => "rejected",
        }
    }
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<Closed> for Status {
    fn from(closed: Closed) -> Status {
        match closed {
            Closed::Delivered => Status::Delivered,
            Closed::Blocked => Status::Blocked,
            Closed::Failed => Status::Failed,
            Closed::Cancelled => Status::Cancelled,
            Closed::Lost => Status::Lost,
        }
    }
}

impl Event {
    /// The name stored in `job_transition.event`.
    fn name(self) -> &'static str {
        match self {
            Event::Plan => "job plan",
            Event::Close(_) => "job close",
            Event::Feedback { .. } => "feedback",
        }
    }
}

impl Shown {
    /// The one rule for the shown status: `running` when the job is planned
    /// and at least one of its sub-agent runs is open.
    pub fn of(stored: Status, open_runs: u32) -> Shown {
        match stored {
            Status::Planned if open_runs > 0 => Shown::Running,
            _ => Shown::Stored(stored),
        }
    }
}

impl fmt::Display for Shown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Shown::Stored(status) => status.fmt(f),
            Shown::Running => f.write_str("running"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_statuses_are_one_list_in_rust_and_in_the_schema() {
        for status in Status::ALL {
            let is_listed = OPEN_STATUSES_SQL.contains(&format!("'{status}'"));
            assert_eq!(is_listed, status.is_open(), "{status}");
        }
        let one_open_job_index = format!("WHERE status IN {OPEN_STATUSES_SQL} AND");
        assert!(include_str!("schema.sql").contains(&one_open_job_index));
    }
}
