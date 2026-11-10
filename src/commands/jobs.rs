//! `rex jobs`: lists jobs for scripts and agents. This file holds the data
//! side: the filter, the SELECTs, the summary of one job and its two output
//! forms. `rex watch` builds its job list from the same summary, so the screen
//! and `rex jobs --json` cannot drift apart.

use std::path::Path;

use rusqlite::{Connection, params_from_iter};
use serde::Serialize;

use crate::capture::{JobRuns, TokenCounts, TokenTotal};
use crate::error::{CommandError, RexError};
use crate::store::{JobId, StoreError};

/// How many characters of the request a summary keeps.
pub const EXCERPT_CHARS: usize = 200;

/// Arguments of `rex jobs`.
#[derive(clap::Args)]
pub struct JobsOptions {
    /// Print one JSON document instead of one line per job
    #[arg(long)]
    pub json: bool,

    /// Keep jobs whose request contains this text, ignoring ASCII case; repeat to require several
    #[arg(long, value_name = "TEXT")]
    pub search: Vec<String>,

    /// Keep jobs created on or after this UTC date, as YYYY-MM-DD
    #[arg(long, value_name = "DATE")]
    pub since: Option<String>,

    /// Keep jobs created on or before this UTC date, as YYYY-MM-DD
    #[arg(long, value_name = "DATE")]
    pub until: Option<String>,
}

