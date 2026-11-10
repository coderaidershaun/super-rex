//! Prints a command's output to stdout. Kept in one place so every command
//! treats a reader that stops early the same way.

use std::io::{self, ErrorKind, Write};

use crate::error::RexError;

/// Write `line` to stdout followed by a newline. A closed pipe is handled as
/// in [`print_to_stdout`].
///
/// # Errors
/// [`RexError::Stdout`] if the line cannot be written.
pub(super) fn print_line_to_stdout(line: &str) -> Result<(), RexError> {
    print_to_stdout(&format!("{line}\n"))
}

/// Write `text` to stdout as it is, with no newline added.
///
/// # Errors
/// [`RexError::Stdout`] if the text cannot be written.
pub(super) fn print_to_stdout(text: &str) -> Result<(), RexError> {
    let mut stdout = io::stdout().lock();
    let written = stdout
        .write_all(text.as_bytes())
        .and_then(|()| stdout.flush());

    // A reader such as `head` closing the pipe early is a normal way to use
    // a command, not a failure.
    match written {
        Err(source) if source.kind() == ErrorKind::BrokenPipe => Ok(()),
        other => other.map_err(|source| RexError::Stdout { source }),
    }
}
