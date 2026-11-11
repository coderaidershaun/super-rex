//! The one place where a job's stored status changes after it is created, so
//! that every command that moves a job follows the same table.

use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ValueRef};
use rusqlite::{OptionalExtension, Transaction, params};

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
}

/// The statuses that `rex job close` can give.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Closed {
    Delivered,
    Blocked,
    Failed,
    Cancelled,
}

/// The two feedback verdicts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Verdict {
    Accepted,
    Rejected,
}

/// What happened to a job. Creating a job is not an event: `rex job create`
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
    "{text} is not a stored job status; one of created, planned, blocked, failed, delivered, accepted, rejected, analyzed, cancelled"
)]
pub struct StatusError {
    text: String,
}

/// Moves the job by the transition table and writes one history row per step.
/// A close always writes a row, even `blocked` to `blocked`. A feedback that
/// leaves the status writes nothing. Run it inside the caller's
/// [`crate::store::Store::write`], so the caller's own rows and the status
/// change are one transaction.
///
/// # Errors
/// - [`StoreError::JobNotFound`] if there is no such job.
/// - [`StoreError::Refused`] for a plan or close the table does not allow.
/// - [`StoreError::NoOutcomeYet`] for feedback on a `created` or `planned` job.
/// - [`StoreError::AnalysisDue`] for feedback on a `rejected` job that gives no analysis.
/// - [`StoreError::Sqlite`] if a read or write fails.
///
/// A refused event writes nothing.
///
/// `rex job close --status delivered` while a delivery is blocked is refused by
/// that command, not here, because the check reads the deliveries.
pub fn apply(tx: &Transaction<'_>, job: &JobId, event: Event) -> Result<Change, StoreError> {
    let previous = stored_status(tx, job)?;
    let steps = steps(job, previous, event)?;
    let Some(&current) = steps.last() else {
        return Ok(Change {
            previous,
            current: previous,
        });
    };

    // One time for the whole call, so two rows of one feedback share it.
    let now: String = tx.query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ', 'now')", [], |row| {
        row.get(0)
    })?;
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
        // instead of replacing its parent's plan.
        Event::Plan => match previous {
            Created => Ok(vec![Planned]),
            _ => Err(refused("rex job plan", Accepts::CreatedJob)),
        },
        Event::Close(closed) => match (previous, closed) {
            (Created, Closed::Delivered) => Err(refused(
                "rex job close --status delivered",
                Accepts::PlannedOrBlockedJob,
            )),
            (Created | Planned | Blocked, closed) => Ok(vec![closed.into()]),
            _ => Err(refused("rex job close", Accepts::OpenOrBlockedJob)),
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
            // records the rest; the other three already say how the work ended.
            (Blocked | Failed | Cancelled | Analyzed, _, _) => Ok(vec![]),
        },
    }
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
    /// [`StatusError`] unless `text` is one of the nine lower-case names.
    fn from_str(text: &str) -> Result<Status, StatusError> {
        use Status::*;
        [
            Created, Planned, Blocked, Failed, Delivered, Accepted, Rejected, Analyzed, Cancelled,
        ]
        .into_iter()
        .find(|status| status.as_str() == text)
        .ok_or_else(|| StatusError {
            text: text.to_owned(),
        })
    }
}

/// A stored text that is not one of the nine statuses fails the read of its column.
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
