//! The one place the job commands ask capture for the sub-agent runs, so that
//! only this file changes when capture's own calls arrive.
//!
//! SMELL: capture does not yet offer `take_in` and `job_runs`. Until it does,
//! `take_in` here does nothing and `job_runs` gives a job with no runs, so the
//! shown status is never `running`. Replace both bodies with calls to capture's
//! functions, and these types with capture's own, when capture offers them.

use std::time::SystemTime;

use crate::capture::{CaptureError, TokenTotal};
use crate::store::{JobId, Store};

/// The runs of one job as they stand now, named as capture names them.
pub(super) struct JobRuns {
    pub(super) runs: Vec<Run>,
    pub(super) open: u32,
    pub(super) tokens: TokenTotal,
    pub(super) session_tokens: TokenTotal,
    pub(super) incomplete: bool,
    pub(super) damaged_log_lines: u64,
}

/// One sub-agent run, named as capture names it.
#[allow(dead_code)]
pub(super) struct Run {
    pub(super) agent_id: String,
    pub(super) parent_agent_id: Option<String>,
    pub(super) depth: Option<u32>,
    pub(super) role: Option<String>,
    pub(super) tag_job: Option<String>,
    pub(super) agent_type: Option<String>,
    pub(super) model_ran: Option<String>,
    pub(super) effort_ran: Option<String>,
    pub(super) background: Option<bool>,
    pub(super) state: RunState,
    pub(super) started_at: Option<String>,
    pub(super) stopped_at: Option<String>,
    pub(super) duration_ms: Option<u64>,
    pub(super) tool_uses: Option<u64>,
    pub(super) lines_added: Option<u64>,
    pub(super) lines_removed: Option<u64>,
    pub(super) brief: Option<String>,
    pub(super) final_message: Option<String>,
    pub(super) skills_opened: Vec<String>,
    pub(super) tokens: TokenTotal,
}

/// Whether a run is still going.
#[allow(dead_code)]
#[derive(Clone, Copy)]
pub(super) enum RunState {
    Open,
    Finished,
    Unconfirmed,
}

/// Takes the new lines of the event log into the store.
///
/// # Errors
/// [`CaptureError`] if the event log cannot be read or the store fails.
pub(super) fn take_in(_store: &mut Store) -> Result<(), CaptureError> {
    Ok(())
}

/// Takes in the event log, then gives the runs of `job` as they stand at `now`.
///
/// # Errors
/// [`CaptureError`] if the event log cannot be read or the store fails.
pub(super) fn job_runs(
    _store: &mut Store,
    _job: &JobId,
    _now: SystemTime,
) -> Result<JobRuns, CaptureError> {
    Ok(JobRuns {
        runs: Vec::new(),
        open: 0,
        tokens: TokenTotal::Unknown,
        session_tokens: TokenTotal::Unknown,
        incomplete: false,
        damaged_log_lines: 0,
    })
}

impl RunState {
    pub(super) fn name(self) -> &'static str {
        match self {
            RunState::Open => "open",
            RunState::Finished => "finished",
            RunState::Unconfirmed => "unconfirmed",
        }
    }
}
