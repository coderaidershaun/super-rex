//! Every way `rex worktree` can fail, each with what to do instead.

use std::path::PathBuf;

use crate::error::{CommandError, FailureClass, RexError};

/// A failure of `rex worktree`.
#[derive(thiserror::Error, Debug)]
pub enum WorktreeError {
    #[error("{} is not the main checkout of a Git repository (the folder that holds a .git folder); give the repo's main checkout", repo.display())]
    NotMainCheckout { repo: PathBuf },

    #[error("the repo at {} has no folder for job worktrees beside it (<parent>/<name>.worktrees is missing a name or is a symbolic link); move the repo or remove that link", repo.display())]
    NoWorktreesFolder { repo: PathBuf },

    #[error("{} is not an absolute path without . or ..; give the full path of the worktree", dir.display())]
    NotPlainAbsolute { dir: PathBuf },

    #[error("{} is not inside {}, the only folder rex makes or removes job worktrees in; give a path under it", dir.display(), root.display())]
    Outside { dir: PathBuf, root: PathBuf },

    #[error("{} already exists; give a path that does not exist yet", dir.display())]
    Exists { dir: PathBuf },

    #[error("{} is not a linked worktree of {}; rex removes only its own job worktrees", dir.display(), repo.display())]
    NotWorktree { dir: PathBuf, repo: PathBuf },

    #[error("{} is not valid UTF-8, so it cannot be written into the settings file; use a path of plain text", dir.display())]
    NotUtf8 { dir: PathBuf },

    #[error(
        "{commit} is not a full commit hash (40 or 64 hex digits); give the output of git rev-parse"
    )]
    NotCommitHash { commit: String },

    #[error("{branch} starts with -, so git would read it as an option; give a branch name")]
    BranchLikeOption { branch: String },

    #[error("could not run git; install git, or put it on PATH")]
    GitNotRun {
        #[source]
        source: std::io::Error,
    },

    #[error("\"git {args}\" failed: {output}")]
    Git { args: String, output: String },

    #[error("git made {}, but it is not a linked worktree of {} there; remove it with git worktree remove, then try again", dir.display(), repo.display())]
    NotLinkedAfterAdd { dir: PathBuf, repo: PathBuf },

    #[error(
        ".claude/settings.local.json is not valid JSON, so nothing was made; correct it, or run rex init"
    )]
    SettingsNotJson {
        #[source]
        source: serde_json::Error,
    },

    #[error(
        ".claude/settings.local.json is not a JSON object, so nothing was made; make its top level an object"
    )]
    SettingsNotObject,

    #[error(
        ".claude/settings.local.json has a permissions that is not an object or an additionalDirectories that is not a list, so nothing was made; correct it"
    )]
    SettingsNotList,

    #[error(transparent)]
    Rex(#[from] RexError),
}

impl CommandError for WorktreeError {
    fn failure_class(&self) -> FailureClass {
        match self {
            WorktreeError::NotMainCheckout { .. }
            | WorktreeError::NoWorktreesFolder { .. }
            | WorktreeError::NotPlainAbsolute { .. }
            | WorktreeError::Outside { .. }
            | WorktreeError::Exists { .. }
            | WorktreeError::NotWorktree { .. }
            | WorktreeError::NotUtf8 { .. }
            | WorktreeError::NotCommitHash { .. }
            | WorktreeError::BranchLikeOption { .. }
            | WorktreeError::Git { .. }
            | WorktreeError::SettingsNotJson { .. }
            | WorktreeError::SettingsNotObject
            | WorktreeError::SettingsNotList => FailureClass::Reject,
            WorktreeError::GitNotRun { .. } => FailureClass::Retry,
            // Git made a folder that is not what it should be: a person looks first.
            WorktreeError::NotLinkedAfterAdd { .. } => FailureClass::Halt,
            WorktreeError::Rex(error) => error.failure_class(),
        }
    }

    fn is_usage(&self) -> bool {
        matches!(
            self,
            WorktreeError::NotMainCheckout { .. }
                | WorktreeError::NoWorktreesFolder { .. }
                | WorktreeError::NotPlainAbsolute { .. }
                | WorktreeError::Outside { .. }
                | WorktreeError::Exists { .. }
                | WorktreeError::NotWorktree { .. }
                | WorktreeError::NotUtf8 { .. }
                | WorktreeError::NotCommitHash { .. }
                | WorktreeError::BranchLikeOption { .. }
        )
    }
}