#[derive(thiserror::Error, Debug)]
pub enum JobsError {
    #[error("--{flag} needs a date as YYYY-MM-DD, not {text}")]
    BadDate { flag: &'static str, text: String },

    #[error("cannot write the jobs list as JSON")]
    Json(#[source] serde_json::Error),

    #[error(transparent)]
    Store(#[from] StoreError),

    #[error(transparent)]
    Rex(#[from] RexError),
}

impl CommandError for JobsError {
    fn is_usage(&self) -> bool {
        matches!(self, JobsError::BadDate { .. })
    }
}

/// # Errors
/// [`RexError::NotBuilt`] until the command is built.
pub fn run(_cwd: &Path, _options: JobsOptions) -> Result<(), JobsError> {
    Err(RexError::NotBuilt { command: "jobs" }.into())
}

/// The checked filter of `rex jobs`.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct JobFilter {
    pub search: Vec<String>,
    pub since: Option<String>,
    pub until: Option<String>,
}

impl JobsOptions {
    /// # Errors
    /// [`JobsError::BadDate`] if `--since` or `--until` is not a `YYYY-MM-DD` date.
    pub fn filter(&self) -> Result<JobFilter, JobsError> {
        Ok(JobFilter {
            search: self.search.clone(),
            since: self
                .since
                .as_deref()
                .map(|text| checked_date("since", text))
                .transpose()?,
            until: self
                .until
                .as_deref()
                .map(|text| checked_date("until", text))
                .transpose()?,
        })
    }
}

fn checked_date(flag: &'static str, text: &str) -> Result<String, JobsError> {
    if is_date(text) {
        Ok(text.to_owned())
    } else {
        Err(JobsError::BadDate {
            flag,
            text: text.to_owned(),
        })
    }
}

/// `YYYY-MM-DD` with a month of 01 to 12 and a day of 01 to 31. A day such as
/// 31 February passes: dates are compared as text, so it is harmless.
fn is_date(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    let digits = |range: std::ops::Range<usize>| -> Option<u32> {
        let part = &bytes[range];
        if part.iter().all(u8::is_ascii_digit) {
            part.iter()
                .try_fold(0_u32, |sum, d| Some(sum * 10 + u32::from(d - b'0')))
        } else {
            None
        }
    };
    let (Some(_), Some(month), Some(day)) = (digits(0..4), digits(5..7), digits(8..10)) else {
        return false;
    };
    (1..=12).contains(&month) && (1..=31).contains(&day)
}

impl JobFilter {
    /// The SELECT for this filter and its parameters, in order. Newest job first;
    /// the ID breaks a tie so the order is stable.
    pub fn sql(&self) -> (String, Vec<String>) {
        let mut sql = String::from(
            "SELECT id, created_at, updated_at, status, request, asked_by, class_version, \
             work_type, size, consequence, languages, orchestrator_name, orchestrator_version, \
             merge_agreed, builder_model, builder_effort FROM job WHERE 1 = 1",
        );
        let mut params = Vec::new();
        for term in &self.search {
            sql.push_str(" AND instr(lower(request), lower(?)) > 0");
            params.push(term.clone());
        }
        if let Some(since) = &self.since {
            sql.push_str(" AND substr(created_at, 1, 10) >= ?");
            params.push(since.clone());
        }
        if let Some(until) = &self.until {
            sql.push_str(" AND substr(created_at, 1, 10) <= ?");
            params.push(until.clone());
        }
        sql.push_str(" ORDER BY created_at DESC, id DESC");
        (sql, params)
    }
}

/// The columns of one job that the summary needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobRow {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub stored_status: String,
    pub request: String,
    pub asked_by: String,
    pub class_version: i64,
    pub work_type: String,
    pub size: String,
    pub consequence: String,
    /// Sorted names joined by commas, or `none`.
    pub languages: String,
    pub orchestrator: Option<(String, i64)>,
    pub merge_agreed: bool,
    pub builder_model: Option<String>,
    pub builder_effort: Option<String>,
}

/// One row of the `delivery` table, in the columns the summary shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveryRow {
    pub repo: String,
    pub branch: Option<String>,
    pub pr: Option<String>,
    pub outcome: String,
}

fn store_error(source: rusqlite::Error) -> StoreError {
    StoreError::Sqlite { path: None, source }
}

/// The jobs that match `filter`, newest first.
///
/// # Errors
/// [`StoreError::Sqlite`] if SQLite fails.
pub fn select_job_rows(conn: &Connection, filter: &JobFilter) -> Result<Vec<JobRow>, StoreError> {
    let (sql, params) = filter.sql();
    let mut statement = conn.prepare(&sql).map_err(store_error)?;
    let rows = statement
        .query_map(params_from_iter(params.iter()), |row| {
            let orchestrator_name: Option<String> = row.get(11)?;
            let orchestrator_version: Option<i64> = row.get(12)?;
            Ok(JobRow {
                id: row.get(0)?,
                created_at: row.get(1)?,
                updated_at: row.get(2)?,
                stored_status: row.get(3)?,
                request: row.get(4)?,
                asked_by: row.get(5)?,
                class_version: row.get(6)?,
                work_type: row.get(7)?,
                size: row.get(8)?,
                consequence: row.get(9)?,
                languages: row.get(10)?,
                orchestrator: orchestrator_name.zip(orchestrator_version),
                merge_agreed: row.get::<_, i64>(13)? != 0,
                builder_model: row.get(14)?,
                builder_effort: row.get(15)?,
            })
        })
        .map_err(store_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(store_error)
}

/// The deliveries of one job, in the order they were first recorded.
///
/// # Errors
/// [`StoreError::Sqlite`] if SQLite fails.
pub fn select_deliveries(conn: &Connection, job: &JobId) -> Result<Vec<DeliveryRow>, StoreError> {
    let mut statement = conn
        .prepare("SELECT repo, branch, pr, outcome FROM delivery WHERE job_id = ? ORDER BY id")
        .map_err(store_error)?;
    let rows = statement
        .query_map([job.as_str()], |row| {
            Ok(DeliveryRow {
                repo: row.get(0)?,
                branch: row.get(1)?,
                pr: row.get(2)?,
                outcome: row.get(3)?,
            })
        })
        .map_err(store_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(store_error)
}

/// Whether the job has at least one feedback row.
///
/// # Errors
/// [`StoreError::Sqlite`] if SQLite fails.
pub fn has_feedback(conn: &Connection, job: &JobId) -> Result<bool, StoreError> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM feedback WHERE job_id = ?)",
        [job.as_str()],
        |row| row.get(0),
    )
    .map_err(store_error)
}

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
    /// `shown_status` is the stored status, or `running` when the caller worked
    /// that out from the open runs.
    pub fn new(
        row: JobRow,
        deliveries: Vec<DeliveryRow>,
        runs: &JobRuns,
        has_feedback: bool,
        shown_status: &str,
    ) -> JobSummary {
        let request_truncated = row.request.chars().nth(EXCERPT_CHARS).is_some();
        JobSummary {
            id: row.id,
            created_at: row.created_at,
            updated_at: row.updated_at,
            status: shown_status.to_owned(),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../store/schema.sql"))
            .unwrap();
        conn
    }

    fn insert(conn: &Connection, id: &str, created_at: &str, request: &str) {
        conn.execute(
            "INSERT INTO job (id, request, asked_by, class_version, work_type, size, consequence, \
             languages, merge_agreed, session_id, status, created_at, updated_at) \
             VALUES (?1, ?2, 'human', 1, 'fix', 'small', 'low', 'go,rust', 0, ?1, 'created', ?3, ?3)",
            (id, request, created_at),
        )
        .unwrap();
    }

    fn empty_runs() -> JobRuns {
        JobRuns {
            runs: vec![],
            open: 0,
            finished: 0,
            unconfirmed: 0,
            tokens: TokenTotal::Unknown,
            session_tokens: TokenTotal::Unknown,
            incomplete: false,
            damaged_log_lines: 0,
        }
    }

    #[test]
    fn filter_keeps_matching_jobs_newest_first() {
        let conn = store();
        insert(
            &conn,
            "0000000001",
            "2026-11-09T10:00:00.000Z",
            "Add a JSON flag",
        );
        insert(
            &conn,
            "0000000002",
            "2026-11-10T10:00:00.000Z",
            "Fix json output\nof status",
        );
        insert(
            &conn,
            "0000000003",
            "2026-11-10T11:00:00.000Z",
            "Other work",
        );
        let filter = JobFilter {
            search: vec!["json".into(), "OUTPUT".into()],
            since: Some("2026-11-10".into()),
            until: Some("2026-11-10".into()),
        };
        let rows = select_job_rows(&conn, &filter).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "0000000002");
        let all = select_job_rows(&conn, &JobFilter::default()).unwrap();
        let ids: Vec<_> = all.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, ["0000000003", "0000000002", "0000000001"]);
    }

    #[test]
    fn summary_has_the_contract_shape() {
        let conn = store();
        let long = "é".repeat(EXCERPT_CHARS + 5);
        insert(&conn, "0000000001", "2026-11-09T10:00:00.000Z", &long);
        let row = select_job_rows(&conn, &JobFilter::default())
            .unwrap()
            .remove(0);
        let summary = JobSummary::new(row, vec![], &empty_runs(), false, "created");
        assert!(summary.request_truncated);
        assert_eq!(summary.request.chars().count(), EXCERPT_CHARS);
        assert_eq!(summary.languages, ["go", "rust"]);
        let json = jobs_json(&[summary]).unwrap();
        assert!(json.starts_with(r#"{"schema":1,"jobs":[{"id":"0000000001","created_at":"#));
        assert!(json.contains(r#""orchestrator":null,"#));
        assert!(json.contains(r#""runs":{"open":0,"finished":0,"unconfirmed":0},"tokens":null,"incomplete":false,"has_feedback":false}]}"#));
        assert_eq!(jobs_json(&[]).unwrap(), r#"{"schema":1,"jobs":[]}"#);
    }

    #[test]
    fn plain_line_flattens_line_breaks() {
        let conn = store();
        insert(
            &conn,
            "0000000001",
            "2026-11-09T10:00:00.000Z",
            "one\r\ntwo\nthree",
        );
        let row = select_job_rows(&conn, &JobFilter::default())
            .unwrap()
            .remove(0);
        let summary = JobSummary::new(row, vec![], &empty_runs(), false, "running");
        assert_eq!(
            summary.plain_line(),
            "0000000001 running fix small low go,rust 2026-11-09T10:00:00.000Z one two three"
        );
    }

    #[test]
    fn dates_are_checked() {
        for good in ["2026-11-10", "2026-01-31"] {
            assert!(is_date(good), "{good}");
        }
        for bad in [
            "2026-13-01",
            "2026-00-10",
            "2026-11-32",
            "2026-11-00",
            "26-11-10",
            "2026/11/10",
            "2026-1-100",
            "20a6-11-10",
            "",
        ] {
            assert!(!is_date(bad), "{bad}");
        }
    }
}
