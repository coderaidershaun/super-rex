//! The tie of a recovered run to its job: the job of its session that was open
//! when the run started. A run that has a row keeps the job it has.

use rusqlite::{Connection, OptionalExtension, named_params};

use super::stored::closed_at_sql;

/// The agents of a session that have a row, in any job.
pub(crate) fn session_agents(conn: &Connection, session_id: &str) -> rusqlite::Result<Vec<String>> {
    let mut select = conn.prepare("SELECT agent_id FROM sub_agent_run WHERE session_id = ?1")?;
    let agents = select.query_map([session_id], |row| row.get(0))?;
    agents.collect()
}

/// The job of the session that was open at `at`, a transcript's `timestamp`:
/// created at or before it, and not closed at or before it. A job that is still
/// open has no close time.
pub(crate) fn job_open_at(
    conn: &Connection,
    session_id: &str,
    at: &str,
) -> rusqlite::Result<Option<String>> {
    let closed_at = closed_at_sql();
    conn.query_row(
        &format!(
            "WITH tie (at) AS (SELECT strftime('%Y-%m-%dT%H:%M:%fZ', :text))
             SELECT job.id FROM job, tie
             WHERE job.session_id = :session
               AND job.created_at <= tie.at
               AND COALESCE({closed_at} > tie.at, 1)
             ORDER BY job.created_at DESC
             LIMIT 1"
        ),
        named_params! {
            ":text": at, ":session": session_id,
        },
        |row| row.get(0),
    )
    .optional()
}
