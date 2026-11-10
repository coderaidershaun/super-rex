//! Errors for every command, and the trait that lets `main` choose the exit
//! code. A failure that involves a file carries its path, so the user learns
//! where it happened, not just what went wrong.

use std::path::PathBuf;

/// Failure from any `rex` command. File failures carry the path where they happened.
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

    #[error("`pre-commit install` exited with {status}: {stderr}")]
    PreCommitInstall {
        status: std::process::ExitStatus,
        stderr: String,
    },

    #[error("{} is a symbolic link; rex does not write through one", path.display())]
    SymbolicLink { path: PathBuf },

    #[error("failed to write to stdout")]
    Stdout {
        #[source]
        source: std::io::Error,
    },

    #[error("`rex {command}` is not built yet")]
    NotBuilt { command: &'static str },
}

/// What `main` asks of the error a command returns, to choose the exit code.
pub trait CommandError: std::error::Error + Send + Sync + 'static {
    /// True when the command line is at fault and not the work.
    fn is_usage(&self) -> bool {
        false
    }
}

impl CommandError for RexError {}
