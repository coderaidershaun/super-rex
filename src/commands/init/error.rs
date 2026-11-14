//! Every way `rex init` can fail. Each text is what the person reads on
//! stderr, so it names what is wrong and how to fix it.

use std::error::Error;
use std::path::PathBuf;
use std::process::ExitStatus;

use crate::error::{CommandError, FailureClass, RexError};
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

    #[error(
        ".rex/config.json could not be read and was left as it is, so the plugin cannot tell whether the record is kept; make it readable, then run rex init again"
    )]
    ConfigUnreadable {
        #[source]
        source: std::io::Error,
    },

    #[error(
        ".rex/config.json is not valid JSON and was left as it is, so the plugin stays off in this repo; correct it or remove it, then run rex init again"
    )]
    ConfigNotJson {
        #[source]
        source: serde_json::Error,
    },

    #[error(
        ".rex/config.json is not a JSON object and was left as it is, so the plugin stays off in this repo; make its top level an object or remove it, then run rex init again"
    )]
    ConfigNotObject,

    #[error(
        "could not run \"{command}\", so the super-rex plugin is not installed; check the Claude Code install, then run rex init again"
    )]
    PluginNotRun {
        command: String,
        #[source]
        source: std::io::Error,
    },

    #[error(
        "\"{command}\" exited with {status}, so the super-rex plugin is not installed; fix what it names, or give another --plugin-source, then run rex init again"
    )]
    PluginCommand {
        command: String,
        status: ExitStatus,
        #[source]
        stderr: Option<ToolStderr>,
    },

    /// No fix is given: the store's own error, shown after this text, says
    /// what went wrong, and no one fix covers every case.
    #[error("the store could not be set up")]
    Store {
        #[source]
        source: StoreError,
    },

    /// The one migration step that needs the person: a row that names a row
    /// that is not there can only be dropped or restored, and either is their choice.
    #[error(
        ".rex/rex.db is version 1, and {count} {rows} in it {names} a row that is not there, so rex did not migrate it: carrying the {row_or_rows} over or dropping {it_or_them} is your choice, and dropping {it_or_them} loses that record; list {it_or_them} with: sqlite3 .rex/rex.db 'PRAGMA foreign_key_check;' then delete {it_or_them} or restore the store from a backup, and run rex init --with-storage again",
        rows = if *count == 1 { "row" } else { "rows" },
        names = if *count == 1 { "names" } else { "name" },
        row_or_rows = if *count == 1 { "row" } else { "rows" },
        it_or_them = if *count == 1 { "it" } else { "them" }
    )]
    StoreNeedsChoice { count: i64 },

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

impl CommandError for InitError {
    fn failure_class(&self) -> FailureClass {
        match self {
            InitError::NotRepoRoot { .. }
            | InitError::GitIsFile { .. }
            | InitError::ClaudeNotFound
            | InitError::ClaudeVersionUnreadable { .. }
            | InitError::ClaudeTooOld { .. }
            | InitError::SettingsNotJson { .. }
            | InitError::SettingsNotObject
            | InitError::SettingsUnreadable { .. }
            | InitError::SettingsLinked { .. }
            | InitError::ConfigUnreadable { .. }
            | InitError::ConfigNotJson { .. }
            | InitError::ConfigNotObject
            | InitError::PreCommitInstall { .. }
            | InitError::Incomplete { .. } => FailureClass::Reject,
            // A repo whose agents run with a safety setting off must not be used
            // until a person sees it.
            InitError::SandboxToolsMissing { .. }
            | InitError::SafetySettingKept { .. }
            | InitError::StoreNeedsChoice { .. } => FailureClass::Halt,
            InitError::ClaudeNotRun { .. }
            | InitError::PluginNotRun { .. }
            | InitError::PluginCommand { .. } => FailureClass::Retry,
            InitError::Store { source } => source.failure_class(),
            InitError::Rex(error) => error.failure_class(),
        }
    }
}

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
