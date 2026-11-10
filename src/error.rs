//! The error of the helpers that commands share, and the trait that every
//! command's error implements so that `main` can choose the exit code. A
//! failure that involves a file carries its path, so the user learns where it
//! happened, not just what went wrong.

use std::path::PathBuf;

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
    /// True when the command line is at fault and not the work: exit 2, not 1.
    /// Clap's own errors never get here, because clap exits with 2 itself.
    fn is_usage(&self) -> bool {
        false
    }
}

impl CommandError for RexError {}
