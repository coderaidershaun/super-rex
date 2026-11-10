//! The event log: `rex hook` appends one line for each event, and intake reads
//! the new lines back. Both sides live here so the line has one definition.

use std::fs::{File, OpenOptions};
use std::io::{self, ErrorKind, Read, Seek, SeekFrom, Write};
use std::path::Path;

use serde_json::{Value, json};

use super::error::CaptureError;

/// The line format version that this code writes and reads.
const VERSION: u64 = 1;

/// Append the event `input` to the log at `path` as one line stamped with `at_ms`,
/// making the file IF it is absent. Input that is not a JSON object is kept as a
/// string under `raw`, so nothing is dropped. Empty input writes nothing.
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
/// never waits for another.
fn append(path: &Path, line: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new().append(true).create(true).open(path)?;
    let written = file.write(line)?;
    // A short write (a full disk) is tried once more and no further. What it leaves
    // behind is one damaged line, which every reader of the log skips.
    if let Some(rest) = line.get(written..)
        && !rest.is_empty()
    {
        let _written_again = file.write(rest)?;
    }
    Ok(())
}

/// The whole lines found after one offset of the log, and the offset to read from
/// next time.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LogRead {
    /// In log order.
    pub(crate) lines: Vec<LogLine>,
    /// Just past the last newline read. Bytes after it are an unfinished line.
    pub(crate) offset: u64,
}

/// One whole line of the log.
#[allow(dead_code)]
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
#[allow(dead_code)]
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

    fn line(text: &str) -> Vec<u8> {
        format!("{text}\n").into_bytes()
    }

    #[test]
    fn written_lines_read_back_in_order() {
        let mut bytes = line_for(b"{\"hook_event_name\":\"SubagentStart\"}\n", 7).unwrap();
        bytes.extend(line_for(b"not json", 8).unwrap());
        bytes.extend(line_for(b"{\"b\":[1,2]}", 9).unwrap());
        let read = whole_lines(&bytes, 100);
        assert_eq!(
            read.lines,
            vec![
                LogLine::Event {
                    at_ms: 7,
                    event: json!({ "hook_event_name": "SubagentStart" })
                },
                LogLine::Raw,
                LogLine::Event {
                    at_ms: 9,
                    event: json!({ "b": [1, 2] })
                },
            ]
        );
        assert_eq!(read.offset, 100 + bytes.len() as u64);
    }

    #[test]
    fn unfinished_line_waits_for_the_next_read() {
        let mut bytes = line("{\"v\":1,\"at_ms\":1,\"event\":{}}");
        let whole = bytes.len() as u64;
        bytes.extend(b"{\"v\":1,\"at_ms\":2,\"ev");
        let read = whole_lines(&bytes, 0);
        assert_eq!(read.lines.len(), 1);
        assert_eq!(read.offset, whole);
        assert_eq!(whole_lines(b"{\"v\":1", 5).offset, 5);
    }

    #[test]
    fn unusable_lines_are_damaged() {
        let mut bytes = Vec::new();
        for text in [
            "{\"v\":1,\"at_ms\":1,\"ev",
            "{\"at_ms\":1,\"event\":{}}",
            "{\"v\":2,\"at_ms\":1,\"event\":{}}",
            "{\"v\":1,\"event\":{}}",
            "{\"v\":1,\"at_ms\":1,\"event\":\"text\"}",
            "{\"v\":1,\"at_ms\":1}",
            "[1]",
            "",
        ] {
            bytes.extend(line(text));
        }
        let read = whole_lines(&bytes, 0);
        assert_eq!(read.lines, vec![LogLine::Damaged; 8]);
    }

    #[test]
    fn empty_input_gives_no_line() {
        assert_eq!(line_for(b"", 1), None);
        assert_eq!(line_for(b" \n", 1), None);
    }
}
