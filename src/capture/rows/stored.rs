//! Plain reads of what the store holds for one job: the job's time window, its runs
//! and the main session's stored figure. Stored times also come back as milliseconds.

use std::time::{Duration, SystemTime};

use rusqlite::{Connection, OptionalExtension, Row};

use super::integer::unsigned;
use super::readings::Judged;
use crate::capture::tokens::{Reading, TokenCounts, TokenTotal};
use crate::store::JobId;

/// The facts of a job that capture uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredJob {
    pub(crate) session_id: String,
    pub(crate) created_at: String,
    /// The `at` of the job's first transition to a status that is not open.
    pub(crate) closed_at: Option<String>,
    pub(crate) closed_ms: Option<u64>,
    pub(crate) builder_model: Option<String>,
    pub(crate) builder_effort: Option<String>,
}

/// One row of `sub_agent_run`, with its times also in milliseconds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredRun {
    pub(crate) id: i64,
    pub(crate) agent_id: String,
    pub(crate) parent_agent_id: Option<String>,
    pub(crate) depth: Option<u32>,
    pub(crate) role: Option<String>,
    pub(crate) tag_job: Option<String>,
    pub(crate) agent_type: Option<String>,
    pub(crate) model_ran: Option<String>,
    pub(crate) effort_ran: Option<String>,
    pub(crate) brief: Option<String>,
    /// Joined by commas, as the store keeps a list.
    pub(crate) skills_opened: Option<String>,
    pub(crate) background: Option<bool>,
    pub(crate) started_at: Option<String>,
    pub(crate) started_ms: Option<u64>,
    pub(crate) stopped_at: Option<String>,
    pub(crate) stopped_ms: Option<u64>,
    pub(crate) duration_ms: Option<u64>,
    pub(crate) tool_uses: Option<u64>,
    pub(crate) lines_added: Option<u64>,
    pub(crate) lines_removed: Option<u64>,
    pub(crate) final_message: Option<String>,
    pub(crate) transcript_path: Option<String>,
    pub(crate) total: StoredTotal,
}

/// A stored token total and the reading it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredTotal {
    /// `None` while no reading was stored.
    pub(crate) counts: Option<TokenCounts>,
    pub(crate) is_final: bool,
    pub(crate) bytes: Option<u64>,
    pub(crate) damaged_lines: Option<u64>,
    pub(crate) read_at: Option<String>,
    pub(crate) read_ms: Option<u64>,
}

impl StoredTotal {
    pub(crate) fn total(&self) -> TokenTotal {
        match (self.counts, self.is_final) {
            (None, _) => TokenTotal::Unknown,
            (Some(counts), false) => TokenTotal::Provisional(counts),
            (Some(counts), true) => TokenTotal::Final(counts),
        }
    }

    /// The stored reading, as the rule for a final total needs it.
    pub(crate) fn reading(&self) -> Option<Reading> {
        self.counts?;
        Some(Reading {
            transcript_bytes: self.bytes?,
            damaged_lines: self.damaged_lines.unwrap_or(0),
            read_at: SystemTime::UNIX_EPOCH + Duration::from_millis(self.read_ms?),
        })
    }

    /// The stored reading as a step to final names it, so that the step changes
    /// nothing IF a newer reading was stored in the meantime.
    pub(crate) fn judged(&self) -> Option<Judged> {
        Some(Judged {
            bytes: self.bytes?,
            read_at: self.read_at.clone()?,
        })
    }

    pub(crate) fn has_damaged_lines(&self) -> bool {
        self.damaged_lines.is_some_and(|damaged| damaged > 0)
    }
}

/// The job `job`, IF the store has it.
pub(crate) fn job(conn: &Connection, job: &JobId) -> rusqlite::Result<Option<StoredJob>> {
    conn.query_row(
        "SELECT session_id, created_at, closed_at,
                CAST(ROUND((julianday(closed_at) - 2440587.5) * 86400000) AS INTEGER),
                builder_model, builder_effort
         FROM (SELECT session_id, created_at, builder_model, builder_effort,
                      (SELECT MIN(at) FROM job_transition
                       WHERE job_transition.job_id = job.id
                         AND to_status NOT IN ('created', 'planned')) AS closed_at
               FROM job WHERE id = ?1)",
        [job.as_str()],
        |row| {
            Ok(StoredJob {
                session_id: row.get(0)?,
                created_at: row.get(1)?,
                closed_at: row.get(2)?,
                closed_ms: unsigned(row, 3)?,
                builder_model: row.get(4)?,
                builder_effort: row.get(5)?,
            })
        },
    )
    .optional()
}

