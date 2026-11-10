//! The main session's own tokens inside the job's time. They follow the rule for a
//! run that is not open, with one step more, because the main transcript keeps
//! growing after the job closes.

use std::path::PathBuf;

use rusqlite::Connection;

use super::Read;
use super::changes::SessionChange;
use crate::capture::error::CaptureError;
use crate::capture::main_transcript::read_main_transcript;
use crate::capture::rows::{self, Builder, Reading, StoredTotal};
use crate::capture::runs::MainFacts;
use crate::capture::tokens::{RunActivity, SETTLE, Step, TotalCheck};

impl Read<'_> {
    /// What this read stores for the main session. `None` when the figure is final,
    /// when the session logged no line (its transcript is not known), or when
    /// nothing changed.
    pub(super) fn session_change(
        &mut self,
        conn: &Connection,
        stored: Option<&StoredTotal>,
    ) -> Result<Option<SessionChange>, CaptureError> {
        if stored.is_some_and(|stored| stored.is_final) {
            return Ok(None);
        }
        let Some(main) = rows::session_transcript(conn, &self.stored_job.session_id)? else {
            return Ok(None);
        };
        let main = PathBuf::from(main);
        let (bytes, _) = self.size_and_time(&main);
        let check = TotalCheck {
            stored: stored.and_then(StoredTotal::reading),
            transcript_bytes: bytes,
            closed_at: self.closed_at(),
            run: RunActivity::NotOpen,
            now: self.now,
        };
        match check.next_step() {
            Step::Keep => return Ok(None),
            Step::Final => {
                return Ok(stored
                    .and_then(StoredTotal::judged)
                    .map(SessionChange::Settle));
            }
            Step::ReadAgain => {}
        }
        let job = self.stored_job;
        let facts = match read_main_transcript(
            &main,
            self.job,
            &job.created_at,
            job.closed_at.as_deref(),
        ) {
            Ok(Some(facts)) => facts,
            Ok(None) => return Ok(None),
            Err(_) => {
                self.has_unreadable_file = true;
                return Ok(None);
            }
        };
        if let Some(judged) = stored
            .filter(|stored| self.is_unchanged_after_close(stored, bytes, &facts))
            .and_then(StoredTotal::judged)
        {
            return Ok(Some(SessionChange::Settle(judged)));
        }
        Ok(Some(self.store_change(facts)))
    }

    /// The new reading, and the builder's model and effort where the job has none yet.
    fn store_change(&self, facts: MainFacts) -> SessionChange {
        let job = self.stored_job;
        SessionChange::Store {
            reading: Reading {
                tokens: facts.tokens,
                bytes: facts.bytes,
                damaged_lines: facts.damaged_lines,
                read_ms: self.now_ms,
            },
            builder: Builder {
                model: facts.builder_model.filter(|_| job.builder_model.is_none()),
                effort: facts
                    .builder_effort
                    .filter(|_| job.builder_effort.is_none()),
                now_ms: self.now_ms,
            },
        }
    }

    /// The one step more: after the close the transcript grew, but the requests inside
    /// the job's time give the stored figure again, that figure was read after the
    /// close at least [`SETTLE`] ago, and the parse found no damaged line.
    fn is_unchanged_after_close(
        &self,
        stored: &StoredTotal,
        bytes_now: Option<u64>,
        facts: &MainFacts,
    ) -> bool {
        let (Some(closed_at), Some(reading)) = (self.closed_at(), stored.reading()) else {
            return false;
        };
        let has_grown = Some(reading.transcript_bytes) != bytes_now;
        let is_settled = reading.read_at >= closed_at
            && self
                .now
                .duration_since(reading.read_at)
                .is_ok_and(|age| age >= SETTLE);
        has_grown && is_settled && facts.damaged_lines == 0 && stored.counts == Some(facts.tokens)
    }
}
