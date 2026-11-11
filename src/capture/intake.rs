//! Takes the new lines of the event log into the store in one write, and ties each
//! new run to the job of its session that was open when the run started.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use rusqlite::Connection;

use super::error::CaptureError;
use super::event::{Call, CallOutcome, Completion, Event, EventKind, Stop};
use super::hook::{self, LogLine};
use super::recovery::{first_record_at, subagents_folder, transcript_name};
use super::rows::{self, CallColumns, RunLine, StopColumns, TieTime};
use super::tags::Tags;
use crate::store::Store;

/// Takes in the lines after the saved offset. A log with no whole new line since
/// the last intake takes no lock, so a read with nothing new costs no write.
///
/// # Errors
/// [`CaptureError::Store`] when the store fails; [`CaptureError::ReadLog`] when the
/// log exists and cannot be read.
pub(super) fn take_in(store: &mut Store) -> Result<(), CaptureError> {
    let log_path = store.paths().event_log.clone();
    let offset = rows::offset(store.conn())?;
    if log_size(&log_path)? == offset {
        return Ok(());
    }
    // A hook that is half way through its line makes the log longer and gives
    // nothing whole to take in, so the log is read before the lock is asked for.
    if hook::read_log(&log_path, offset)?.offset == offset {
        return Ok(());
    }
    store.write(|tx| {
        // Read again under the write lock, so two commands at the same moment never
        // take in one line two times.
        let read = hook::read_log(&log_path, rows::offset(tx)?)?;
        let intake = Intake {
            conn: tx,
            seen_at: rows::now_text(tx)?,
        };
        let mut damaged = 0;
        for line in &read.lines {
            match line {
                LogLine::Damaged => damaged += 1,
                LogLine::Raw => {}
                LogLine::Event { at_ms, event } => intake.event(&Event::of(event), *at_ms)?,
            }
        }
        rows::save_intake(tx, read.offset, damaged)?;
        Ok(())
    })
}

/// One intake: the write it runs in, and the time it gives every run it makes.
struct Intake<'a> {
    conn: &'a Connection,
    seen_at: String,
}

/// A line that names a run.
struct Named<'a> {
    session_id: &'a str,
    agent_id: &'a str,
    kind: &'a EventKind,
    at_ms: u64,
}

fn log_size(path: &Path) -> Result<u64, CaptureError> {
    match fs::metadata(path) {
        Ok(metadata) => Ok(metadata.len()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(0),
        Err(source) => Err(CaptureError::ReadLog {
            path: path.to_path_buf(),
            source,
        }),
    }
}

impl Intake<'_> {
    /// Applies one event. The session's main transcript is kept the first time any of
    /// its lines is seen, also a line that names no run.
    fn event(&self, event: &Event, at_ms: u64) -> rusqlite::Result<()> {
        let Some(session_id) = event.session_id.as_deref() else {
            return Ok(());
        };
        if let Some(main) = event.transcript_path.as_deref() {
            rows::remember_transcript(self.conn, session_id, main)?;
        }
        let Some(agent_id) = agent_of(&event.kind) else {
            return Ok(());
        };
        let named = Named {
            session_id,
            agent_id,
            kind: &event.kind,
            at_ms,
        };
        let Some(tie) = self.tie(&named)? else {
            return Ok(());
        };
        let derived = event
            .transcript_path
            .as_deref()
            .and_then(|main| derived_transcript(main, session_id, agent_id));
        let line = RunLine {
            job_id: &tie.job_id,
            session_id,
            agent_id,
            seen_at: &self.seen_at,
            at_ms,
            started_estimate: tie.started_estimate.as_ref(),
            derived_transcript: derived.as_deref(),
        };
        self.upsert(&line, &event.kind)
    }

    /// Writes the columns that `kind` gives to the run that `line` names.
    fn upsert(&self, line: &RunLine<'_>, kind: &EventKind) -> rusqlite::Result<()> {
        match kind {
            EventKind::Start(start) => {
                rows::upsert_start(self.conn, line, start.agent_type.as_deref())
            }
            EventKind::Stop(stop) => {
                let columns = StopColumns {
                    agent_type: stop.agent_type.as_deref(),
                    effort: stop.effort.as_deref(),
                    final_message: stop.last_message.as_deref(),
                    transcript_path: stop.agent_transcript_path.as_deref(),
                };
                rows::upsert_stop(self.conn, line, &columns)
            }
            EventKind::Call(call) => {
                let tags = call
                    .brief
                    .as_deref()
                    .map(Tags::of_brief)
                    .unwrap_or_default();
                rows::upsert_call(self.conn, line, &call_columns(call, &tags, line.at_ms))
            }
            EventKind::Other => Ok(()),
        }
    }

    /// A run with a row keeps its job. A new run gets the job of its session that
    /// was open at its tie time; `None` when there is none, and the line is skipped.
    fn tie(&self, named: &Named<'_>) -> rusqlite::Result<Option<Tie>> {
        if let Some(job_id) = rows::run_job(self.conn, named.session_id, named.agent_id)? {
            return Ok(Some(Tie {
                job_id,
                started_estimate: None,
            }));
        }
        if !rows::session_has_job(self.conn, named.session_id)? {
            return Ok(None);
        }
        let at = tie_time(named.kind, named.at_ms);
        let job = rows::job_open_at(self.conn, named.session_id, &at)?;
        Ok(job.map(|job_id| Tie {
            job_id,
            started_estimate: Some(at),
        }))
    }
}

