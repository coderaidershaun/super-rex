//! The spend limit of a job: the default that its stated budget and its size give,
//! or the amount a person set with `rex limit`, kept in the store's `meta` table.

use std::fmt;
use std::str::FromStr;

use clap::ValueEnum;
use rusqlite::{OptionalExtension, params};

use crate::capture::Cost;
use crate::commands::token_text::cost_text;
use crate::oms::Level;
use crate::store::class::Size;
use crate::store::{JobId, Store, StoreError};

const NANODOLLARS_PER_DOLLAR: u64 = 1_000_000_000;

/// From this share of the limit, `rex status` warns.
const WARN_PERCENT: u64 = 80;

/// The most sub-agents the builder plans for a job of each size. The builder skill
/// states the same numbers, and a test holds the two equal.
fn agent_cap(size: Size) -> u64 {
    match size {
        Size::Small => 5,
        Size::Medium => 6,
        Size::Large => 10,
    }
}

/// What one sub-agent may cost at each stated budget, in billionths of a dollar. A
/// job that stated no budget gets the medium amount, so a runaway still stops.
fn nanodollars_per_agent(budget: Option<Level>) -> u64 {
    match budget.unwrap_or(Level::Medium) {
        Level::Low => 1_500_000_000,
        Level::Medium => 4_000_000_000,
        Level::High => 8_000_000_000,
    }
}

/// The main session's spend counts against the limit too. Measured on the jobs in
/// the store, it costs about as much as two sub-agents.
const SESSION_IN_AGENTS: u64 = 2;

