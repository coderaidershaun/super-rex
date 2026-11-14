//! The error of the helpers that commands share, and the trait that every
//! command's error implements so that `main` can choose the exit code. A
//! failure that involves a file carries its path, so the user learns where it
//! happened, not just what went wrong.

use std::path::PathBuf;

/// What the caller of a command does after it fails: stop all new work and
/// keep the state for a person (`Halt`), rebuild one part from a trusted
/// source (`Reset`), do the same again after a delay (`Retry`), or drop this
/// one input and go on (`Reject`). The plugin acts on it through the exit code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureClass {
    Halt,
    Reset,
    Retry,
    Reject,
}

/// Failure of a helper that several commands share, such as printing to
/// stdout. `rex codebase`, `rex smells`, `rex commands` and `rex hook` fail
/// only in these ways and return it as it is. Every other command has its own
/// error type, which passes this one on with `#[error(transparent)]`; a new
/// failure goes there, not here.
#[derive(thiserror::Error, Debug)]
pub enum RexError {
    #[error("io error at {}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("walk failed at {}", path.display())]
    Walk {
        path: PathBuf,
        #[source]
        source: ignore::Error,
    },

    #[error("{} is a symbolic link; rex does not write through one", path.display())]
    SymbolicLink { path: PathBuf },

    #[error("failed to write to stdout")]
    Stdout {
        #[source]
        source: std::io::Error,
    },
}

/// What `main` asks of the error a command returns, to choose the exit code.
/// Each command has its own `thiserror` enum with `impl CommandError for
/// TheError {}`.
///
/// `main` prints the error, then each of its sources, on one line after
/// `error: `. So a text starts lower-case (unless it starts with a name such as
/// `SQLite`), has no full stop and no `error:` of its own, says what failed with
/// the value and path seen, then the fix when it is known, and leaves the
/// underlying error to `#[source]`.
pub trait CommandError: std::error::Error + Send + Sync + 'static {
    /// What the caller does next. Every variant names its class, with no
    /// arm for the rest, so a new variant cannot get one by accident; a
    /// variant that wraps another error gives the inner error's class.
    fn failure_class(&self) -> FailureClass;

    /// True when the command line is at fault and not the work: exit 2, not 1.
    /// Clap's own errors never get here, because clap exits with 2 itself.
    fn is_usage(&self) -> bool {
        false
    }

    /// The exit code: 2 for a usage error, else 1. An error that a program
    /// branches on, such as a refused batch of `rex ingest`, gives its own.
    fn exit_code(&self) -> u8 {
        if self.is_usage() { 2 } else { 1 }
    }

    /// True for a refusal whose text a program reads as it is: `main` prints it
    /// alone on its line, with no `error: ` in front.
    fn is_bare_line(&self) -> bool {
        false
    }
}

impl CommandError for RexError {
    fn failure_class(&self) -> FailureClass {
        match self {
            RexError::Io { .. } | RexError::Walk { .. } => FailureClass::Retry,
            RexError::SymbolicLink { .. } | RexError::Stdout { .. } => FailureClass::Reject,
        }
    }
}
