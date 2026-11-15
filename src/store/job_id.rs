//! The job ID: short, safe in a Git branch name, and checked once where it
//! enters rex, so the rest of the code can trust it.

use std::fmt;
use std::str::FromStr;

use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ValueRef};

const LENGTH: usize = 10;

/// A checked job ID: 10 characters of `0-9` and `a-z`, such as `0g4cbf7k2q`.
/// The mod mints each ID (`plugin/hooks/book/ids.ts`): six base-36 characters
/// of seconds since 2026-01-01, then four that do not repeat.
/// It holds no `-`, so a skill reads it back from a job branch name: cut the
/// name at the first `--`, then take the text after the last `-`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct JobId(String);

impl JobId {
    /// The ID as text, as stored in the `job.id` column.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Text that is not a job ID. Used as the value parser error of a `<job>`
/// argument, so it exits as a usage error.
#[derive(thiserror::Error, Debug)]
#[error("{text} is not a job ID; a job ID is 10 lower-case letters and digits, such as 0g4cbf7k2q")]
pub struct JobIdError {
    text: String,
}

impl FromStr for JobId {
    type Err = JobIdError;

    /// # Errors
    /// [`JobIdError`] unless `text` is 10 characters of `0-9` and `a-z`.
    fn from_str(text: &str) -> Result<JobId, JobIdError> {
        let is_valid = text.len() == LENGTH
            && text
                .bytes()
                .all(|byte| byte.is_ascii_digit() || byte.is_ascii_lowercase());
        if !is_valid {
            return Err(JobIdError {
                text: text.to_owned(),
            });
        }
        Ok(JobId(text.to_owned()))
    }
}

/// A stored ID that is not of the job ID form fails the read of its column.
impl FromSql for JobId {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<JobId> {
        let id = value.as_str()?.parse();
        id.map_err(|error: JobIdError| FromSqlError::Other(Box::new(error)))
    }
}

impl fmt::Display for JobId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
