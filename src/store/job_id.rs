//! The job ID: short, safe in a Git branch name, and checked once where it
//! enters rex, so the rest of the code can trust it.

use std::fmt;
use std::str::FromStr;

use super::error::StoreError;

const LENGTH: usize = 10;

/// A checked job ID: 10 characters of `0-9` and `a-z`, such as `0g4cbf7k2q`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct JobId(String);

impl JobId {
    /// The ID as text, as stored in the `job.id` column.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for JobId {
    type Err = StoreError;

    /// # Errors
    /// [`StoreError::InvalidInput`] unless `text` is 10 characters of `0-9` and `a-z`.
    fn from_str(text: &str) -> Result<JobId, StoreError> {
        let is_valid = text.len() == LENGTH
            && text
                .bytes()
                .all(|byte| byte.is_ascii_digit() || byte.is_ascii_lowercase());
        if !is_valid {
            return Err(StoreError::InvalidInput {
                what: format!(
                    "job ID {text:?} is not 10 characters of 0-9 and a-z; give an ID that rex job create printed, such as 0g4cbf7k2q"
                ),
            });
        }
        Ok(JobId(text.to_owned()))
    }
}

impl fmt::Display for JobId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
