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
    pub(super) is_folder_unreadable: bool,
}

/// The `timestamp` of the first record of the transcript at `path`. Only one line is
/// read. `None` when the file is absent, cannot be read, or its first record has no time.
pub(super) fn first_record_at(path: &Path) -> Option<String> {
    let mut first = String::new();
    BufReader::new(File::open(path).ok()?)
        .read_line(&mut first)
        .ok()?;
    let record: Value = serde_json::from_str(&first).ok()?;
    text_at(&record, "timestamp").map(str::to_owned)
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
/// [`CaptureError::Store`] when the store fails. A folder that cannot be listed is
/// reported in [`Recovery::is_folder_unreadable`], not as an error.
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
    let entries = match fs::read_dir(&folder) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(recovery),
        Err(_) => {
            recovery.is_folder_unreadable = true;
            return Ok(recovery);
        }
    };
    let known: HashSet<String> = rows::session_agents(store.conn(), session_id)?
        .into_iter()
        .collect();
    let mut found = Vec::new();
    for entry in entries {
        let Ok(entry) = entry else {
            recovery.is_folder_unreadable = true;
            continue;
        };
        let name = entry.file_name();
        let Some(agent) = name
            .to_str()
            .and_then(|name| name.strip_prefix("agent-"))
            .and_then(|name| name.strip_suffix(".meta.json"))
        else {
            continue;
        };
        if known.contains(agent) {
            continue;
        }
        let transcript = folder.join(format!("agent-{agent}.jsonl"));
        let (Some(first_at), Some(transcript)) =
            (first_record_at(&transcript), transcript.to_str())
        else {
            continue;
        };
        let tie = TieTime::Text(first_at.clone());
        if let Some(job) = rows::job_open_at(store.conn(), session_id, &tie)? {
            found.push((job, agent.to_owned(), transcript.to_owned(), first_at));
        }
    }
    if found.is_empty() {
        return Ok(recovery);
    }
    store.write(|tx| {
        let seen_at = rows::now_text(tx)?;
        for (job, agent, transcript, first_at) in &found {
            let run = RecoveredRun {
                job_id: job,
                session_id,
                agent_id: agent,
                transcript,
                first_at,
                seen_at: &seen_at,
            };
            rows::upsert_recovered(tx, &run)?;
        }
        Ok::<_, CaptureError>(())
    })?;
    recovery.added = found.len();
    Ok(recovery)
}
