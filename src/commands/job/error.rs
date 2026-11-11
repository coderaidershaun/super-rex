//! What can go wrong in a job command that the store cannot see: the command
//! line and the input files. Each text says what failed, then the fix.

use std::path::PathBuf;

use crate::capture::CaptureError;
use crate::error::{CommandError, RexError};
use crate::store::StoreError;

/// Failure of `rex job create`, `plan`, `close` or `show`.
#[derive(thiserror::Error, Debug)]
pub enum JobError {
    #[error(
        "no session ID; pass --session <id>, or run inside Claude Code, which sets CLAUDE_CODE_SESSION_ID"
    )]
    NoSession,

    #[error(
        "--lang none cannot be combined with other languages; give none alone, or only language names"
    )]
    LangNoneMixed,

    #[error(
        "give a job ID; CLAUDE_CODE_SESSION_ID is not set, so there is no session to find the open job of"
    )]
    NoJobArgument,

    /// `file` names the file's job, such as `request` or `plan`.
    #[error("cannot read {file} file {}", path.display())]
    ReadFile {
        file: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("request file {} is empty; write the request text into it", path.display())]
    EmptyRequest { path: PathBuf },

    #[error("{file} file {} is not valid", path.display())]
    InvalidFile {
        file: &'static str,
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("plan file {} has no roles; give at least one role", path.display())]
    NoRoles { path: PathBuf },

    #[error("plan file {} names role {role} twice; give each role once", path.display())]
    RoleTwice { path: PathBuf, role: String },

    #[error(
        "role name {role} in plan file {} is not valid; use lower-case letters, digits and -",
        path.display()
    )]
    BadRoleName { role: String, path: PathBuf },

    #[error(
        "orchestrator name {name} in plan file {} is not valid; use lower-case letters, digits and -",
        path.display()
    )]
    BadOrchestratorName { name: String, path: PathBuf },

    #[error(
        "skill name {name} in plan file {} is not valid; give the skill folder name, lower-case letters and digits in groups joined by single hyphens, such as rex-code-tdd for rex-code-tdd/mocking.md",
        path.display()
    )]
    BadSkillName { name: String, path: PathBuf },

    #[error(
        "skill {name} of role {role} in plan file {} is not in the job's \"skills\"; add it to \"skills\", or remove it from the role",
        path.display()
    )]
    RoleSkillNotInJob {
        role: String,
        name: String,
        path: PathBuf,
    },

    #[error(
        "role {role} in plan file {} names skill {name} twice; give each skill once",
        path.display()
    )]
    RoleSkillTwice {
        role: String,
        name: String,
        path: PathBuf,
    },

    #[error(
        "plan file {} has an empty breakdown; write the work breakdown into \"breakdown\"",
        path.display()
    )]
    EmptyBreakdown { path: PathBuf },

    #[error(
        "role {role} lacks {key}; give all four of intelligence, reliability, consequence and verification (each low, medium, high or none)"
    )]
    MissingInput { role: String, key: &'static str },

    /// `store` is the `.rex` folder of the store.
    #[error(
        "orchestrator {name} not found at {}/orchestrators/{name}; write the orchestrator into that folder first",
        store.display()
    )]
    OrchestratorNotFound { name: String, store: PathBuf },

    #[error(
        "\"{key}\" of the delivery to {repo} in deliveries file {} is the text none; write null or leave the key out",
        path.display()
    )]
    NoneText {
        key: &'static str,
        repo: String,
        path: PathBuf,
    },

    #[error(
        "repo {given} in deliveries file {} is not in a Git repository; give a path inside the repository the job changed",
        path.display()
    )]
    RepoNotInGit { given: String, path: PathBuf },

    #[error(
        "deliveries file {} names repo {} twice; give one entry per repository",
        path.display(),
        root.display()
    )]
    RepoTwice { path: PathBuf, root: PathBuf },

    #[error(
        "delivery to {} is merged but no merge was agreed for it; record it as pr-open or kept-local, or set \"merge_agreed\": true if the requester agreed",
        root.display()
    )]
    MergedNotAgreed { root: PathBuf },

    #[error(
        "delivery to {} is {outcome} but merge_agreed is {is_agreed}; pr-open and kept-local mean no merge was agreed, so they need merge_agreed false, and blocked means an agreed merge could not be completed, so it needs merge_agreed true",
        root.display()
    )]
    AgreementMismatch {
        root: PathBuf,
        outcome: &'static str,
        is_agreed: bool,
    },

    #[error(
        "delivery to {} is merged but has no merge_commit; add the full hash of the squash commit on main",
        root.display()
    )]
    MergedNoCommit { root: PathBuf },

    #[error(
        "delivery to {} is pr-open but has no pr; add the pull request URL",
        root.display()
    )]
    PrOpenNoPr { root: PathBuf },

    #[error(
        "delivery to {} is {outcome} but has no note; add a \"note\" that says what remains to do",
        root.display()
    )]
    NoNote {
        root: PathBuf,
        outcome: &'static str,
    },

    #[error(
        "job {job} cannot be delivered while its delivery to {repo} is blocked; close it as blocked, or record that repository's new outcome in the deliveries file"
    )]
    DeliveredWhileBlocked { job: String, repo: String },

    #[error(transparent)]
    Store(#[from] StoreError),

    #[error(transparent)]
    Capture(#[from] CaptureError),

    #[error(transparent)]
    Rex(#[from] RexError),
}

impl CommandError for JobError {
    fn is_usage(&self) -> bool {
        matches!(
            self,
            JobError::NoSession | JobError::LangNoneMixed | JobError::NoJobArgument
        )
    }
}

impl From<rusqlite::Error> for JobError {
    fn from(source: rusqlite::Error) -> Self {
        JobError::Store(source.into())
    }
}
