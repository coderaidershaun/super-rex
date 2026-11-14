//! `rex ingest`'s work: takes one batch of the plugin's events into the store in
//! one transaction, each event once, in sequence, from the stream that knows its secret.

mod batch;
mod data;
mod error;
mod job;
mod record;

pub use error::IngestError;

use std::path::Path;

use rusqlite::{OptionalExtension, Transaction, params};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::store::Store;
use crate::store::integer::signed;
use batch::{Batch, Event};
use data::Data;

/// What applying an event needs besides the event: the write it runs in, the
/// session of its batch, and the library of orchestrator folders.
pub(crate) struct Apply<'a> {
    tx: &'a Transaction<'a>,
    session: &'a str,
    orchestrators: &'a Path,
    describe: &'a dyn Fn(&Path) -> Option<String>,
}

/// Takes the batch in `input` into `store` and gives the session's last
/// sequence number. A stream's first batch stores the hash of its secret; an
/// event the store already holds is skipped; the batch is written in one
/// transaction, or not at all. `describe` reads the description of an
/// orchestrator folder for the orchestrator index.
///
/// # Errors
/// - [`IngestError::Malformed`] for a batch that does not fit the batch schema.
/// - [`IngestError::WrongSecret`] for a stream that knows another secret.
/// - [`IngestError::Gap`] for a sequence number past the next one.
/// - [`IngestError::Conflict`] for a sequence number the store holds with other content.
/// - [`IngestError::DoesNotFit`] for an event that does not fit the record.
/// - [`IngestError::Store`] when the store fails.
pub fn ingest(
    store: &mut Store,
    input: &[u8],
    describe: &dyn Fn(&Path) -> Option<String>,
) -> Result<u64, IngestError> {
    let batch = Batch::parse(input)?;
    let orchestrators = store.paths().orchestrators.clone();
    store.write(|tx| {
        check_secret(tx, &batch)?;
        let mut last = last_sequence(tx, &batch.session)?;
        let apply = Apply {
            tx,
            session: &batch.session,
            orchestrators: &orchestrators,
            describe,
        };
        for event in &batch.events {
            if event.seq <= last {
                check_repeat(tx, &batch.session, event)?;
                continue;
            }
            if event.seq != last + 1 {
                return Err(IngestError::Gap {
                    expected: last + 1,
                    got: event.seq,
                });
            }
            insert_event(tx, &batch, event)?;
            apply.event(event)?;
            last = event.seq;
        }
        Ok(last)
    })
}

/// Stores the hash of the secret at a stream's first batch, and refuses a
/// later batch of the stream that brings another.
fn check_secret(tx: &Transaction<'_>, batch: &Batch) -> Result<(), IngestError> {
    let sent = sha256_hex(&batch.secret);
    let kept: Option<String> = tx
        .query_row(
            "SELECT secret_sha256 FROM ingest_stream WHERE session_id = ?1 AND stream = ?2",
            params![batch.session, batch.stream],
            |row| row.get(0),
        )
        .optional()
        .map_err(crate::store::StoreError::from)?;
    match kept {
        Some(kept) if kept == sent => Ok(()),
        Some(_) => Err(IngestError::WrongSecret {
            session: batch.session.clone(),
            stream: batch.stream.clone(),
        }),
        None => {
            tx.execute(
                "INSERT INTO ingest_stream (session_id, stream, secret_sha256, opened_at)
                 VALUES (?1, ?2, ?3, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
                params![batch.session, batch.stream, sent],
            )
            .map_err(crate::store::StoreError::from)?;
            Ok(())
        }
    }
}

fn last_sequence(tx: &Transaction<'_>, session: &str) -> Result<u64, IngestError> {
    let last: i64 = tx
        .query_row(
            "SELECT COALESCE(MAX(seq), 0) FROM ingest_event WHERE session_id = ?1",
            [session],
            |row| row.get(0),
        )
        .map_err(crate::store::StoreError::from)?;
    Ok(u64::try_from(last).unwrap_or_default())
}

/// An event sent again must be the one the store holds, from whichever stream:
/// after a new stream the plugin sends again what it could not confirm.
fn check_repeat(tx: &Transaction<'_>, session: &str, event: &Event) -> Result<(), IngestError> {
    let kept: Option<(String, String, String, Option<String>, String)> = tx
        .query_row(
            "SELECT kind, at, agent, job_id, data FROM ingest_event WHERE session_id = ?1 AND seq = ?2",
            params![session, signed(event.seq)],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )
        .optional()
        .map_err(crate::store::StoreError::from)?;
    let conflict = || IngestError::Conflict { seq: event.seq };
    let Some((kind, at, agent, job, data)) = kept else {
        return Err(conflict());
    };
    let kept_data: Value = serde_json::from_str(&data).map_err(|_| conflict())?;
    let is_same = kind == event.kind()
        && at == event.at
        && agent == event.agent
        && job.as_deref() == event.job.as_ref().map(|job| job.as_str())
        && kept_data == event.sent_data;
    if is_same { Ok(()) } else { Err(conflict()) }
}

fn insert_event(tx: &Transaction<'_>, batch: &Batch, event: &Event) -> Result<(), IngestError> {
    tx.execute(
        "INSERT INTO ingest_event (session_id, seq, stream, kind, at, agent, job_id, data)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            batch.session,
            signed(event.seq),
            batch.stream,
            event.kind(),
            event.at,
            event.agent,
            event.job.as_ref().map(|job| job.as_str()),
            event.sent_data.to_string(),
        ],
    )
    .map_err(crate::store::StoreError::from)?;
    Ok(())
}

fn sha256_hex(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

impl Apply<'_> {
    /// Writes what `event` changes in the tables of the record. An event that
    /// changes no table, such as a prompt, is kept in the event row alone.
    fn event(&self, event: &Event) -> Result<(), IngestError> {
        match &event.data {
            Data::SessionStart(data) => self.session_start(data),
            Data::JobOpen(data) => self.job_open(event, data),
            Data::JobPlan(data) => self.job_plan(event, data),
            Data::JobAmend(data) => self.job_amend(event, data),
            Data::JobClose(data) => self.job_close(event, data),
            Data::JobMergeAnswer(data) => self.job_merge_answer(event, data),
            Data::JobLimit(data) => self.job_limit(event, data),
            Data::JobDegraded(data) => self.job_degraded(event, data),
            Data::JobRebind(data) => self.job_rebind(event, data),
            Data::Decision(data) => self.decision(event, data),
            Data::RunStart(data) => self.run_start(event, data),
            Data::RunMark(data) => self.run_mark(data),
            Data::RunEnd(data) => self.run_end(event, data),
            Data::SkillOpen(data) => self.skill_open(event, data),
            Data::QuestionAsk(data) => self.question_ask(event, data),
            Data::QuestionAnswer(data) => self.question_answer(event, data),
            Data::TreeOpen(data) => self.tree_open(event, data),
            Data::TreeLink(data) => self.tree_link(event, data),
            Data::TreeState(data) => self.tree_state(data.tree.as_str(), data.state.as_str(), None),
            Data::TreeClose(data) => self.tree_close(event, data),
            Data::Prompt
            | Data::JobDoneCheck
            | Data::JobWake
            | Data::TreeNote
            | Data::MergeDone => Ok(()),
        }
    }
}
