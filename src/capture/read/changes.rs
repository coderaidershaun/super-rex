//! Everything one read stores, in one transaction and in the guarded forms, so a
//! reading from a second `rex` at the same moment never undoes a final total.

use rusqlite::Transaction;

use crate::capture::error::CaptureError;
use crate::capture::rows::{self, Builder, Fill, Judged, Reading, RunReading};

/// The writes of one read, by run ID.
#[derive(Debug, Default)]
pub(super) struct Changes {
    pub(super) fills: Vec<(i64, Fill)>,
    pub(super) readings: Vec<(i64, RunReading)>,
    pub(super) settled: Vec<(i64, Judged)>,
    pub(super) session: Option<SessionChange>,
}

/// What a read stores for the main session.
#[derive(Debug)]
pub(super) enum SessionChange {
    Store { reading: Reading, builder: Builder },
    Settle(Judged),
}

impl Changes {
    pub(super) fn is_empty(&self) -> bool {
        self.fills.is_empty()
            && self.readings.is_empty()
            && self.settled.is_empty()
            && self.session.is_none()
    }

    pub(super) fn apply(&self, tx: &Transaction<'_>, job: &str) -> Result<(), CaptureError> {
        for (run_id, fill) in &self.fills {
            rows::fill_run(tx, *run_id, fill)?;
        }
        for (run_id, reading) in &self.readings {
            rows::store_run_reading(tx, *run_id, reading)?;
        }
        for (run_id, judged) in &self.settled {
            rows::settle_run(tx, *run_id, judged)?;
        }
        match &self.session {
            Some(SessionChange::Store { reading, builder }) => {
                rows::store_session_reading(tx, job, reading)?;
                rows::fill_builder(tx, job, builder)?;
            }
            Some(SessionChange::Settle(judged)) => rows::settle_session(tx, job, judged)?,
            None => {}
        }
        Ok(())
    }
}
