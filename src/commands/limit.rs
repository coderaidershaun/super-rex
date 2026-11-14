//! `rex limit`: shows what a job has spent against its limit, and sets a new
//! limit, which is how a person at a shell lets a refused job start sub-agents again.

use std::fmt;
use std::path::Path;
use std::str::FromStr;
use std::time::SystemTime;

use clap::ValueEnum;
use rusqlite::{OptionalExtension, params};

use crate::capture::{CaptureError, Cost, job_runs};
use crate::commands::stdout::print_to_stdout;
use crate::commands::token_text::{cost_text, cost_usd};
use crate::error::{CommandError, FailureClass, RexError};
use crate::oms::Level;
use crate::store::class::Size;
use crate::store::{JobId, Store, StoreError, StorePaths};

/// Print a job's spend and spend limit, or set a new limit.
#[derive(clap::Args)]
#[command(
    after_help = "Prints `limit_usd=`, `spent_usd=` and `source=` (`default` from the job's budget and size, `set` by a person, or `policy`). Spend is the cost of the job's sub-agents and main session at list prices. Once spend reaches the limit, the plugin refuses new sub-agents."
)]
pub struct LimitOptions {
    /// The job whose spend and limit to print.
    pub job: JobId,

    /// The new limit in US dollars, such as 12 or 7.50. It replaces the old one.
    #[arg(long, value_name = "DOLLARS")]
    pub set: Option<Dollars>,

    /// Print one JSON line: job, spent_usd, limit_usd, source.
    #[arg(long)]
    pub json: bool,
}

#[derive(thiserror::Error, Debug)]
pub enum LimitError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Capture(#[from] CaptureError),
    #[error(transparent)]
    Rex(#[from] RexError),
}

impl CommandError for LimitError {
    fn failure_class(&self) -> FailureClass {
        match self {
            LimitError::Store(error) => error.failure_class(),
            LimitError::Capture(error) => error.failure_class(),
            LimitError::Rex(error) => error.failure_class(),
        }
    }

    fn exit_code(&self) -> u8 {
        match self {
            LimitError::Store(error) => error.exit_code(),
            LimitError::Capture(error) => error.exit_code(),
            _ => {
                if self.is_usage() {
                    2
                } else {
                    1
                }
            }
        }
    }
}

/// Sets the limit when `--set` is given, then prints the limit and the spend.
///
/// # Errors
/// - [`StoreError::NotFound`] if the repo has no store.
/// - [`StoreError::JobNotFound`] if the job is not in the store.
/// - [`CaptureError`] if the event log or the store cannot be read.
/// - [`RexError::Stdout`] if the output cannot be written.
pub fn run(cwd: &Path, options: LimitOptions) -> Result<(), LimitError> {
    let paths = StorePaths::discover(cwd)?;
    let mut store = Store::open(&paths)?;
    if let Some(dollars) = options.set {
        SpendLimit::set(&mut store, &options.job, dollars)?;
    }
    let runs = job_runs(&mut store, &options.job, SystemTime::now())?;
    let limit = SpendLimit::of(&store, &options.job)?;
    let spent = runs.cost.unwrap_or_default();
    let limit_cost = Cost {
        nanodollars: limit.nanodollars,
        unpriced_requests: 0,
    };
    if options.json {
        let line = serde_json::json!({
            "job": options.job.as_str(),
            "spent_usd": spent.dollars(),
            "limit_usd": limit_cost.dollars(),
            "source": limit.source.to_string(),
        });
        print_to_stdout(&format!("{line}\n"))?;
        return Ok(());
    }
    print_to_stdout(&format!(
        "limit_usd={}\nspent_usd={}\nsource={}\n",
        cost_usd(&limit_cost),
        cost_usd(&spent),
        limit.source
    ))?;
    Ok(())
}

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
    Policy,
}

impl fmt::Display for LimitSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            LimitSource::Default => "default",
            LimitSource::Set => "set",
            LimitSource::Policy => "policy",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpendLimit {
    pub nanodollars: u64,
    pub source: LimitSource,
}

impl SpendLimit {
    /// The limit of `job`: the one its row holds, else the default of its
    /// budget and size.
    ///
    /// # Errors
    /// - [`StoreError::JobNotFound`] for a job not in the store.
    /// - [`StoreError::Sqlite`] when the read fails, or a stored size, budget or
    ///   source is not a value rex writes.
    pub fn of(store: &Store, job: &JobId) -> Result<SpendLimit, StoreError> {
        type Row = (Option<i64>, Option<String>, Option<String>, Option<String>);
        let row: Option<Row> = store
            .conn()
            .query_row(
                "SELECT limit_nano, limit_source, size, pref_budget FROM job WHERE id = ?1",
                [job.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()
            .map_err(StoreError::from)?;
        let Some((stored, source, size, budget)) = row else {
            return Err(StoreError::JobNotFound {
                job: job.to_string(),
                store: store.paths().dir.clone(),
            });
        };
        if let Some(nanodollars) = stored {
            let nanodollars = u64::try_from(nanodollars)
                .map_err(|error| StoreError::from(conversion_failure(error)))?;
            let source = match source.as_deref() {
                Some("set") => LimitSource::Set,
                Some("policy") => LimitSource::Policy,
                _ => LimitSource::Default,
            };
            return Ok(SpendLimit {
                nanodollars,
                source,
            });
        }
        // A job the plugin left unclassified counts as medium, which is the
        // middle of the caps.
        let size = size
            .map(|text| <Size as ValueEnum>::from_str(&text, false))
            .transpose()
            .map_err(|error| StoreError::from(conversion_failure(Unknown(error))))?
            .unwrap_or(Size::Medium);
        let budget = budget
            .map(|text| <Level as ValueEnum>::from_str(&text, false))
            .transpose()
            .map_err(|error| StoreError::from(conversion_failure(Unknown(error))))?;
        Ok(SpendLimit {
            nanodollars: default_nanodollars(budget, size),
            source: LimitSource::Default,
        })
    }

    /// Stores `dollars` as the limit of `job`, set by a person, in place of the one it had.
    ///
    /// # Errors
    /// - [`StoreError::JobNotFound`] for a job not in the store.
    /// - [`StoreError::Sqlite`] when the write fails.
    pub fn set(store: &mut Store, job: &JobId, dollars: Dollars) -> Result<(), StoreError> {
        let dir = store.paths().dir.clone();
        store.write(|tx| {
            let changed = tx.execute(
                "UPDATE job SET limit_nano = ?2, limit_source = 'set',
                                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 WHERE id = ?1",
                params![job.as_str(), i64::try_from(dollars.0).unwrap_or(i64::MAX)],
            )?;
            if changed == 0 {
                return Err(StoreError::JobNotFound {
                    job: job.to_string(),
                    store: dir,
                });
            }
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
