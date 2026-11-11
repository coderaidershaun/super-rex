//! One upsert for each kind of line that names a run. Each column is written by its
//! first source (`COALESCE(excluded.x, x)`) and filled by a later one only while it
//! is NULL (`COALESCE(x, excluded.x)`), so the stored run is the same whatever the
//! order of its lines, and a line taken in twice changes nothing. The update lists
//! never hold `job_id` or `first_seen_at`: a run keeps the job it was first tied to.
//!
//! A teammate of an agent team has many start lines and stop lines, because it stops
//! each time it goes idle and starts again when a message wakes it. Start lines give
//! `started_at` the earliest of them and `last_started_at` the newest; stop lines give
//! `stopped_at` and `last_stopped_at` the newest. These are the same in any order too.

use rusqlite::{Connection, named_params};

use super::tie::TieTime;
use crate::store::integer::{signed, signed_or_null};

/// What every line that names a run brings, besides its own columns.
pub(crate) struct RunLine<'a> {
    /// The run's job when it has a row, else the job open at its tie time.
    pub(crate) job_id: &'a str,
    pub(crate) session_id: &'a str,
    pub(crate) agent_id: &'a str,
    /// The time of this intake, the same for every row it makes.
    pub(crate) seen_at: &'a str,
    pub(crate) at_ms: u64,
    /// The tie time of a run that has no row yet, kept as its start.
    pub(crate) started_estimate: Option<&'a TieTime>,
    /// Where the run's transcript is, worked out from the main transcript's path.
    pub(crate) derived_transcript: Option<&'a str>,
}

impl RunLine<'_> {
    fn start_ms(&self) -> Option<i64> {
        signed_or_null(self.started_estimate.and_then(TieTime::ms))
    }

    fn start_text(&self) -> Option<&str> {
        self.started_estimate.and_then(TieTime::text)
    }
}

/// A `SubagentStart` line: the first source of `agent_type` and `started_at`, and
/// the only source of `last_started_at`. The first start line replaces a tie-time
/// estimate in `started_at`; each later one keeps the earliest start.
pub(crate) fn upsert_start(
    conn: &Connection,
    line: &RunLine<'_>,
    agent_type: Option<&str>,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO sub_agent_run (job_id, session_id, agent_id, first_seen_at, agent_type,
                                    started_at, last_started_at, transcript_path)
         VALUES (:job, :session, :agent, :seen_at, :agent_type,
                 strftime('%Y-%m-%dT%H:%M:%fZ', :at_ms / 1000.0, 'unixepoch'),
                 strftime('%Y-%m-%dT%H:%M:%fZ', :at_ms / 1000.0, 'unixepoch'), :derived)
         ON CONFLICT(session_id, agent_id) DO UPDATE SET
             agent_type = COALESCE(excluded.agent_type, agent_type),
             started_at = CASE WHEN last_started_at IS NULL THEN excluded.started_at
                               ELSE MIN(COALESCE(started_at, excluded.started_at),
                                        excluded.started_at) END,
             last_started_at = MAX(COALESCE(last_started_at, excluded.last_started_at),
                                   excluded.last_started_at),
             transcript_path = COALESCE(transcript_path, excluded.transcript_path)",
        named_params! {
            ":job": line.job_id, ":session": line.session_id, ":agent": line.agent_id,
            ":seen_at": line.seen_at, ":agent_type": agent_type, ":at_ms": signed(line.at_ms),
            ":derived": line.derived_transcript,
        },
    )?;
    Ok(())
}

/// The columns a `SubagentStop` line gives.
pub(crate) struct StopColumns<'a> {
    pub(crate) agent_type: Option<&'a str>,
    pub(crate) effort: Option<&'a str>,
    pub(crate) final_message: Option<&'a str>,
    pub(crate) transcript_path: Option<&'a str>,
}

/// A `SubagentStop` line: the first source of `effort_ran`, `final_message`,
/// `transcript_path` and `stopped_at`, and the only source of `last_stopped_at`. For
/// `agent_type` the start line ranks first, so that the order of the two lines cannot
/// matter. The first stop line replaces a stop time from the call; each later one
/// keeps the newest stop.
pub(crate) fn upsert_stop(
    conn: &Connection,
    line: &RunLine<'_>,
    stop: &StopColumns<'_>,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO sub_agent_run (job_id, session_id, agent_id, first_seen_at, agent_type,
                                    effort_ran, final_message, transcript_path, stopped_at,
                                    last_stopped_at, started_at)
         VALUES (:job, :session, :agent, :seen_at, :agent_type, :effort, :final_message,
                 COALESCE(:transcript, :derived),
                 strftime('%Y-%m-%dT%H:%M:%fZ', :at_ms / 1000.0, 'unixepoch'),
                 strftime('%Y-%m-%dT%H:%M:%fZ', :at_ms / 1000.0, 'unixepoch'),
                 COALESCE(strftime('%Y-%m-%dT%H:%M:%fZ', :start_ms / 1000.0, 'unixepoch'),
                          strftime('%Y-%m-%dT%H:%M:%fZ', :start_text)))
         ON CONFLICT(session_id, agent_id) DO UPDATE SET
             agent_type = COALESCE(agent_type, excluded.agent_type),
             effort_ran = COALESCE(excluded.effort_ran, effort_ran),
             final_message = COALESCE(excluded.final_message, final_message),
             transcript_path = COALESCE(:transcript, transcript_path, :derived),
             stopped_at = CASE WHEN last_stopped_at IS NULL THEN excluded.stopped_at
                               ELSE MAX(COALESCE(stopped_at, excluded.stopped_at),
                                        excluded.stopped_at) END,
             last_stopped_at = MAX(COALESCE(last_stopped_at, excluded.last_stopped_at),
                                   excluded.last_stopped_at),
             started_at = COALESCE(started_at, excluded.started_at)",
        named_params! {
            ":job": line.job_id, ":session": line.session_id, ":agent": line.agent_id,
            ":seen_at": line.seen_at, ":agent_type": stop.agent_type, ":effort": stop.effort,
            ":final_message": stop.final_message, ":transcript": stop.transcript_path,
            ":derived": line.derived_transcript, ":at_ms": signed(line.at_ms),
            ":start_ms": line.start_ms(), ":start_text": line.start_text(),
        },
    )?;
    Ok(())
}

