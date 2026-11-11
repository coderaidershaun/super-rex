//! The screen of `rex watch` as plain values and the texts they hold. This file
//! never reads the store and draws nothing: the caller fills a [`ViewModel`].

use crate::capture::{Cost, TokenCounts, TokenTotal};
use crate::commands::jobs::{JobSummary, RunCounts};
use crate::commands::token_text::{cost_text, hit_text, short_count, split_text};

/// One line of the job list: the columns of the plain `rex jobs` line, then
/// the run counts and the token texts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobListRow {
    pub id: String,
    pub status: String,
    pub work_type: String,
    pub size: String,
    pub consequence: String,
    pub languages: String,
    pub created_at: String,
    pub excerpt: String,
    pub runs: RunCounts,
    /// The orchestrator skill that runs the job and its version, such as
    /// `pipeline v1`; empty while the plan names none.
    pub orchestrator: String,
    /// The cost at list prices, with `~` while the total can grow. Empty when the
    /// tokens are unknown.
    pub cost: String,
    pub hit: String,
    /// The four counts, the hit % and the cost on one line, for the job panel.
    pub split: String,
}

impl JobListRow {
    pub fn of(summary: &JobSummary) -> JobListRow {
        JobListRow {
            id: summary.id.clone(),
            status: summary.status.clone(),
            work_type: summary.work_type.clone(),
            size: summary.size.clone(),
            consequence: summary.consequence.clone(),
            languages: summary.languages.join(","),
            created_at: summary.created_at.clone(),
            excerpt: summary.excerpt_on_one_line(),
            runs: summary.runs,
            orchestrator: summary
                .orchestrator
                .as_ref()
                .map(|orchestrator| format!("{} v{}", orchestrator.name, orchestrator.version))
                .unwrap_or_default(),
            cost: summary
                .tokens
                .zip(summary.cost)
                .map(|(tokens, cost)| marked(&cost_text(&cost.0), tokens.provisional))
                .unwrap_or_default(),
            hit: summary
                .tokens
                .map(|tokens| hit_text(&tokens.counts()))
                .unwrap_or_default(),
            split: summary
                .tokens
                .zip(summary.cost)
                .map(|(tokens, cost)| {
                    let split = split_text(&tokens.counts());
                    marked(
                        &format!("{split}  cost {}", cost_text(&cost.0)),
                        tokens.provisional,
                    )
                })
                .unwrap_or_default(),
        }
    }
}

/// The token columns of one tree row. Every text is empty when the tokens are unknown.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TokenTexts {
    pub input: String,
    pub cache_write: String,
    pub cache_read: String,
    pub output: String,
    pub hit: String,
    /// The cost at list prices, with `~` while the total can grow.
    pub cost: String,
}

impl TokenTexts {
    /// `cost` is the cost of `total`, `None` when it is unknown.
    pub fn of(total: &TokenTotal, cost: Option<Cost>) -> TokenTexts {
        let (counts, provisional) = match total {
            TokenTotal::Unknown => return TokenTexts::default(),
            TokenTotal::Provisional(counts) => (counts, true),
            TokenTotal::Final(counts) => (counts, false),
        };
        let TokenCounts {
            input,
            output,
            cache_read,
            cache_creation,
        } = *counts;
        TokenTexts {
            input: short_count(input),
            cache_write: short_count(cache_creation),
            cache_read: short_count(cache_read),
            output: short_count(output),
            hit: hit_text(counts),
            cost: cost.map_or_else(String::new, |cost| marked(&cost_text(&cost), provisional)),
        }
    }
}

/// `text` with `~` after it when the total can still grow.
fn marked(text: &str, provisional: bool) -> String {
    let mark = if provisional { "~" } else { "" };
    format!("{text}{mark}")
}

/// One sub-agent run in the tree, shown at `depth` levels below the top.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeRow {
    pub depth: usize,
    pub role: String,
    /// The model that ran, else `-`.
    pub model: String,
    pub effort: String,
    /// `-` on the row of the main session, which has no run state.
    pub state: &'static str,
    /// `-` when the run has no duration yet.
    pub duration: String,
    pub tokens: TokenTexts,
    /// The job named by the run's own tag, when it differs from the job shown.
    pub tag_job: Option<String>,
}

/// Everything `draw` needs. `tree` holds the runs of the selected job.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ViewModel {
    pub jobs: Vec<JobListRow>,
    pub tree: Vec<TreeRow>,
}

/// `41s`, `2m05s`, `1h02m`. `-` when the run has no duration.
pub fn duration_text(duration_ms: Option<u64>) -> String {
    let Some(milliseconds) = duration_ms else {
        return "-".to_owned();
    };
    let seconds = milliseconds / 1_000;
    if seconds < 60 {
        format!("{seconds}s")
    } else if seconds < 3_600 {
        format!("{}m{:02}s", seconds / 60, seconds % 60)
    } else {
        format!("{}h{:02}m", seconds / 3_600, seconds % 3_600 / 60)
    }
}

/// `2026-11-10T09:05:00.000Z` as `2026-11-10 09:05`. Other text stays as it is.
pub(super) fn created_text(created_at: &str) -> String {
    match (
        created_at.get(..10),
        created_at.get(10..11),
        created_at.get(11..16),
    ) {
        (Some(date), Some("T"), Some(time)) => format!("{date} {time}"),
        _ => created_at.to_owned(),
    }
}

/// The guide in front of each run's role, such as `│  └─ `, from the depth of
/// every row in the order they are shown.
pub(super) fn tree_guides(depths: &[usize]) -> Vec<String> {
    (0..depths.len())
        .map(|row| {
            let depth = depths[row];
            (1..=depth)
                .map(
                    |level| match (level == depth, has_later_sibling(depths, row, level)) {
                        (true, true) => "├─ ",
                        (true, false) => "└─ ",
                        (false, true) => "│  ",
                        (false, false) => "   ",
                    },
                )
                .collect()
        })
        .collect()
}

/// Whether a row at `level` comes after `row` before the tree climbs above `level`.
fn has_later_sibling(depths: &[usize], row: usize, level: usize) -> bool {
    depths[row + 1..]
        .iter()
        .take_while(|depth| **depth >= level)
        .any(|depth| *depth == level)
}
