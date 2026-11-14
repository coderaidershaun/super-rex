//! The cost of record of sub-agent runs: reads the transcripts of the runs that
//! `rex ingest` recorded, and finds the runs whose start never reached it.

mod error;
mod price;
mod read;
mod recovery;
mod rows;
mod runs;
mod tags;
mod tokens;
mod transcripts;

use std::path::Path;
use std::time::SystemTime;

use crate::store::{JobId, Store};

pub use error::CaptureError;
pub use price::Cost;
pub(crate) use rows::remember_transcript as remember_main_transcript;

/// Where Claude Code writes the transcript of `agent_id`, a sub-agent of
/// `session_id`, beside the session's main transcript: known before the run's
/// end names it, so a run whose end is lost is still read. `None` while the
/// session's main transcript is not known.
///
/// # Errors
/// The read of the session's main transcript from the store fails.
pub(crate) fn derived_transcript(
    conn: &rusqlite::Connection,
    session_id: &str,
    agent_id: &str,
) -> rusqlite::Result<Option<String>> {
    let Some(main) = rows::session_transcript(conn, session_id)? else {
        return Ok(None);
    };
    let path = recovery::subagents_folder(std::path::Path::new(&main), session_id)
        .map(|folder| folder.join(recovery::transcript_name(agent_id)));
    Ok(path.and_then(|path| path.to_str().map(str::to_owned)))
}
pub use runs::{JobRuns, MainFacts, Run, RunState};
pub use tokens::{Reading, RunActivity, SETTLE, Step, TokenCounts, TokenTotal, TotalCheck};
pub use transcripts::{TranscriptFacts, read_transcript};

/// Finds runs that left no event, reads transcripts, stores totals, and
/// returns the runs of one job as they stand at `now`. A transcript,
/// metadata file or folder that cannot be read counts as absent and sets
/// [`JobRuns::incomplete`].
///
/// # Errors
/// - [`CaptureError::Store`] when the store fails, and with
///   [`crate::store::StoreError::JobNotFound`] for a job not in the store.
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
    transcripts::read_main_transcript(path, job, from, to)
}
