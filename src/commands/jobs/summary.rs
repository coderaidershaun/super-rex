//! One job as `rex jobs` shows it: the summary, its JSON document and its
//! plain line. `rex watch` fills its job list from the same summary.

use serde::Serialize;

use super::JobsError;
use super::select::{DeliveryRow, JobRow};
use crate::capture::{JobRuns, TokenCounts, TokenTotal};
use crate::store::status::Shown;

/// How many characters of the request a summary keeps.
pub const EXCERPT_CHARS: usize = 200;

/// One job as `rex jobs --json` prints it. The field order is the order of the keys.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct JobSummary {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub status: String,
    pub stored_status: String,
    pub request: String,
    pub request_truncated: bool,
    pub asked_by: String,
    pub class_version: i64,
    pub work_type: String,
    pub size: String,
    pub consequence: String,
    pub languages: Vec<String>,
    pub orchestrator: Option<OrchestratorRef>,
    pub merge_agreed: bool,
    pub builder_model: Option<String>,
    pub builder_effort: Option<String>,
    pub deliveries: Vec<DeliverySummary>,
    pub runs: RunCounts,
    pub tokens: Option<TokenSummary>,
    pub incomplete: bool,
    pub has_feedback: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OrchestratorRef {
    pub name: String,
    pub version: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeliverySummary {
    pub repo: String,
    pub branch: Option<String>,
    pub pr: Option<String>,
    pub outcome: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RunCounts {
    pub open: u32,
    pub finished: u32,
    pub unconfirmed: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TokenSummary {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_creation: u64,
    pub provisional: bool,
}

impl TokenSummary {
    /// `None` when nothing was ever read.
    pub fn of(total: &TokenTotal) -> Option<TokenSummary> {
        let (counts, provisional) = match total {
            TokenTotal::Unknown => return None,
            TokenTotal::Provisional(counts) => (counts, true),
            TokenTotal::Final(counts) => (counts, false),
        };
        let TokenCounts {
            input,
            output,
            cache_read,
            cache_creation,
        } = *counts;
        Some(TokenSummary {
            input,
            output,
            cache_read,
            cache_creation,
            provisional,
        })
    }

    /// The sum of the four counts, saturating.
    pub fn sum(&self) -> u64 {
        self.input
            .saturating_add(self.output)
            .saturating_add(self.cache_read)
            .saturating_add(self.cache_creation)
    }
}

impl JobSummary {
    pub fn new(
        row: JobRow,
        deliveries: Vec<DeliveryRow>,
        runs: &JobRuns,
        has_feedback: bool,
        shown: Shown,
    ) -> JobSummary {
        let request_truncated = row.request.chars().nth(EXCERPT_CHARS).is_some();
        JobSummary {
            id: row.id,
            created_at: row.created_at,
            updated_at: row.updated_at,
            status: shown.to_string(),
            stored_status: row.stored_status,
            request: row.request.chars().take(EXCERPT_CHARS).collect(),
            request_truncated,
            asked_by: row.asked_by,
            class_version: row.class_version,
            work_type: row.work_type,
            size: row.size,
            consequence: row.consequence,
            languages: row.languages.split(',').map(str::to_owned).collect(),
            orchestrator: row
                .orchestrator
                .map(|(name, version)| OrchestratorRef { name, version }),
            merge_agreed: row.merge_agreed,
            builder_model: row.builder_model,
            builder_effort: row.builder_effort,
            deliveries: deliveries
                .into_iter()
                .map(|d| DeliverySummary {
                    repo: d.repo,
                    branch: d.branch,
                    pr: d.pr,
                    outcome: d.outcome,
                })
                .collect(),
            runs: RunCounts {
                open: runs.open,
                finished: runs.finished,
                unconfirmed: runs.unconfirmed,
            },
            tokens: TokenSummary::of(&runs.tokens),
            incomplete: runs.incomplete,
            has_feedback,
        }
    }

    /// The request excerpt with each line break turned into one space.
    pub fn excerpt_on_one_line(&self) -> String {
        self.request.replace("\r\n", " ").replace(['\n', '\r'], " ")
    }

    /// The plain output line: `<id> <status> <type> <size> <consequence> <languages> <created_at> <excerpt>`.
    pub fn plain_line(&self) -> String {
        format!(
            "{} {} {} {} {} {} {} {}",
            self.id,
            self.status,
            self.work_type,
            self.size,
            self.consequence,
            self.languages.join(","),
            self.created_at,
            self.excerpt_on_one_line(),
        )
    }
}

#[derive(Serialize)]
struct JobsDocument<'a> {
    schema: u32,
    jobs: &'a [JobSummary],
}

/// The one-line JSON document of `rex jobs --json`.
///
/// # Errors
/// [`JobsError::Json`] if serialising fails.
pub fn jobs_json(jobs: &[JobSummary]) -> Result<String, JobsError> {
    serde_json::to_string(&JobsDocument { schema: 1, jobs }).map_err(JobsError::Json)
}
