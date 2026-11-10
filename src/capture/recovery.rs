//! The files Claude Code writes beside each sub-agent's transcript: a metadata file
//! with facts no event gives, and, for a sub-agent whose events were all lost, the
//! only sign that it ran.

use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, ErrorKind};
use std::path::Path;

use serde_json::Value;

use super::error::CaptureError;
use super::rows::{self, RecoveredRun, TieTime};
use super::transcript::text_at;
use crate::store::Store;

/// What one metadata file says about its run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Metadata {
    pub(super) agent_type: Option<String>,
    pub(super) spawn_depth: Option<u32>,
    /// Absent for a run that the main session started.
    pub(super) parent_agent_id: Option<String>,
    pub(super) background: Option<bool>,
    /// Claude Code adds `finished: true` when the run ends.
    pub(super) is_finished: bool,
}

/// The result of one search for runs that left no line.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Recovery {
    pub(super) added: usize,
    /// The folder, or a transcript in it, exists and could not be read.
    pub(super) has_unreadable_file: bool,
}

/// A run that left no line: its metadata file has no row.
struct Unrecorded {
    job_id: String,
    agent_id: String,
    transcript: String,
    first_at: String,
}

/// The `timestamp` of the first record of the transcript at `path`. Only one line is
/// read. `Ok(None)` when the file is absent, or its first record has no time.
///
/// # Errors
/// The file exists and cannot be read.
pub(super) fn first_record_at(path: &Path) -> io::Result<Option<String>> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let mut first = String::new();
    BufReader::new(file).read_line(&mut first)?;
    let Ok(record) = serde_json::from_str::<Value>(&first) else {
        return Ok(None);
    };
    Ok(text_at(&record, "timestamp").map(str::to_owned))
}

/// The metadata file at `path`. `Ok(None)` when there is no file.
///
/// # Errors
/// The file exists and cannot be read, or is not a JSON object. Claude Code
/// rewrites the file, so a read can also catch it half written.
pub(super) fn read_metadata(path: &Path) -> io::Result<Option<Metadata>> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let file: Value = serde_json::from_slice(&bytes)?;
    if !file.is_object() {
        return Err(io::Error::new(ErrorKind::InvalidData, "not a JSON object"));
    }
    let owned = |key: &str| text_at(&file, key).map(str::to_owned);
    Ok(Some(Metadata {
        agent_type: owned("agentType"),
        spawn_depth: file
            .get("spawnDepth")
            .and_then(Value::as_u64)
            .and_then(|depth| u32::try_from(depth).ok()),
        parent_agent_id: owned("parentAgentId"),
        background: match text_at(&file, "requestShape") {
            Some("background") => Some(true),
            Some("foreground") => Some(false),
            _ => None,
        },
        is_finished: file.get("finished") == Some(&Value::Bool(true)),
    }))
}

/// Gives a run to each metadata file of the session that has no run, IF a job of
/// the session was open at the time of its transcript's first record. Only a
/// session that logged a line is searched, in the folder beside its main transcript.
///
/// # Errors
/// [`CaptureError::Store`] when the store fails. A folder or transcript that cannot
/// be read is reported in [`Recovery::has_unreadable_file`], not as an error.
pub(super) fn recover(store: &mut Store, session_id: &str) -> Result<Recovery, CaptureError> {
    let mut recovery = Recovery::default();
    let Some(main) = rows::session_transcript(store.conn(), session_id)? else {
        return Ok(recovery);
    };
    let Some(folder) = Path::new(&main)
        .parent()
        .map(|parent| parent.join(session_id).join("subagents"))
    else {
        return Ok(recovery);
    };
    let found = unrecorded_runs(store, session_id, &folder, &mut recovery)?;
    if found.is_empty() {
        return Ok(recovery);
    }
    store.write(|tx| {
        let seen_at = rows::now_text(tx)?;
        for run in &found {
            let run = RecoveredRun {
                job_id: &run.job_id,
                session_id,
                agent_id: &run.agent_id,
                transcript: &run.transcript,
                first_at: &run.first_at,
                seen_at: &seen_at,
            };
            rows::upsert_recovered(tx, &run)?;
        }
        Ok::<_, CaptureError>(())
    })?;
    recovery.added = found.len();
    Ok(recovery)
}

/// The runs in `folder` that have a metadata file and no row, each with the job of
/// the session that was open at its transcript's first record. A run with no such
/// job is left out.
// SMELL: a run that started while no job was open never gets a row, so every read of
// the session's jobs opens its transcript again to find that out.
fn unrecorded_runs(
    store: &Store,
    session_id: &str,
    folder: &Path,
    recovery: &mut Recovery,
) -> Result<Vec<Unrecorded>, CaptureError> {
    let entries = match fs::read_dir(folder) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(_) => {
            recovery.has_unreadable_file = true;
            return Ok(Vec::new());
        }
    };
    let known: HashSet<String> = rows::session_agents(store.conn(), session_id)?
        .into_iter()
        .collect();
    let mut found = Vec::new();
    for entry in entries {
        let Ok(entry) = entry else {
            recovery.has_unreadable_file = true;
            continue;
        };
        let name = entry.file_name();
        let Some(agent) = name.to_str().and_then(metadata_agent) else {
            continue;
        };
        if known.contains(agent) {
            continue;
        }
        let transcript = folder.join(format!("agent-{agent}.jsonl"));
        let first_at = first_record_at(&transcript).unwrap_or_else(|_| {
            recovery.has_unreadable_file = true;
            None
        });
        let (Some(first_at), Some(transcript)) = (first_at, transcript.to_str()) else {
            continue;
        };
        let tie = TieTime::Text(first_at.clone());
        if let Some(job_id) = rows::job_open_at(store.conn(), session_id, &tie)? {
            found.push(Unrecorded {
                job_id,
                agent_id: agent.to_owned(),
                transcript: transcript.to_owned(),
                first_at,
            });
        }
    }
    Ok(found)
}

/// The agent ID in the name of a metadata file, `agent-<id>.meta.json`.
fn metadata_agent(name: &str) -> Option<&str> {
    name.strip_prefix("agent-")?.strip_suffix(".meta.json")
}
