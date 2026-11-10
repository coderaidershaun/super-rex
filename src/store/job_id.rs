//! The job ID: short, safe in a Git branch name, and checked once where it
//! enters rex, so the rest of the code can trust it.

use std::fmt;
use std::hash::{BuildHasher, RandomState};
use std::str::FromStr;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use super::error::StoreError;

const LENGTH: usize = 10;
const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";

/// 2026-01-01T00:00:00Z. The first six characters count seconds from here.
const EPOCH_UNIX_SECONDS: i64 = 1_767_225_600;
const SECONDS_WIDTH: u32 = 6;
const RANDOM_WIDTH: u32 = 4;
/// Six base-36 characters hold about 69 years of seconds, so until late 2094.
const SECONDS_LIMIT: u64 = 36_u64.pow(SECONDS_WIDTH);
const RANDOM_LIMIT: u64 = 36_u64.pow(RANDOM_WIDTH);

static COUNTER: AtomicU64 = AtomicU64::new(0);
static SEED: OnceLock<u64> = OnceLock::new();

/// A checked job ID: 10 characters of `0-9` and `a-z`, such as `0g4cbf7k2q`.
/// It holds no `-`, so a skill reads it back from a job branch name: cut the
/// name at the first `--`, then take the text after the last `-`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct JobId(String);

impl JobId {
    /// A new job ID for now. The first six characters are the time, so IDs sort
    /// by when they were made. The last four never repeat within one process
    /// for 1,679,616 calls; across processes they start from a random point, so
    /// a clash is rare and the caller draws again.
    ///
    /// # Errors
    /// [`StoreError::ClockOutOfRange`] if the system clock is before 2026 or
    /// after late 2094.
    pub fn generate() -> Result<JobId, StoreError> {
        let unix_seconds = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(since) => i64::try_from(since.as_secs()).unwrap_or(i64::MAX),
            Err(before) => i64::try_from(before.duration().as_secs()).map_or(i64::MIN, |s| -s),
        };
        let seed = *SEED.get_or_init(|| RandomState::new().hash_one(0_u8));
        let count = COUNTER.fetch_add(1, Ordering::Relaxed);
        JobId::from_parts(unix_seconds, (seed % RANDOM_LIMIT).wrapping_add(count))
    }

    fn from_parts(unix_seconds: i64, random: u64) -> Result<JobId, StoreError> {
        let seconds = unix_seconds
            .checked_sub(EPOCH_UNIX_SECONDS)
            .and_then(|seconds| u64::try_from(seconds).ok())
            .filter(|seconds| *seconds < SECONDS_LIMIT)
            .ok_or(StoreError::ClockOutOfRange { unix_seconds })?;
        let mut id = String::with_capacity(LENGTH);
        push_base36(&mut id, seconds, SECONDS_WIDTH);
        push_base36(&mut id, random % RANDOM_LIMIT, RANDOM_WIDTH);
        Ok(JobId(id))
    }

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

/// Writes `value` as exactly `width` base-36 characters, most significant
/// first and padded with zeros, so that text order is number order.
fn push_base36(out: &mut String, value: u64, width: u32) {
    for place in (0..width).rev() {
        let digit = (value / 36_u64.pow(place)) % 36;
        out.push(char::from(DIGITS[digit as usize]));
    }
}

impl fmt::Display for JobId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_id_is_short_sortable_and_branch_safe() {
        let start = EPOCH_UNIX_SECONDS;
        let last = start + SECONDS_LIMIT as i64 - 1;
        let earlier = JobId::from_parts(start, RANDOM_LIMIT - 1).unwrap();
        let later = JobId::from_parts(start + 1, 0).unwrap();
        let much_later = JobId::from_parts(last, 0).unwrap();

        for id in [&earlier, &later, &much_later] {
            let text = id.to_string();
            assert_eq!(text.len(), 10, "{text}");
            assert!(!text.contains('-'), "{text}");
            assert!(
                text.bytes()
                    .all(|b| b.is_ascii_digit() || b.is_ascii_lowercase()),
                "{text}"
            );
            assert_eq!(text.parse::<JobId>().unwrap(), *id);
        }
        assert!(earlier.to_string() < later.to_string());
        assert!(later.to_string() < much_later.to_string());

        assert!(matches!(
            JobId::from_parts(start - 1, 0),
            Err(StoreError::ClockOutOfRange { .. })
        ));
        assert!(matches!(
            JobId::from_parts(last + 1, 0),
            Err(StoreError::ClockOutOfRange { .. })
        ));
    }
}
