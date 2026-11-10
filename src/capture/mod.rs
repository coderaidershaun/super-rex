//! Turns the events that `rex hook` logs into the record of sub-agent runs.

mod error;
mod runs;
mod tokens;
mod transcript;

use std::path::Path;
use std::time::SystemTime;

use crate::store::{JobId, Store};

pub use error::CaptureError;
pub use runs::{JobRuns, MainFacts, Run, RunState};
pub use tokens::{Reading, RunActivity, SETTLE, Step, TokenCounts, TokenTotal, TotalCheck};
pub use transcript::{TranscriptFacts, read_transcript};

/// Takes the new lines of the event log into the store. It does nothing yet.
///
/// # Errors
/// - [`CaptureError::Store`] when the store fails.
/// - [`CaptureError::ReadLog`] when the event log exists and cannot be read.
pub fn take_in(_store: &mut Store) -> Result<(), CaptureError> {
    Ok(())
}

/// Returns the runs of one job as they stand at `now`. It does nothing yet, and
/// always returns no runs.
///
/// # Errors
/// - [`CaptureError::Store`] when the store fails, or the job is not in the store.
/// - [`CaptureError::ReadLog`] when the event log exists and cannot be read.
pub fn job_runs(
    _store: &mut Store,
    _job: &JobId,
    _now: SystemTime,
) -> Result<JobRuns, CaptureError> {
    Ok(JobRuns {
        runs: Vec::new(),
        open: 0,
        finished: 0,
        unconfirmed: 0,
        tokens: TokenTotal::Unknown,
        session_tokens: TokenTotal::Unknown,
        incomplete: false,
        damaged_log_lines: 0,
    })
}

/// Reads the main session's transcript: the tokens of the requests in `[from, to)`,
/// and the model and effort of the `rex job create` call that printed the job.
/// It does nothing yet, and always returns `None`.
///
/// # Errors
/// - [`CaptureError::ReadTranscript`] when the file exists and cannot be read.
pub fn read_main_transcript(
    _path: &Path,
    _job: &JobId,
    _from: &str,
    _to: Option<&str>,
) -> Result<Option<MainFacts>, CaptureError> {
    Ok(None)
}
