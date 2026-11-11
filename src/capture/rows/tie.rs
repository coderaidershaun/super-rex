//! The tie of a run to its job: the job of its session that was open when the run
//! started. A run that has a row keeps the job it has.

use rusqlite::{Connection, OptionalExtension, named_params};

use super::stored::closed_at_sql;
use crate::store::integer::signed_or_null;

/// A time as a log line or a transcript gives it: milliseconds from the hook, or the
/// `timestamp` text of a transcript record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TieTime {
    Ms(u64),
    Text(String),
}

impl TieTime {
    pub(super) fn ms(&self) -> Option<u64> {
        match self {
            TieTime::Ms(ms) => Some(*ms),
            TieTime::Text(_) => None,
        }
    }

    pub(super) fn text(&self) -> Option<&str> {
        match self {
            TieTime::Ms(_) => None,
            TieTime::Text(text) => Some(text),
        }
    }
}

/// The job a run is tied to, IF the run has a row.
pub(crate) fn run_job(
    conn: &Connection,
    session_id: &str,
    agent_id: &str,
) -> rusqlite::Result<Option<String>> {
    conn.query_row(
        "SELECT job_id FROM sub_agent_run WHERE session_id = ?1 AND agent_id = ?2",
        [session_id, agent_id],
        |row| row.get(0),
    )
    .optional()
}

/// The agents of a session that have a row, in any job.
pub(crate) fn session_agents(conn: &Connection, session_id: &str) -> rusqlite::Result<Vec<String>> {
    let mut select = conn.prepare("SELECT agent_id FROM sub_agent_run WHERE session_id = ?1")?;
    let agents = select.query_map([session_id], |row| row.get(0))?;
    agents.collect()
}

pub(crate) fn session_has_job(conn: &Connection, session_id: &str) -> rusqlite::Result<bool> {
    conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM job WHERE session_id = ?1)",
        [session_id],
        |row| row.get(0),
    )
}

/// The job of the session that was open at `at`: created at or before it, and not
/// closed at or before it. A job that is still open has no close time.
pub(crate) fn job_open_at(
    conn: &Connection,
    session_id: &str,
    at: &TieTime,
) -> rusqlite::Result<Option<String>> {
    let closed_at = closed_at_sql();
    conn.query_row(
        &format!(
            "WITH tie (at) AS (
                 SELECT COALESCE(strftime('%Y-%m-%dT%H:%M:%fZ', :ms / 1000.0, 'unixepoch'),
                                 strftime('%Y-%m-%dT%H:%M:%fZ', :text)))
             SELECT job.id FROM job, tie
             WHERE job.session_id = :session
               AND job.created_at <= tie.at
               AND COALESCE({closed_at} > tie.at, 1)
             ORDER BY job.created_at DESC
             LIMIT 1"
        ),
        named_params! {
            ":ms": signed_or_null(at.ms()), ":text": at.text(), ":session": session_id,
        },
        |row| row.get(0),
    )
    .optional()
}
