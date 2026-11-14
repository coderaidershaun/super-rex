//! One batch of events from the plugin, read and checked against the batch
//! schema before anything is written, so a batch that does not fit changes nothing.

use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::data::Data;
use super::error::IngestError;
use crate::store::JobId;

/// The most events one batch may hold.
const MAX_EVENTS: usize = 500;

/// A batch as it arrives.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawBatch {
    version: u64,
    session: String,
    stream: String,
    secret: String,
    events: Vec<RawEvent>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEvent {
    seq: u64,
    at: String,
    kind: String,
    agent: String,
    job: Option<String>,
    data: Value,
}

/// A checked batch.
#[derive(Debug)]
pub(super) struct Batch {
    pub(super) session: String,
    pub(super) stream: String,
    pub(super) secret: String,
    pub(super) events: Vec<Event>,
}

/// A checked event: its fields as sent, `data` both as sent and read into its kind.
#[derive(Debug)]
pub(super) struct Event {
    pub(super) seq: u64,
    pub(super) at: String,
    pub(super) agent: String,
    pub(super) job: Option<JobId>,
    pub(super) sent_data: Value,
    pub(super) data: Data,
}

impl Event {
    pub(super) fn kind(&self) -> &'static str {
        self.data.kind()
    }

    /// The job of a job event, which the batch check made sure is there.
    pub(super) fn job(&self) -> Result<&JobId, IngestError> {
        self.job
            .as_ref()
            .ok_or_else(|| self.does_not_fit("it names no job"))
    }

    pub(super) fn does_not_fit(&self, reason: impl Into<String>) -> IngestError {
        IngestError::DoesNotFit {
            seq: self.seq,
            kind: self.kind(),
            reason: reason.into(),
        }
    }
}

impl Batch {
    /// Reads and checks `input`: the batch schema's form, the closed list of
    /// kinds, the `data` keys of each kind, and the form of each text.
    ///
    /// # Errors
    /// [`IngestError::Malformed`], naming the first thing that does not fit.
    pub(super) fn parse(input: &[u8]) -> Result<Batch, IngestError> {
        let raw: RawBatch = serde_json::from_slice(input).map_err(malformed)?;
        if raw.version != 1 {
            return Err(malformed(format!("version must be 1, got {}", raw.version)));
        }
        if raw.session.is_empty() {
            return Err(malformed("session is empty"));
        }
        if !is_hex(&raw.stream, 16) {
            return Err(malformed(format!(
                "stream must be 16 hex digits, got {:?}",
                raw.stream
            )));
        }
        if !is_hex(&raw.secret, 64) {
            return Err(malformed("secret must be 64 hex digits"));
        }
        if raw.events.len() > MAX_EVENTS {
            return Err(malformed(format!(
                "a batch holds at most {MAX_EVENTS} events, got {}",
                raw.events.len()
            )));
        }
        let mut events = Vec::with_capacity(raw.events.len());
        let mut previous = 0;
        for event in raw.events {
            if event.seq <= previous {
                return Err(malformed(format!(
                    "seq must rise within a batch, got {} after {previous}",
                    event.seq
                )));
            }
            previous = event.seq;
            events.push(Event::check(event)?);
        }
        Ok(Batch {
            session: raw.session,
            stream: raw.stream,
            secret: raw.secret,
            events,
        })
    }
}

impl Event {
    fn check(raw: RawEvent) -> Result<Event, IngestError> {
        let seq = raw.seq;
        let at_fault = |reason: String| malformed(format!("seq {seq} {}: {reason}", raw.kind));
        if seq == 0 {
            return Err(malformed("seq starts at 1, got 0"));
        }
        if !is_store_time(&raw.at) {
            return Err(at_fault(format!(
                "at must be YYYY-MM-DDTHH:MM:SS.sssZ, got {:?}",
                raw.at
            )));
        }
        let job = raw
            .job
            .as_deref()
            .map(str::parse::<JobId>)
            .transpose()
            .map_err(|error| at_fault(error.to_string()))?;
        let data = Data::read(&raw.kind, &raw.data).map_err(at_fault)?;
        if data.needs_job() && job.is_none() {
            return Err(at_fault("job is null".to_owned()));
        }
        Ok(Event {
            seq,
            at: raw.at,
            agent: raw.agent,
            job,
            sent_data: raw.data,
            data,
        })
    }
}

/// `data` read as the type of its kind. A key it does not know, or one it
/// lacks, is the text of the error.
pub(super) fn read_data<T: DeserializeOwned>(data: &Value) -> Result<T, String> {
    T::deserialize(data).map_err(|error| error.to_string())
}

fn malformed(reason: impl ToString) -> IngestError {
    IngestError::Malformed {
        reason: reason.to_string(),
    }
}

fn is_hex(text: &str, length: usize) -> bool {
    text.len() == length
        && text
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

/// `YYYY-MM-DDTHH:MM:SS.sssZ`, the one form of time in the store.
fn is_store_time(text: &str) -> bool {
    let bytes = text.as_bytes();
    let form = b"0000-00-00T00:00:00.000Z";
    bytes.len() == form.len()
        && bytes.iter().zip(form).all(|(byte, shape)| match shape {
            b'0' => byte.is_ascii_digit(),
            other => byte == other,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_time_must_have_the_store_form() {
        assert!(is_store_time("2026-11-13T20:00:00.000Z"));
        assert!(!is_store_time("2026-11-13T20:00:00Z"));
        assert!(!is_store_time("2026-11-13 20:00:00.000Z"));
        assert!(!is_store_time("2026-11-13T20:00:00.000+00:00"));
    }
}
