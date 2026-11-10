//! Every way `rex init` can fail. Each text is what the person reads on
//! stderr, so it names what is wrong and how to fix it.

use std::error::Error;
use std::path::PathBuf;
use std::process::ExitStatus;

use crate::error::{CommandError, RexError};

/// A failure of `rex init`. None of them is a wrong command line.
#[derive(thiserror::Error, Debug)]
pub enum InitError {
    #[error("{} is not the root of a Git repository; run rex init in the folder that holds .git, or run git init first", cwd.display())]
    NotRepoRoot { cwd: PathBuf },

    #[error("{} is a linked worktree, a submodule or a --separate-git-dir checkout (.git is a file); run rex init in the main checkout of the repository", cwd.display())]
    GitIsFile { cwd: PathBuf },

    #[error(
        "Claude Code was not found: claude is not on PATH; install Claude Code, then run rex init again"
    )]
    ClaudeNotFound,

    #[error(
        "could not run \"claude --version\"; check the Claude Code install, then run rex init again"
    )]
    ClaudeNotRun {
        #[source]
        source: std::io::Error,
    },

    #[error(
        "could not read the Claude Code version from \"claude --version\" (it printed: {printed}); run claude update, then run rex init again"
    )]
    ClaudeVersionUnreadable { printed: String },

    #[error(
        "Claude Code {found} is older than {minimum}, the oldest version SuperRex works with; run claude update, then run rex init again"
    )]
    ClaudeTooOld { found: String, minimum: String },

    #[error(
        "the Claude Code sandbox cannot start on this machine: {missing} not found on PATH, so Claude Code will refuse to start in this repo until they are installed; install them with sudo apt-get install bubblewrap socat (Ubuntu or Debian), or sudo dnf install bubblewrap socat (Fedora)"
    )]
    SandboxToolsMissing { missing: String },

    #[error(
        "`pre-commit install` exited with {status}; fix what it names, then run rex init again"
    )]
    PreCommitInstall {
        status: ExitStatus,
        #[source]
        stderr: Option<ToolStderr>,
    },

    #[error("rex init did not finish {count} step(s); each is named above")]
    Incomplete { count: usize },

    #[error(transparent)]
    Rex(#[from] RexError),
}

impl CommandError for InitError {}

impl InitError {
    /// The text and each of its sources, joined by `: `, as one line.
    pub(super) fn with_sources(&self) -> String {
        let mut text = self.to_string();
        let mut source = self.source();
        while let Some(cause) = source {
            text.push_str(": ");
            text.push_str(&cause.to_string());
            source = cause.source();
        }
        text
    }
}

/// What a tool wrote to stderr, made into one line, so it can be the source
/// of the failure it explains.
#[derive(thiserror::Error, Debug)]
#[error("{0}")]
pub struct ToolStderr(String);

impl ToolStderr {
    /// The non-blank lines of `stderr`, trimmed and joined by spaces, or
    /// `None` when the tool wrote nothing.
    pub(super) fn from_output(stderr: &[u8]) -> Option<Self> {
        let text = String::from_utf8_lossy(stderr);
        let lines: Vec<&str> = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect();
        if lines.is_empty() {
            None
        } else {
            Some(Self(lines.join(" ")))
        }
    }
}
