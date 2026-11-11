//! The rows that `rex recall` gives, and the lines it prints for them.

use serde::{Serialize, Serializer};

use super::super::token_text::cost_usd;
use crate::capture::{Cost, TokenCounts};
use crate::store::StorePaths;
use crate::store::status::Status;

#[derive(Serialize)]
pub(super) struct Report {
    pub(super) schema: u32,
    pub(super) job: String,
    pub(super) recorded: bool,
    pub(super) lessons: Vec<Lesson>,
    pub(super) orchestrators: Vec<Group>,
    pub(super) unused: Vec<Unused>,
}

#[derive(Serialize)]
pub(super) struct Lesson {
    pub(super) feedback: i64,
    pub(super) job: String,
    pub(super) score: i64,
    pub(super) status: String,
    #[serde(rename = "cost_usd", serialize_with = "usd")]
    pub(super) cost: Option<Cost>,
    pub(super) unpriced_requests: Option<u64>,
    pub(super) cache_hit: Option<u64>,
    pub(super) causes: Vec<String>,
    pub(super) text: String,
}

#[derive(Serialize)]
pub(super) struct Group {
    pub(super) name: String,
    pub(super) version: i64,
    pub(super) best_match: i64,
    pub(super) runs: i64,
    pub(super) accepted: i64,
    pub(super) missed: i64,
    /// The mean over the jobs whose cost is stored, cut, not rounded.
    #[serde(rename = "cost_per_job_usd", serialize_with = "usd")]
    pub(super) cost_per_job: Option<Cost>,
    /// Requests on a model with no price, over all those jobs. Above 0, the
    /// cost is a lower bound.
    pub(super) unpriced_requests: Option<u64>,
    /// Of the tokens of all those jobs together.
    pub(super) cache_hit: Option<u64>,
    pub(super) path: String,
    #[serde(skip)]
    spent: Spent,
}

/// What one job spent: the sum over its main session and its runs, as stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct Spent {
    pub(super) cost: Cost,
    pub(super) tokens: TokenCounts,
    /// The jobs in this sum. A group adds several.
    pub(super) jobs: u64,
}

impl Spent {
    fn add(&mut self, other: Spent) {
        self.cost += other.cost;
        self.tokens += other.tokens;
        self.jobs += other.jobs;
    }

    pub(super) fn cost_per_job(&self) -> Option<Cost> {
        (self.jobs > 0).then(|| Cost {
            nanodollars: self.cost.nanodollars / self.jobs,
            unpriced_requests: self.cost.unpriced_requests,
        })
    }
}

impl Group {
    pub(super) fn empty(paths: &StorePaths, name: String, version: i64) -> Group {
        Group {
            path: skill_path(paths, &name),
            name,
            version,
            best_match: 0,
            runs: 0,
            accepted: 0,
            missed: 0,
            cost_per_job: None,
            unpriced_requests: None,
            cache_hit: None,
            spent: Spent::default(),
        }
    }

    /// Adds one similar job, by its score, stored status, and what it spent
    /// when that is stored. A job that is still open has spent only part of
    /// its cost, so its spend stays out of the mean.
    pub(super) fn count(&mut self, score: i64, status: &str, spent: Option<Spent>) {
        self.best_match = self.best_match.max(score);
        self.runs += 1;
        let status = status.parse();
        match status {
            Ok(Status::Accepted) => self.accepted += 1,
            Ok(Status::Rejected | Status::Analyzed | Status::Failed) => self.missed += 1,
            _ => {}
        }
        let is_open = status.is_ok_and(Status::is_open);
        if let Some(spent) = spent.filter(|_| !is_open) {
            self.spent.add(spent);
            self.cost_per_job = self.spent.cost_per_job();
            self.unpriced_requests = Some(self.spent.cost.unpriced_requests);
            self.cache_hit = self.spent.tokens.cache_hit_percent();
        }
    }

    /// Full costs first, cheapest first. Then lower bounds, which cannot be
    /// compared with a full cost, then groups with no stored cost.
    pub(super) fn cost_rank(&self) -> (u8, u64) {
        match self.cost_per_job {
            Some(cost) if cost.unpriced_requests == 0 => (0, cost.nanodollars),
            Some(_) => (1, 0),
            None => (2, 0),
        }
    }
}

#[derive(Serialize)]
pub(super) struct Unused {
    pub(super) name: String,
    pub(super) version: i64,
    pub(super) path: String,
    pub(super) description: String,
}

pub(super) fn skill_path(paths: &StorePaths, name: &str) -> String {
    paths
        .orchestrators
        .join(name)
        .join("SKILL.md")
        .display()
        .to_string()
}

/// Dollars as a JSON number, as `rex jobs --json` gives them.
fn usd<S: Serializer>(cost: &Option<Cost>, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_some(&cost.map(|cost| cost.dollars()))
}

/// `0.113724`, `≥0.113724` when requests on a model with no price are left
/// out, or `none` when no cost is stored.
fn usd_text(cost: Option<Cost>) -> String {
    let Some(cost) = cost else {
        return "none".to_owned();
    };
    let floor = if cost.unpriced_requests > 0 {
        "≥"
    } else {
        ""
    };
    format!("{floor}{}", cost_usd(&cost))
}

/// `90%`, or `none` when no prompt token is stored.
fn hit_text(percent: Option<u64>) -> String {
    percent.map_or_else(|| "none".to_owned(), |percent| format!("{percent}%"))
}

fn one_line(text: &str) -> String {
    text.replace("\r\n", " ").replace(['\n', '\r'], " ")
}

pub(super) fn plain_lines(report: &Report) -> Vec<String> {
    let mut lines = vec![
        format!("job={}", report.job),
        format!("recorded={}", if report.recorded { "yes" } else { "no" }),
        format!("lessons={}", report.lessons.len()),
    ];
    for lesson in &report.lessons {
        let causes = if lesson.causes.is_empty() {
            "none".to_owned()
        } else {
            lesson.causes.join(",")
        };
        lines.push(format!(
            "lesson feedback={} job={} score={} status={} cost_usd={} cache_hit={} causes={causes} text={}",
            lesson.feedback,
            lesson.job,
            lesson.score,
            lesson.status,
            usd_text(lesson.cost),
            hit_text(lesson.cache_hit),
            one_line(&lesson.text),
        ));
    }
    lines.push(format!("orchestrators={}", report.orchestrators.len()));
    for group in &report.orchestrators {
        lines.push(format!(
            "orchestrator name={} version={} best_match={} runs={} accepted={} missed={} cost_per_job_usd={} cache_hit={} path={}",
            group.name,
            group.version,
            group.best_match,
            group.runs,
            group.accepted,
            group.missed,
            usd_text(group.cost_per_job),
            hit_text(group.cache_hit),
            group.path,
        ));
    }
    lines.push(format!("unused={}", report.unused.len()));
    for unused in &report.unused {
        lines.push(format!(
            "unused name={} version={} path={} description={}",
            unused.name,
            unused.version,
            unused.path,
            one_line(&unused.description),
        ));
    }
    lines
}