fn agent_of(kind: &EventKind) -> Option<&str> {
    match kind {
        EventKind::Start(start) => Some(&start.agent_id),
        EventKind::Stop(stop) => Some(&stop.agent_id),
        EventKind::Call(call) => call.agent_id.as_deref(),
        EventKind::Other => None,
    }
}

fn call_columns<'a>(call: &'a Call, tags: &'a Tags, at_ms: u64) -> CallColumns<'a> {
    let asked = CallColumns {
        parent_agent_id: call.caller_agent_id.as_deref(),
        brief: call.brief.as_deref(),
        role: tags.role.as_deref(),
        tag_job: tags.job.as_deref(),
        model_ran: None,
        duration_ms: None,
        tool_uses: None,
        lines_added: None,
        lines_removed: None,
        final_message: None,
        background: None,
        stopped_at_ms: None,
    };
    match &call.outcome {
        CallOutcome::Completed(done) => CallColumns {
            model_ran: done.model.as_deref(),
            duration_ms: done.duration_ms,
            tool_uses: done.tool_uses,
            lines_added: Some(done.lines_added),
            lines_removed: Some(done.lines_removed),
            final_message: done.final_text.as_deref(),
            background: Some(false),
            stopped_at_ms: Some(at_ms),
            ..asked
        },
        CallOutcome::Launched { model } => CallColumns {
            model_ran: model.as_deref(),
            background: Some(true),
            ..asked
        },
        CallOutcome::OtherStatus => asked,
    }
}

/// The job a line's run belongs to, and for a run with no row its tie time.
struct Tie {
    job_id: String,
    started_estimate: Option<TieTime>,
}

/// The best estimate of when a run started, from the first line seen for it. A
/// start line or a launch fires at the start; other lines come from the end of the
/// run, which may be after its job closed. Known limit: with the start line lost and
/// no duration or transcript to go back from, the line's own time is used, and that
/// can tie the run to a later job.
fn tie_time(kind: &EventKind, at_ms: u64) -> TieTime {
    match kind {
        EventKind::Call(Call {
            outcome:
                CallOutcome::Completed(Completion {
                    duration_ms: Some(duration_ms),
                    ..
                }),
            ..
        }) => TieTime::Ms(at_ms.saturating_sub(*duration_ms)),
        EventKind::Stop(Stop {
            agent_transcript_path: Some(path),
            ..
        }) => first_record_at(Path::new(path))
            .ok()
            .flatten()
            .map_or(TieTime::Ms(at_ms), TieTime::Text),
        _ => TieTime::Ms(at_ms),
    }
}

/// Where Claude Code writes a sub-agent's transcript, known before its stop line
/// names it.
fn derived_transcript(main: &str, session_id: &str, agent_id: &str) -> Option<String> {
    let path = subagents_folder(Path::new(main), session_id)?.join(transcript_name(agent_id));
    path.to_str().map(str::to_owned)
}
