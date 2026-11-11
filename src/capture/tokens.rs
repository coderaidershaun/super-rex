//! Token counts, and the rule that says when a stored total is final. Transcripts are
//! written a little after a run stops, so a total read too early must stay provisional.

use std::ops::AddAssign;
use std::time::{Duration, SystemTime};

/// The four token counts that Claude Code bills for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TokenCounts {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_creation: u64,
}

/// Every call re-reads its cached context and counts it as `cache_read` again, so
/// cache reads dominate a long run. These views keep them apart from the rest.
impl TokenCounts {
    /// The whole percent of prompt tokens served from the cache, cut, not
    /// rounded. `None` when there was no prompt token.
    pub fn cache_hit_percent(&self) -> Option<u64> {
        let prompt = [self.input, self.cache_creation, self.cache_read]
            .into_iter()
            .map(u128::from)
            .sum::<u128>();
        (prompt > 0).then(|| {
            let percent = u128::from(self.cache_read) * 100 / prompt;
            u64::try_from(percent).unwrap_or(100)
        })
    }
}

/// The counts come from files that anyone can write, so a sum stops at the largest
/// count instead of overflowing.
impl AddAssign for TokenCounts {
    fn add_assign(&mut self, other: Self) {
        self.input = self.input.saturating_add(other.input);
        self.output = self.output.saturating_add(other.output);
        self.cache_read = self.cache_read.saturating_add(other.cache_read);
        self.cache_creation = self.cache_creation.saturating_add(other.cache_creation);
    }
}

/// A run's or a session's tokens, as far as they are known.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenTotal {
    /// Nothing was ever read: no transcript was found.
    Unknown,
    /// Read, but the transcript can still grow.
    Provisional(TokenCounts),
    /// Read after the close and found unchanged by a later read. It is never read again.
    Final(TokenCounts),
}

/// How long after a reading made after the close the transcript must keep its size
/// before that reading's total is final. Claude Code writes a transcript's last records
/// a little after the run stops.
pub const SETTLE: Duration = Duration::from_secs(10);

/// What an earlier read of a transcript found, as the store holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reading {
    pub transcript_bytes: u64,
    pub damaged_lines: u64,
    pub read_at: SystemTime,
}

/// Whether the run that writes the transcript can still be working. The main session
/// of a job has no run, so it is checked as `NotOpen`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunActivity {
    Open,
    NotOpen,
}

/// What a read does with the stored total of one transcript.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// The stored total is final from now on. The file is never read again.
    Final,
    /// The stored total stays, still provisional. The file is not parsed.
    Keep,
    /// Parse the file and store the new reading, read at `now`.
    ReadAgain,
}

/// The facts that decide whether a stored total is final, as they stand at `now`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TotalCheck {
    /// `None` when nothing was ever stored.
    pub stored: Option<Reading>,
    /// Size of the transcript now. `None` when there is no file.
    pub transcript_bytes: Option<u64>,
    /// When the job closed. `None` while it is open.
    pub closed_at: Option<SystemTime>,
    pub run: RunActivity,
    pub now: SystemTime,
}

impl TotalCheck {
    /// Decide what to do with the stored total. A total becomes final only when a
    /// reading made after the close is at least [`SETTLE`] old, the file still has the
    /// size it had then, and that reading found no damaged line. A total stored before
    /// the close can never become final, because the run may still have been writing.
    pub fn next_step(&self) -> Step {
        let Some(bytes_now) = self.transcript_bytes else {
            return Step::Keep;
        };
        let Some(stored) = self
            .stored
            .filter(|stored| stored.transcript_bytes == bytes_now)
        else {
            return Step::ReadAgain;
        };
        let Some(closed_at) = self.closed_at else {
            return Step::Keep;
        };
        if self.run == RunActivity::Open {
            return Step::Keep;
        }
        if stored.read_at < closed_at {
            return Step::ReadAgain;
        }
        if stored.damaged_lines > 0 {
            return Step::Keep;
        }
        match self.now.duration_since(stored.read_at) {
            Ok(age) if age >= SETTLE => Step::Final,
            _ => Step::Keep,
        }
    }
}
