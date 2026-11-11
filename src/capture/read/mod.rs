//! The runs of one job as they stand at one moment. Files are read outside any
//! transaction, and what changed is stored in one short write, so the write lock is
//! never held while a transcript is parsed.

mod changes;
mod files;
mod result;
mod session;

use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use super::error::CaptureError;
use super::intake;
use super::recovery;
use super::rows::{self, StoredJob};
use super::runs::JobRuns;
use crate::store::{JobId, Store, StoreError};
use files::Need;

pub(super) fn job_runs(
    store: &mut Store,
    job: &JobId,
    now: SystemTime,
) -> Result<JobRuns, CaptureError> {
    intake::take_in(store)?;
    let stored_job = rows::job(store.conn(), job)?.ok_or_else(|| StoreError::JobNotFound {
        job: job.to_string(),
        store: store.paths().dir.clone(),
    })?;
    let mut read = Read {
        job,
        stored_job: &stored_job,
        now,
        now_ms: millis(now),
        has_unreadable_file: false,
    };
    let mut runs = rows::runs(store.conn(), job)?;
    let session = rows::session_total(store.conn(), job)?;
    let is_all_final = runs.iter().all(|run| run.total.is_final)
        && session.as_ref().is_some_and(|session| session.is_final);
    if is_all_final {
        let files = read.observe(&runs, Need::State);
        let states = read.states(&runs, &files);
        return read.result(store.conn(), &runs, &states);
    }

    let recovery = recovery::recover(store, &stored_job.session_id)?;
    read.has_unreadable_file |= recovery.has_unreadable_file;
    if recovery.added > 0 {
        runs = rows::runs(store.conn(), job)?;
    }
    let files = read.observe(&runs, Need::LaterSources);
    let states = read.states(&runs, &files);
    let mut changes = read.run_changes(&runs, &files, &states);
    changes.session = read.session_change(store.conn(), session.as_ref())?;
    if !changes.is_empty() {
        store.write(|tx| changes.apply(tx, job.as_str()))?;
    }
    let runs = rows::runs(store.conn(), job)?;
    read.result(store.conn(), &runs, &states)
}

/// The last activity of each run of `job`, by agent ID, in milliseconds since the
/// epoch: the newest of the run's start and its transcript's modification time, and
/// of the same for each run below it. Reads no log and writes nothing.
// SMELL: this reads every run's files a second time, after `job_runs` read them, and
// takes the clock only to fill a field it does not use. The read of `job_runs` should
// hand back the activity it already found.
pub(crate) fn last_activity_ms(
    store: &Store,
    job: &JobId,
) -> Result<HashMap<String, u64>, CaptureError> {
    let Some(stored_job) = rows::job(store.conn(), job)? else {
        return Ok(HashMap::new());
    };
    let now = SystemTime::now();
    let mut read = Read {
        job,
        stored_job: &stored_job,
        now,
        now_ms: millis(now),
        has_unreadable_file: false,
    };
    let runs = rows::runs(store.conn(), job)?;
    let files = read.observe(&runs, Need::State);
    Ok(files::last_activities(&runs, &files))
}

/// One call of `job_runs`: the job, the moment of the read, and whether a file of
/// the job could not be read at this read.
struct Read<'a> {
    job: &'a JobId,
    stored_job: &'a StoredJob,
    now: SystemTime,
    now_ms: u64,
    has_unreadable_file: bool,
}

impl Read<'_> {
    fn closed_at(&self) -> Option<SystemTime> {
        self.stored_job
            .closed_ms
            .map(|ms| SystemTime::UNIX_EPOCH + Duration::from_millis(ms))
    }
}

fn millis(time: SystemTime) -> u64 {
    time.duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}