/// The runs of `job`, by `started_at`, then `agent_id`.
pub(crate) fn runs(conn: &Connection, job: &JobId) -> rusqlite::Result<Vec<StoredRun>> {
    let mut select = conn.prepare(
        "SELECT id, agent_id, parent_agent_id, depth, role, tag_job, agent_type, model_ran,
                effort_ran, brief, skills_opened, background,
                started_at, CAST(ROUND((julianday(started_at) - 2440587.5) * 86400000) AS INTEGER),
                stopped_at, CAST(ROUND((julianday(stopped_at) - 2440587.5) * 86400000) AS INTEGER),
                duration_ms, tool_uses, lines_added, lines_removed, final_message, transcript_path,
                tokens_input, tokens_output, tokens_cache_read, tokens_cache_creation,
                tokens_provisional, transcript_bytes, transcript_damaged_lines, tokens_read_at,
                CAST(ROUND((julianday(tokens_read_at) - 2440587.5) * 86400000) AS INTEGER)
         FROM sub_agent_run WHERE job_id = ?1
         ORDER BY started_at, agent_id",
    )?;
    let runs = select.query_map([job.as_str()], |row| {
        Ok(StoredRun {
            id: row.get(0)?,
            agent_id: row.get(1)?,
            parent_agent_id: row.get(2)?,
            depth: row.get(3)?,
            role: row.get(4)?,
            tag_job: row.get(5)?,
            agent_type: row.get(6)?,
            model_ran: row.get(7)?,
            effort_ran: row.get(8)?,
            brief: row.get(9)?,
            skills_opened: row.get(10)?,
            background: row.get(11)?,
            started_at: row.get(12)?,
            started_ms: unsigned(row, 13)?,
            stopped_at: row.get(14)?,
            stopped_ms: unsigned(row, 15)?,
            duration_ms: unsigned(row, 16)?,
            tool_uses: unsigned(row, 17)?,
            lines_added: unsigned(row, 18)?,
            lines_removed: unsigned(row, 19)?,
            final_message: row.get(20)?,
            transcript_path: row.get(21)?,
            total: total_from(row, 22)?,
        })
    })?;
    runs.collect()
}

/// The main session's stored figure for `job`. `None` = no row.
pub(crate) fn session_total(
    conn: &Connection,
    job: &JobId,
) -> rusqlite::Result<Option<StoredTotal>> {
    conn.query_row(
        "SELECT tokens_input, tokens_output, tokens_cache_read, tokens_cache_creation,
                tokens_provisional, transcript_bytes, transcript_damaged_lines, tokens_read_at,
                CAST(ROUND((julianday(tokens_read_at) - 2440587.5) * 86400000) AS INTEGER)
         FROM session_tokens WHERE job_id = ?1",
        [job.as_str()],
        |row| total_from(row, 0),
    )
    .optional()
}

/// The nine token columns that start at `first`: the four counts, the provisional
/// flag, the bytes, the damaged lines, the read time and that time in milliseconds.
fn total_from(row: &Row<'_>, first: usize) -> rusqlite::Result<StoredTotal> {
    let counts = [
        unsigned(row, first)?,
        unsigned(row, first + 1)?,
        unsigned(row, first + 2)?,
        unsigned(row, first + 3)?,
    ];
    let is_provisional: bool = row.get(first + 4)?;
    let [input, output, cache_read, cache_creation] = counts;
    let counts = counts.iter().any(Option::is_some).then(|| TokenCounts {
        input: input.unwrap_or(0),
        output: output.unwrap_or(0),
        cache_read: cache_read.unwrap_or(0),
        cache_creation: cache_creation.unwrap_or(0),
    });
    Ok(StoredTotal {
        counts,
        is_final: !is_provisional,
        bytes: unsigned(row, first + 5)?,
        damaged_lines: unsigned(row, first + 6)?,
        read_at: row.get(first + 7)?,
        read_ms: unsigned(row, first + 8)?,
    })
}
