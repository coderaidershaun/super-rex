//! The row of a run that recovery found: a sub-agent transcript and its metadata
//! file with no event of the run. It fills only what the run's row lacks.

use rusqlite::{Connection, named_params};

/// A run that recovery found: a metadata file and a transcript, and no line.
pub(crate) struct RecoveredRun<'a> {
    pub(crate) job_id: &'a str,
    pub(crate) session_id: &'a str,
    pub(crate) agent_id: &'a str,
    pub(crate) transcript: &'a str,
    /// The `timestamp` of the transcript's first record: the run's tie time.
    pub(crate) first_at: &'a str,
    /// The time of this recovery, the same for every row it makes.
    pub(crate) seen_at: &'a str,
}

pub(crate) fn upsert_recovered(conn: &Connection, run: &RecoveredRun<'_>) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO sub_agent_run (job_id, session_id, agent_id, first_seen_at, started_at,
                                    transcript_path)
         VALUES (:job, :session, :agent, :seen_at,
                 strftime('%Y-%m-%dT%H:%M:%fZ', :first_at), :transcript)
         ON CONFLICT(session_id, agent_id) DO UPDATE SET
             started_at = COALESCE(started_at, excluded.started_at),
             transcript_path = COALESCE(transcript_path, excluded.transcript_path)",
        named_params! {
            ":job": run.job_id, ":session": run.session_id, ":agent": run.agent_id,
            ":first_at": run.first_at, ":transcript": run.transcript, ":seen_at": run.seen_at,
        },
    )?;
    Ok(())
}
