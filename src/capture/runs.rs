//! The values that capture hands to its callers: the runs of one job, and the facts
//! read from a main session's transcript.

use super::{TokenCounts, TokenTotal};

/// The runs of one job as they stand at one moment, with the job's totals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobRuns {
    /// Ordered by `started_at`, then `agent_id`. The tree is in `parent_agent_id`.
    pub runs: Vec<Run>,
    pub open: u32,
    pub finished: u32,
    pub unconfirmed: u32,
    /// Every run plus the main session.
    pub tokens: TokenTotal,
    /// The main session alone, inside the job's time.
    pub session_tokens: TokenTotal,
    /// True when the history of the job may miss facts.
    pub incomplete: bool,
    pub damaged_log_lines: u64,
}

/// One sub-agent, from its start to its stop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub agent_id: String,
    /// `None` means the main session started it.
    pub parent_agent_id: Option<String>,
    /// 1 means the main session started it.
    pub depth: Option<u32>,
    /// From the `Rex-Role` tag.
    pub role: Option<String>,
    /// From the `Rex-Job` tag, only when it differs from the job the run belongs to.
    pub tag_job: Option<String>,
    /// The agent type that ran: a grid agent or a built-in one.
    pub agent_type: Option<String>,
    pub model_ran: Option<String>,
    pub effort_ran: Option<String>,
    pub background: Option<bool>,
    pub state: RunState,
    /// Text of the form `YYYY-MM-DDTHH:MM:SS.sssZ`.
    pub started_at: Option<String>,
    pub stopped_at: Option<String>,
    /// From the completed call, else the stop time minus the start time.
    pub duration_ms: Option<u64>,
    pub tool_uses: Option<u64>,
    pub lines_added: Option<u64>,
    pub lines_removed: Option<u64>,
    pub brief: Option<String>,
    pub final_message: Option<String>,
    /// Sorted. Empty means none seen, or no transcript read.
    pub skills_opened: Vec<String>,
    pub tokens: TokenTotal,
}

/// Where a run stands at the moment of the read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunState {
    Open,
    Finished,
    Unconfirmed,
}

/// What a read of the main session's transcript found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MainFacts {
    /// The requests inside the time window.
    pub tokens: TokenCounts,
    /// Includes records whose `timestamp` is not of the store's form.
    pub damaged_lines: u64,
    pub bytes: u64,
    pub builder_model: Option<String>,
    pub builder_effort: Option<String>,
}