/// The limit of a job that no person set: its agent cap plus the main session,
/// times the cost of one agent.
fn default_nanodollars(budget: Option<Level>, size: Size) -> u64 {
    (agent_cap(size) + SESSION_IN_AGENTS) * nanodollars_per_agent(budget)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitSource {
    Default,
    Set,
}

impl fmt::Display for LimitSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            LimitSource::Default => "default",
            LimitSource::Set => "set",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpendLimit {
    pub nanodollars: u64,
    pub source: LimitSource,
}

impl SpendLimit {
    /// The limit of `job`: the amount set for it, else the default of its row.
    ///
    /// # Errors
    /// - [`StoreError::JobNotFound`] for a job not in the store.
    /// - [`StoreError::Sqlite`] when the read fails, or a stored size, budget or
    ///   limit is not a value rex writes.
    pub fn of(store: &Store, job: &JobId) -> Result<SpendLimit, StoreError> {
        let set: Option<String> = store
            .conn()
            .query_row(
                "SELECT value FROM meta WHERE key = ?1",
                [meta_key(job)],
                |row| row.get(0),
            )
            .optional()
            .map_err(StoreError::from)?;
        if let Some(text) = set {
            let nanodollars = text
                .parse()
                .map_err(|error| StoreError::from(conversion_failure(error)))?;
            return Ok(SpendLimit {
                nanodollars,
                source: LimitSource::Set,
            });
        }
        let row: Option<(String, Option<String>)> = store
            .conn()
            .query_row(
                "SELECT size, pref_budget FROM job WHERE id = ?1",
                [job.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(StoreError::from)?;
        let Some((size, budget)) = row else {
            return Err(StoreError::JobNotFound {
                job: job.to_string(),
                store: store.paths().dir.clone(),
            });
        };
        let size = <Size as ValueEnum>::from_str(&size, false)
            .map_err(|error| StoreError::from(conversion_failure(Unknown(error))))?;
        let budget = budget
            .map(|text| <Level as ValueEnum>::from_str(&text, false))
            .transpose()
            .map_err(|error| StoreError::from(conversion_failure(Unknown(error))))?;
        Ok(SpendLimit {
            nanodollars: default_nanodollars(budget, size),
            source: LimitSource::Default,
        })
    }

    /// Stores `dollars` as the limit of `job`, in place of the one it had.
    ///
    /// # Errors
    /// - [`StoreError::JobNotFound`] for a job not in the store.
    /// - [`StoreError::Sqlite`] when the write fails.
    pub fn set(store: &mut Store, job: &JobId, dollars: Dollars) -> Result<(), StoreError> {
        let dir = store.paths().dir.clone();
        store.write(|tx| {
            let is_known = tx
                .query_row(
                    "SELECT 1 FROM job WHERE id = ?1",
                    [job.as_str()],
                    |_| Ok(()),
                )
                .optional()?
                .is_some();
            if !is_known {
                return Err(StoreError::JobNotFound {
                    job: job.to_string(),
                    store: dir,
                });
            }
            tx.execute(
                "INSERT INTO meta (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![meta_key(job), dollars.0.to_string()],
            )?;
            Ok(())
        })
    }

    pub fn is_reached(&self, spent: &Cost) -> bool {
        spent.nanodollars >= self.nanodollars
    }

    /// The text the gate gives the agent whose new sub-agent it refuses.
    pub fn refusal(&self, job: &JobId, spent: &Cost) -> String {
        let (spent, limit) = (cost_text(spent), self.text());
        format!(
            "SuperRex spend limit reached: job {job} has spent {spent} of its {limit} limit, \
             so this agent was not started. Start no more agents. If you are a sub-agent, report \
             the state blocked to the agent that started you, with the spend ({spent}), the limit \
             ({limit}) and the raise command below. If you are the main session, keep the job \
             open: tell the requester the spend, the limit and the raise command, and go on with \
             the same job once they have run it. Close the job blocked only if they decline. \
             Raise command: {}",
            self.raise_command(job)
        )
    }

    /// The line `rex status` prints from 80% of the limit. `None` below it.
    pub fn warning(&self, job: &JobId, spent: &Cost) -> Option<String> {
        let percent = u128::from(spent.nanodollars) * 100 / u128::from(self.nanodollars.max(1));
        if percent < u128::from(WARN_PERCENT) {
            return None;
        }
        let effect = if self.is_reached(spent) {
            "new sub-agents are refused"
        } else {
            "at 100% new sub-agents are refused"
        };
        Some(format!(
            "warning: spend is {percent}% of the {} limit; {effect}; raise it with {}",
            self.text(),
            self.raise_command(job)
        ))
    }

    /// `rex limit <job> --set <twice the limit>`, in whole cents rounded up.
    fn raise_command(&self, job: &JobId) -> String {
        let cents = self.nanodollars.saturating_mul(2).div_ceil(10_000_000);
        let amount = if cents.is_multiple_of(100) {
            format!("{}", cents / 100)
        } else {
            format!("{}.{:02}", cents / 100, cents % 100)
        };
        format!("rex limit {job} --set {amount}")
    }

    /// The limit as money. A spend figure from $100 drops its cents, but a limit is
    /// an amount a person typed, so it keeps them.
    fn text(&self) -> String {
        let cents = self.nanodollars / 10_000_000;
        if cents >= 10_000 {
            return format!("${}.{:02}", cents / 100, cents % 100);
        }
        cost_text(&Cost {
            nanodollars: self.nanodollars,
            unpriced_requests: 0,
        })
    }
}

fn meta_key(job: &JobId) -> String {
    format!("limit.{job}")
}

fn conversion_failure(error: impl std::error::Error + Send + Sync + 'static) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}

/// A stored value that is not one of the names rex writes.
#[derive(thiserror::Error, Debug)]
#[error("unknown stored value: {0}")]
struct Unknown(String);

/// A positive amount of US dollars, in billionths of a dollar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dollars(pub u64);

#[derive(thiserror::Error, Debug, PartialEq, Eq)]
#[error("expected a positive amount of US dollars such as 12 or 7.50, got {text:?}")]
pub struct DollarsError {
    text: String,
}

impl FromStr for Dollars {
    type Err = DollarsError;

    /// Whole dollars and up to nine decimals, read as digits so no amount is rounded.
    fn from_str(text: &str) -> Result<Dollars, DollarsError> {
        let error = || DollarsError {
            text: text.to_owned(),
        };
        let amount = text.strip_prefix('$').unwrap_or(text);
        let (whole, fraction) = amount.split_once('.').unwrap_or((amount, ""));
        let is_digits = |part: &str| part.bytes().all(|byte| byte.is_ascii_digit());
        if whole.is_empty() || !is_digits(whole) || !is_digits(fraction) || fraction.len() > 9 {
            return Err(error());
        }
        let whole: u64 = whole.parse().map_err(|_| error())?;
        let fraction: u64 = format!("{fraction:0<9}").parse().map_err(|_| error())?;
        let nanodollars = whole
            .checked_mul(NANODOLLARS_PER_DOLLAR)
            .and_then(|value| value.checked_add(fraction))
            .filter(|value| *value > 0)
            .ok_or_else(error)?;
        Ok(Dollars(nanodollars))
    }
}
