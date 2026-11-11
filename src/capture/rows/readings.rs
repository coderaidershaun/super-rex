//! What a read of a job's files stores: token readings, the step of a total to
//! final, the columns that a transcript or a metadata file fills while they are
//! NULL, and the builder's model and effort. A reading never undoes a final total,
//! and a step to final that a newer reading has passed changes nothing.

use rusqlite::{Connection, named_params, params};

use super::integer::signed;
use crate::capture::price::Cost;
use crate::capture::tokens::TokenCounts;

/// What one parse of a transcript found, read at `read_ms`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Reading {
    pub(crate) tokens: TokenCounts,
    pub(crate) cost: Cost,
    pub(crate) bytes: u64,
    pub(crate) damaged_lines: u64,
    pub(crate) read_ms: u64,
}

/// A parse of a run's transcript, with the counts that come with each parse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RunReading {
    pub(crate) reading: Reading,
    /// Joined by commas; empty when the run opened no skill.
    pub(crate) skills_opened: String,
    pub(crate) tool_uses: u64,
    pub(crate) lines_added: u64,
    pub(crate) lines_removed: u64,
}

/// The stored reading that a step to final was judged on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Judged {
    pub(crate) bytes: u64,
    pub(crate) read_at: String,
}

/// Stores a parse of a run's transcript, unless its total is already final. The
/// counts replace the stored ones only while no returned call gave them.
pub(crate) fn store_run_reading(
    conn: &Connection,
    run_id: i64,
    run: &RunReading,
) -> rusqlite::Result<()> {
    let reading = &run.reading;
    conn.execute(
        "UPDATE sub_agent_run SET
             tokens_input = :input, tokens_output = :output,
             tokens_cache_read = :cache_read, tokens_cache_creation = :cache_creation,
             cost_nanodollars = :cost, cost_unpriced_requests = :unpriced,
             transcript_bytes = :bytes, transcript_damaged_lines = :damaged,
             tokens_read_at = strftime('%Y-%m-%dT%H:%M:%fZ', :read_ms / 1000.0, 'unixepoch'),
             skills_opened = :skills,
             tool_uses = CASE WHEN duration_ms IS NULL THEN :tool_uses ELSE tool_uses END,
             lines_added = CASE WHEN duration_ms IS NULL THEN :lines_added ELSE lines_added END,
             lines_removed = CASE WHEN duration_ms IS NULL THEN :lines_removed ELSE lines_removed END
         WHERE id = :id AND tokens_provisional = 1",
        named_params! {
            ":input": signed(reading.tokens.input), ":output": signed(reading.tokens.output),
            ":cache_read": signed(reading.tokens.cache_read),
            ":cache_creation": signed(reading.tokens.cache_creation),
            ":cost": signed(reading.cost.nanodollars),
            ":unpriced": signed(reading.cost.unpriced_requests),
            ":bytes": signed(reading.bytes), ":damaged": signed(reading.damaged_lines),
            ":read_ms": signed(reading.read_ms), ":skills": run.skills_opened,
            ":tool_uses": signed(run.tool_uses), ":lines_added": signed(run.lines_added),
            ":lines_removed": signed(run.lines_removed), ":id": run_id,
        },
    )?;
    Ok(())
}

/// Makes a run's total final, IF the stored reading is still the one that was judged.
pub(crate) fn settle_run(conn: &Connection, run_id: i64, judged: &Judged) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE sub_agent_run SET tokens_provisional = 0
         WHERE id = ?1 AND tokens_provisional = 1 AND transcript_bytes = ?2 AND tokens_read_at = ?3",
        params![run_id, signed(judged.bytes), judged.read_at],
    )?;
    Ok(())
}

/// Columns of a run that a later source fills while they are NULL. The metadata
/// file's `spawnDepth` is the first source of `depth`, so it is written as given.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Fill {
    pub(crate) spawn_depth: Option<u32>,
    pub(crate) derived_depth: Option<u32>,
    pub(crate) parent_agent_id: Option<String>,
    pub(crate) agent_type: Option<String>,
    pub(crate) background: Option<bool>,
    /// The `timestamp` text of the transcript's last record.
    pub(crate) stopped_at: Option<String>,
    /// From the first user record of the transcript; the tags are taken only while
    /// the run has no brief from a call of the sub-agent tool.
    pub(crate) brief: Option<String>,
    pub(crate) role: Option<String>,
    pub(crate) tag_job: Option<String>,
    pub(crate) model_ran: Option<String>,
    pub(crate) effort_ran: Option<String>,
    pub(crate) final_message: Option<String>,
}

impl Fill {
    pub(crate) fn is_empty(&self) -> bool {
        *self == Fill::default()
    }
}

