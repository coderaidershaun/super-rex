//! The runs of one job as they stand at one moment. Files are read outside any
//! transaction, and what changed is stored in one short write, so the write lock is
//! never held while a transcript is parsed.

mod changes;
mod files;
mod result;
mod session;
mod state;

use std::time::{Duration, SystemTime};

use super::error::CaptureError;
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
        let standing = read.standing(&runs, &files);
        return read.result(store.conn(), &runs, standing);
    }

    let recovery = recovery::recover(store, &stored_job.session_id)?;
    read.has_unreadable_file |= recovery.has_unreadable_file;
    if recovery.added > 0 {
        runs = rows::runs(store.conn(), job)?;
    }
    let files = read.observe(&runs, Need::LaterSources);
    let standing = read.standing(&runs, &files);
    let mut changes = read.run_changes(&runs, &files, &standing.states);
    changes.session = read.session_change(store.conn(), session.as_ref())?;
    if !changes.is_empty() {
        store.write(|tx| changes.apply(tx, job.as_str()))?;
    }
    let runs = rows::runs(store.conn(), job)?;
    read.result(store.conn(), &runs, standing)
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
