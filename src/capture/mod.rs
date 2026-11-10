//! Turns the events that `rex hook` logs into the record of sub-agent runs.

// SMELL: this folder holds 14 modules, more than the 12 that one folder may hold. The
// two transcript readers, and the hook's log with its registration, could each be grouped.
mod error;
mod event;
mod intake;
mod log;
mod main_transcript;
mod read;
mod recovery;
mod registration;
mod rows;
mod runs;
mod state;
mod tags;
mod tokens;
mod transcript;

use std::path::Path;
use std::time::SystemTime;

use crate::store::{JobId, Store};

pub use error::CaptureError;
pub(crate) use log::append_event;
pub(crate) use read::last_activity_ms;
pub use registration::{HOOK_COMMAND, hook_registration};
pub use runs::{JobRuns, MainFacts, Run, RunState};
pub use tokens::{Reading, RunActivity, SETTLE, Step, TokenCounts, TokenTotal, TotalCheck};
pub use transcript::{TranscriptFacts, read_transcript};

/// Takes the new lines of the event log into the store. Reads no transcript, except
/// the first record of one for a run whose start line was lost. A log that has not
/// grown since the last intake takes no lock.
///
/// # Errors
/// - [`CaptureError::Store`] when the store fails.
/// - [`CaptureError::ReadLog`] when the event log exists and cannot be read.
pub fn take_in(store: &mut Store) -> Result<(), CaptureError> {
    intake::take_in(store)
}

/// Calls [`take_in`], finds runs that left no event, reads transcripts, stores
/// totals, and returns the runs of one job as they stand at `now`. A transcript,
/// metadata file or folder that cannot be read counts as absent and sets
/// [`JobRuns::incomplete`].
///
/// # Errors
/// - [`CaptureError::Store`] when the store fails, and with
///   [`crate::store::StoreError::JobNotFound`] for a job not in the store.
/// - [`CaptureError::ReadLog`] when the event log exists and cannot be read.
pub fn job_runs(store: &mut Store, job: &JobId, now: SystemTime) -> Result<JobRuns, CaptureError> {
    read::job_runs(store, job, now)
}

/// Reads the main session's transcript: the tokens of the requests in `[from, to)`,
/// and the model and effort of the `rex job create` call that printed the job.
/// Times are text of the store's form; `to` is `None` while the job is open. Returns
/// `Ok(None)` when there is no file.
///
/// # Errors
/// - [`CaptureError::ReadTranscript`] when the file exists and cannot be read.
pub fn read_main_transcript(
    path: &Path,
    job: &JobId,
    from: &str,
    to: Option<&str>,
) -> Result<Option<MainFacts>, CaptureError> {
    main_transcript::read_main_transcript(path, job, from, to)
}