pub(crate) fn fill_run(conn: &Connection, run_id: i64, fill: &Fill) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE sub_agent_run SET
             depth = COALESCE(:spawn_depth, depth, :derived_depth),
             parent_agent_id = COALESCE(parent_agent_id, :parent),
             agent_type = COALESCE(agent_type, :agent_type),
             background = COALESCE(background, :background),
             stopped_at = COALESCE(stopped_at, strftime('%Y-%m-%dT%H:%M:%fZ', :stopped_at)),
             role = CASE WHEN brief IS NULL THEN COALESCE(role, :role) ELSE role END,
             tag_job = CASE WHEN brief IS NULL THEN COALESCE(tag_job, NULLIF(:tag_job, job_id))
                            ELSE tag_job END,
             brief = COALESCE(brief, :brief),
             model_ran = COALESCE(model_ran, :model),
             effort_ran = COALESCE(effort_ran, :effort),
             final_message = COALESCE(final_message, :final_message)
         WHERE id = :id",
        named_params! {
            ":spawn_depth": fill.spawn_depth, ":derived_depth": fill.derived_depth,
            ":parent": fill.parent_agent_id, ":agent_type": fill.agent_type,
            ":background": fill.background, ":stopped_at": fill.stopped_at,
            ":role": fill.role, ":tag_job": fill.tag_job, ":brief": fill.brief,
            ":model": fill.model_ran, ":effort": fill.effort_ran,
            ":final_message": fill.final_message, ":id": run_id,
        },
    )?;
    Ok(())
}

/// Stores a parse of the main session's transcript for `job`, unless its figure is
/// already final.
pub(crate) fn store_session_reading(
    conn: &Connection,
    job: &str,
    reading: &Reading,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO session_tokens (job_id, tokens_input, tokens_output, tokens_cache_read,
                                     tokens_cache_creation, cost_nanodollars,
                                     cost_unpriced_requests, transcript_bytes,
                                     transcript_damaged_lines, tokens_read_at)
         VALUES (:job, :input, :output, :cache_read, :cache_creation, :cost, :unpriced,
                 :bytes, :damaged, strftime('%Y-%m-%dT%H:%M:%fZ', :read_ms / 1000.0, 'unixepoch'))
         ON CONFLICT(job_id) DO UPDATE SET
             tokens_input = excluded.tokens_input, tokens_output = excluded.tokens_output,
             tokens_cache_read = excluded.tokens_cache_read,
             tokens_cache_creation = excluded.tokens_cache_creation,
             cost_nanodollars = excluded.cost_nanodollars,
             cost_unpriced_requests = excluded.cost_unpriced_requests,
             transcript_bytes = excluded.transcript_bytes,
             transcript_damaged_lines = excluded.transcript_damaged_lines,
             tokens_read_at = excluded.tokens_read_at
         WHERE tokens_provisional = 1",
        named_params! {
            ":job": job,
            ":input": signed(reading.tokens.input), ":output": signed(reading.tokens.output),
            ":cache_read": signed(reading.tokens.cache_read),
            ":cache_creation": signed(reading.tokens.cache_creation),
            ":cost": signed(reading.cost.nanodollars),
            ":unpriced": signed(reading.cost.unpriced_requests),
            ":bytes": signed(reading.bytes), ":damaged": signed(reading.damaged_lines),
            ":read_ms": signed(reading.read_ms),
        },
    )?;
    Ok(())
}

/// Makes the main session's figure final, IF the stored reading is still the one
/// that was judged.
pub(crate) fn settle_session(
    conn: &Connection,
    job: &str,
    judged: &Judged,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE session_tokens SET tokens_provisional = 0
         WHERE job_id = ?1 AND tokens_provisional = 1 AND transcript_bytes = ?2
           AND tokens_read_at = ?3",
        params![job, signed(judged.bytes), judged.read_at],
    )?;
    Ok(())
}

/// The model and effort that ran the builder, as the main transcript shows them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Builder {
    pub(crate) model: Option<String>,
    pub(crate) effort: Option<String>,
    /// The time of the read, written as the job's `updated_at`.
    pub(crate) now_ms: u64,
}

/// Writes the builder's model and effort, each only while it is NULL, so the flags
/// of `rex job create` come first.
pub(crate) fn fill_builder(
    conn: &Connection,
    job: &str,
    builder: &Builder,
) -> rusqlite::Result<()> {
    if let Some(model) = &builder.model {
        conn.execute(
            "UPDATE job SET builder_model = ?1,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', ?2 / 1000.0, 'unixepoch')
             WHERE id = ?3 AND builder_model IS NULL",
            params![model, signed(builder.now_ms), job],
        )?;
    }
    if let Some(effort) = &builder.effort {
        conn.execute(
            "UPDATE job SET builder_effort = ?1,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', ?2 / 1000.0, 'unixepoch')
             WHERE id = ?3 AND builder_effort IS NULL",
            params![effort, signed(builder.now_ms), job],
        )?;
    }
    Ok(())
}
