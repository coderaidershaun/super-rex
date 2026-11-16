//! The form of the live snapshot that the mod writes to `.rex/live/<session>.json`,
//! key for key as `fixtures/shared/live/snapshot.json` holds it.

#![allow(
    dead_code,
    reason = "each key of the form is named, so a snapshot of another form does not load"
)]

use serde::Deserialize;

/// The `version` of the form this binary reads. A file of any other version is
/// not read and not removed.
pub const VERSION: u32 = 1;

/// One snapshot file.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub version: u32,
    pub session: String,
    pub written_at: String,
    pub storage: String,
    pub versions: Versions,
    pub trees: Vec<LiveTree>,
    pub jobs: Vec<LiveJob>,
    pub runs: Vec<LiveRun>,
    pub questions: Vec<LiveQuestion>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Versions {
    pub r#mod: String,
    pub binary: String,
    pub claude_code: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiveTree {
    pub id: String,
    pub state: String,
    pub master: Option<String>,
    pub charter: String,
    /// What the tree has spent against its cap: its master's own spend and
    /// the spend of every job of the tree.
    pub spent_nano: u64,
    pub cap_nano: u64,
    pub started_at: String,
    pub closed_at: Option<String>,
    pub report_file: Option<String>,
    pub notes: Vec<LiveNote>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiveNote {
    pub at: String,
    pub job: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiveTokens {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_creation: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiveJob {
    pub id: String,
    pub tree: Option<String>,
    pub builder: String,
    pub origin: String,
    pub status: String,
    pub request: String,
    pub r#type: Option<String>,
    pub size: Option<String>,
    pub consequence: Option<String>,
    pub langs: Vec<String>,
    pub orchestrator: Option<String>,
    pub merge: Option<String>,
    pub merge_source: Option<String>,
    pub repo: Option<String>,
    pub branch: Option<String>,
    pub base: Option<String>,
    pub worktree: Option<String>,
    pub limit_nano: Option<u64>,
    pub spent_nano: u64,
    pub tokens: LiveTokens,
    pub degraded: Vec<String>,
    pub deliveries: Vec<LiveDelivery>,
    pub done_checks: Vec<LiveDoneCheck>,
    pub outcome: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiveDelivery {
    pub repo: String,
    pub outcome: String,
    pub merge_commit: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiveDoneCheck {
    pub check: String,
    pub result: String,
    pub at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiveRun {
    pub agent_id: String,
    pub job: Option<String>,
    pub tree: Option<String>,
    pub parent: Option<String>,
    pub depth: u32,
    pub role: Option<String>,
    pub task: Option<String>,
    pub agent_type: Option<String>,
    pub planned: Option<String>,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub state: String,
    pub end_reason: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub tokens: LiveTokens,
    pub spent_nano: u64,
    pub marks: Vec<String>,
    pub lint: Option<String>,
    pub skills_attached: Vec<String>,
    pub skills_opened: Vec<String>,
    pub output_file: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiveQuestion {
    pub id: String,
    pub tree: Option<String>,
    pub job: Option<String>,
    pub question: String,
    pub options: Vec<String>,
    pub recommendation: String,
    pub category: String,
    pub asked_at: String,
    pub answer: Option<LiveAnswer>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiveAnswer {
    pub option: String,
    pub source: String,
    pub at: String,
}