/// The columns a call of the sub-agent tool gives. A launched or failed call leaves
/// the columns of a returned call `None`.
pub(crate) struct CallColumns<'a> {
    pub(crate) parent_agent_id: Option<&'a str>,
    pub(crate) brief: Option<&'a str>,
    pub(crate) role: Option<&'a str>,
    pub(crate) tag_job: Option<&'a str>,
    pub(crate) model_ran: Option<&'a str>,
    pub(crate) duration_ms: Option<u64>,
    pub(crate) tool_uses: Option<u64>,
    pub(crate) lines_added: Option<u64>,
    pub(crate) lines_removed: Option<u64>,
    pub(crate) final_message: Option<&'a str>,
    pub(crate) background: Option<bool>,
    pub(crate) stopped_at_ms: Option<u64>,
}

/// A `PostToolUse` line of the sub-agent tool: the first source of the brief, its
/// tags and what the returned call reports; a later source of `final_message` and
/// `stopped_at`, whose first source is the stop line. `tag_job` is kept only when
/// it differs from the run's own job.
pub(crate) fn upsert_call(
    conn: &Connection,
    line: &RunLine<'_>,
    call: &CallColumns<'_>,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO sub_agent_run (job_id, session_id, agent_id, first_seen_at, parent_agent_id,
                                    brief, role, tag_job, model_ran, duration_ms, tool_uses,
                                    lines_added, lines_removed, final_message, background,
                                    stopped_at, started_at, transcript_path)
         VALUES (:job, :session, :agent, :seen_at, :parent, :brief, :role, NULLIF(:tag_job, :job),
                 :model, :duration_ms, :tool_uses, :lines_added, :lines_removed, :final_message,
                 :background,
                 strftime('%Y-%m-%dT%H:%M:%fZ', :stopped_ms / 1000.0, 'unixepoch'),
                 COALESCE(strftime('%Y-%m-%dT%H:%M:%fZ', :start_ms / 1000.0, 'unixepoch'),
                          strftime('%Y-%m-%dT%H:%M:%fZ', :start_text)),
                 :derived)
         ON CONFLICT(session_id, agent_id) DO UPDATE SET
             parent_agent_id = COALESCE(excluded.parent_agent_id, parent_agent_id),
             brief = COALESCE(excluded.brief, brief),
             role = COALESCE(excluded.role, role),
             tag_job = COALESCE(NULLIF(:tag_job, job_id), tag_job),
             model_ran = COALESCE(excluded.model_ran, model_ran),
             duration_ms = COALESCE(excluded.duration_ms, duration_ms),
             tool_uses = COALESCE(excluded.tool_uses, tool_uses),
             lines_added = COALESCE(excluded.lines_added, lines_added),
             lines_removed = COALESCE(excluded.lines_removed, lines_removed),
             final_message = COALESCE(final_message, excluded.final_message),
             background = COALESCE(excluded.background, background),
             stopped_at = COALESCE(stopped_at, excluded.stopped_at),
             started_at = COALESCE(started_at, excluded.started_at),
             transcript_path = COALESCE(transcript_path, excluded.transcript_path)",
        named_params! {
            ":job": line.job_id, ":session": line.session_id, ":agent": line.agent_id,
            ":seen_at": line.seen_at, ":parent": call.parent_agent_id, ":brief": call.brief,
            ":role": call.role, ":tag_job": call.tag_job, ":model": call.model_ran,
            ":duration_ms": signed_or_null(call.duration_ms),
            ":tool_uses": signed_or_null(call.tool_uses),
            ":lines_added": signed_or_null(call.lines_added),
            ":lines_removed": signed_or_null(call.lines_removed),
            ":final_message": call.final_message, ":background": call.background,
            ":stopped_ms": signed_or_null(call.stopped_at_ms),
            ":start_ms": line.start_ms(), ":start_text": line.start_text(),
            ":derived": line.derived_transcript,
        },
    )?;
    Ok(())
}

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
