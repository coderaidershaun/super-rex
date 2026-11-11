//! The event log: `rex hook` appends one line for each event, and intake reads
//! the new lines back. Both sides live here so the line has one definition.

use std::fs::{File, OpenOptions};
use std::io::{self, ErrorKind, Read, Seek, SeekFrom, Write};
use std::path::Path;

use serde_json::{Value, json};

use crate::capture::error::CaptureError;

/// The line format version that this code writes and reads.
const VERSION: u64 = 1;

/// Append the event `input` to the log at `path` as one line stamped with `at_ms`,
/// making the file IF it is absent. Input that is not a JSON object is kept as a
/// string under `raw`, so nothing is dropped. Empty input writes nothing. Known
/// limit: the log is never cut short or rotated, so it grows by about three lines
/// for each sub-agent.
///
/// # Errors
/// The file cannot be opened or written.
pub(crate) fn append_event(path: &Path, input: &[u8], at_ms: u64) -> io::Result<()> {
    match line_for(input, at_ms) {
        Some(line) => append(path, &line),
        None => Ok(()),
    }
}

fn line_for(input: &[u8], at_ms: u64) -> Option<Vec<u8>> {
    let input = input.trim_ascii();
    if input.is_empty() {
        return None;
    }
    let line = match serde_json::from_slice::<Value>(input) {
        Ok(event @ Value::Object(_)) => json!({ "v": VERSION, "at_ms": at_ms, "event": event }),
        _ => json!({ "v": VERSION, "at_ms": at_ms, "raw": String::from_utf8_lossy(input) }),
    };
    let mut bytes = line.to_string().into_bytes();
    bytes.push(b'\n');
    Some(bytes)
}

/// The line goes out in one `write` call on a file opened for appending, and no
/// lock is taken. Linux and macOS place each such write at the end of the file in
/// one step, so lines from hooks that run at the same moment never mix, and a hook
/// never waits for another. `write_all` is not used: it may split the line into
/// several calls, and another hook's line can come between them.
fn append(path: &Path, line: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new().append(true).create(true).open(path)?;
    let written = file.write(line)?;
    // A short write (a full disk) is ended with a newline, so the damage stays in
    // this one line and the next hook's line starts on a line of its own.
    if written < line.len() {
        let _newline_written = file.write(b"\n")?;
    }
    Ok(())
}

/// The whole lines found after one offset of the log, and the offset to read from
/// next time.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LogRead {
    /// In log order.
    pub(crate) lines: Vec<LogLine>,
    /// Just past the last newline read. Bytes after it are an unfinished line.
    pub(crate) offset: u64,
}

/// One whole line of the log.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum LogLine {
    /// A line of this version that holds an event.
    Event { at_ms: u64, event: Value },
    /// A line that kept input which was not a JSON object. It carries no event.
    Raw,
    /// Not valid JSON, no `v`, a `v` of another version, or no usable content.
    Damaged,
}

/// The whole lines of the log at `path` from byte `offset` on. A log shorter than
/// `offset` was replaced, so it is read from the start. No log gives no lines and
/// offset 0.
///
/// # Errors
/// [`CaptureError::ReadLog`] if the log exists and cannot be read.
pub(crate) fn read_log(path: &Path, offset: u64) -> Result<LogRead, CaptureError> {
    let failed = |source| CaptureError::ReadLog {
        path: path.to_path_buf(),
        source,
    };
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Ok(LogRead {
                lines: Vec::new(),
                offset: 0,
            });
        }
        Err(source) => return Err(failed(source)),
    };
    let size = file.metadata().map_err(failed)?.len();
    let start = if size < offset { 0 } else { offset };
    file.seek(SeekFrom::Start(start)).map_err(failed)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).map_err(failed)?;
    Ok(whole_lines(&bytes, start))
}

/// The whole lines of `bytes`, which were read from byte `start` of the log.
fn whole_lines(bytes: &[u8], start: u64) -> LogRead {
    let Some(last_newline) = bytes.iter().rposition(|byte| *byte == b'\n') else {
        return LogRead {
            lines: Vec::new(),
            offset: start,
        };
    };
    let whole = bytes.get(..last_newline).unwrap_or_default();
    LogRead {
        lines: whole
            .split(|byte| *byte == b'\n')
            .map(LogLine::of)
            .collect(),
        offset: start + last_newline as u64 + 1,
    }
}

impl LogLine {
    fn of(line: &[u8]) -> LogLine {
        let Ok(Value::Object(mut line)) = serde_json::from_slice::<Value>(line) else {
            return LogLine::Damaged;
        };
        if line.get("v").and_then(Value::as_u64) != Some(VERSION) {
            return LogLine::Damaged;
        }
        let at_ms = line.get("at_ms").and_then(Value::as_u64);
        match (at_ms, line.remove("event")) {
            (Some(at_ms), Some(event @ Value::Object(_))) => LogLine::Event { at_ms, event },
            (_, None) if line.get("raw").is_some_and(Value::is_string) => LogLine::Raw,
            _ => LogLine::Damaged,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unfinished_line_waits_for_the_next_read() {
        let mut bytes = b"{\"v\":1,\"at_ms\":1,\"event\":{}}\n".to_vec();
        let whole = bytes.len() as u64;
        bytes.extend(b"{\"v\":1,\"at_ms\":2,\"ev");
        let read = whole_lines(&bytes, 0);
        assert_eq!(read.lines.len(), 1);
        assert_eq!(read.offset, whole);
        assert_eq!(whole_lines(b"{\"v\":1", 5).offset, 5);
    }
}
