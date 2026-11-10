//! The jobs filter and its SELECTs: which jobs match, and the rows `rex jobs`
//! and `rex watch` build their summaries from.

use rusqlite::{Connection, params_from_iter};

use super::{JobsError, JobsOptions};
use crate::store::{JobId, StoreError};

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
    /// the ID breaks a tie so the order is stable. There is no limit, because a
    /// script that reads the list expects it to be complete.
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

/// The jobs that match `filter`, newest first.
///
/// # Errors
/// [`StoreError::Sqlite`] if SQLite fails.
pub fn select_job_rows(conn: &Connection, filter: &JobFilter) -> Result<Vec<JobRow>, StoreError> {
    let (sql, params) = filter.sql();
    let mut statement = conn.prepare(&sql)?;
    let rows = statement.query_map(params_from_iter(params.iter()), |row| {
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
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// The deliveries of one job, in the order they were first recorded.
///
/// # Errors
/// [`StoreError::Sqlite`] if SQLite fails.
pub fn select_deliveries(conn: &Connection, job: &JobId) -> Result<Vec<DeliveryRow>, StoreError> {
    let mut statement = conn
        .prepare("SELECT repo, branch, pr, outcome FROM delivery WHERE job_id = ? ORDER BY id")?;
    let rows = statement.query_map([job.as_str()], |row| {
        Ok(DeliveryRow {
            repo: row.get(0)?,
            branch: row.get(1)?,
            pr: row.get(2)?,
            outcome: row.get(3)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Whether the job has at least one feedback row.
///
/// # Errors
/// [`StoreError::Sqlite`] if SQLite fails.
pub fn has_feedback(conn: &Connection, job: &JobId) -> Result<bool, StoreError> {
    let exists = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM feedback WHERE job_id = ?)",
        [job.as_str()],
        |row| row.get(0),
    )?;
    Ok(exists)
}

#[cfg(test)]
mod tests {
    use super::*;

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
