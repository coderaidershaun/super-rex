//! Every way `rex init` can fail. Each text is what the person reads on
//! stderr, so it names what is wrong and how to fix it.

use std::error::Error;
use std::path::PathBuf;
use std::process::ExitStatus;

use crate::error::{CommandError, RexError};
use crate::store::StoreError;

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
        "the Claude Code sandbox cannot start on this machine: {missing} not found on PATH, so commands run with no sandbox in this repo until they are installed (after rex init --strict, Claude Code refuses to start instead); install them with sudo apt-get install bubblewrap socat (Ubuntu or Debian), or sudo dnf install bubblewrap socat (Fedora)"
    )]
    SandboxToolsMissing { missing: String },

    #[error(
        "kept {key} = {found} in .claude/settings.local.json, so the protection it gives is off; set it to {needed} to turn it on"
    )]
    SafetySettingKept {
        key: String,
        found: String,
        needed: String,
    },

    #[error(
        ".claude/settings.local.json is not valid JSON and was left as it is, so the settings SuperRex needs are not set; correct it, then run rex init again"
    )]
    SettingsNotJson {
        #[source]
        source: serde_json::Error,
    },

    #[error(
        ".claude/settings.local.json is not a JSON object and was left as it is, so the settings SuperRex needs are not set; make its top level an object, then run rex init again"
    )]
    SettingsNotObject,

    #[error(
        ".claude/settings.local.json could not be read and was left as it is, so the settings SuperRex needs are not set; make it readable, then run rex init again"
    )]
    SettingsUnreadable {
        #[source]
        source: std::io::Error,
    },

    #[error(
        "{link} is a symbolic link, and rex does not write through one, so the settings SuperRex needs are not set; add them to .claude/settings.local.json where the link points, then run rex init again"
    )]
    SettingsLinked { link: &'static str },

    /// No fix is given: the store's own error, shown after this text, says
    /// what went wrong, and no one fix covers every case.
    #[error("the store could not be set up")]
    Store {
        #[source]
        source: StoreError,
    },

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
