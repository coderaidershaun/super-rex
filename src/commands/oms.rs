//! `rex oms`: reads a role's seven ratings from the command line and prints
//! the grid agent that the selection function picks for them.

use std::path::Path;

use serde::Serialize;

use crate::commands::stdout::print_line_to_stdout;
use crate::error::{CommandError, FailureClass, RexError};
use crate::oms::{Inputs, Level, Selection, VERSION, select};

/// Print the grid agent for a set of inputs.
#[derive(clap::Args)]
#[command(
    after_help = "Prints agent=<grid agent> first, then the demand before caps, the rules that fired, any conflict, the inputs and the version. A rating left out counts as none."
)]
pub struct OmsOptions {
    /// How long the job may take. `fast` means low and `long` means high.
    #[arg(long, value_enum, value_name = "LEVEL")]
    duration: Option<DurationRating>,

    /// How much the job may spend.
    #[arg(long, value_enum, value_name = "LEVEL")]
    budget: Option<Rating>,

    /// How hard the thinking of the role is.
    #[arg(long, value_enum, value_name = "LEVEL")]
    intelligence: Option<Rating>,

    /// How freely agents of the job act without review.
    #[arg(long, value_enum, value_name = "LEVEL")]
    autonomy: Option<Rating>,

    /// How reliable the role's result must be.
    #[arg(long, value_enum, value_name = "LEVEL")]
    reliability: Option<Rating>,

    /// How serious a failure of the role would be.
    #[arg(long, value_enum, value_name = "LEVEL")]
    consequence: Option<Rating>,

    /// How easy it is to check the role's result.
    #[arg(long, value_enum, value_name = "LEVEL")]
    verification: Option<Rating>,

    /// Print the result as one JSON line.
    #[arg(long)]
    json: bool,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum Rating {
    Low,
    Medium,
    High,
    None,
}

impl Rating {
    fn level(self) -> Option<Level> {
        match self {
            Rating::Low => Some(Level::Low),
            Rating::Medium => Some(Level::Medium),
            Rating::High => Some(Level::High),
            Rating::None => None,
        }
    }
}

/// Like [`Rating`], with the words that people use for a job's duration.
#[derive(Clone, Copy, clap::ValueEnum)]
enum DurationRating {
    #[value(alias = "fast")]
    Low,
    Medium,
    #[value(alias = "long")]
    High,
    None,
}

impl DurationRating {
    fn level(self) -> Option<Level> {
        match self {
            DurationRating::Low => Some(Level::Low),
            DurationRating::Medium => Some(Level::Medium),
            DurationRating::High => Some(Level::High),
            DurationRating::None => None,
        }
    }
}

impl OmsOptions {
    fn inputs(&self) -> Inputs {
        let level = |rating: Option<Rating>| rating.and_then(Rating::level);
        Inputs {
            duration: self.duration.and_then(DurationRating::level),
            budget: level(self.budget),
            intelligence: level(self.intelligence),
            autonomy: level(self.autonomy),
            reliability: level(self.reliability),
            consequence: level(self.consequence),
            verification: level(self.verification),
        }
    }
}

/// Failure of `rex oms`. The selection itself always has an answer, so only
/// the output can fail.
#[derive(thiserror::Error, Debug)]
pub enum OmsError {
    #[error("failed to write the selection as JSON")]
    Json {
        #[source]
        source: serde_json::Error,
    },

    #[error(transparent)]
    Rex(#[from] RexError),
}

impl CommandError for OmsError {
    fn failure_class(&self) -> FailureClass {
        match self {
            OmsError::Json { .. } => FailureClass::Reject,
            OmsError::Rex(error) => error.failure_class(),
        }
    }
}

/// The printed result. The fields are in output order, which the JSON form
/// keeps.
#[derive(Serialize)]
struct Output {
    agent: String,
    demand: String,
    rules: Vec<&'static str>,
    conflict: Vec<&'static str>,
    inputs: String,
    version: u32,
}

impl Output {
    fn of(selection: &Selection) -> Output {
        Output {
            agent: selection.agent.to_string(),
            demand: selection.demand.to_string(),
            rules: selection.rules.iter().map(|rule| rule.id()).collect(),
            conflict: selection.conflict.iter().map(|over| over.name()).collect(),
            inputs: selection.inputs.code(),
            version: VERSION,
        }
    }

    fn text(&self) -> String {
        let conflict = if self.conflict.is_empty() {
            "none".to_owned()
        } else {
            self.conflict.join(",")
        };
        [
            format!("agent={}", self.agent),
            format!("demand={}", self.demand),
            format!("rules={}", self.rules.join(",")),
            format!("conflict={conflict}"),
            format!("inputs={}", self.inputs),
            format!("version={}", self.version),
        ]
        .join("\n")
    }
}

/// Print the selection for the given ratings, as six `key=value` lines or,
/// with `--json`, as one JSON line.
///
/// # Errors
/// [`OmsError::Rex`] if standard output cannot be written, and
/// [`OmsError::Json`] if the result cannot be written as JSON.
pub fn run(_cwd: &Path, options: OmsOptions) -> Result<(), OmsError> {
    let output = Output::of(&select(options.inputs()));
    let printed = if options.json {
        serde_json::to_string(&output).map_err(|source| OmsError::Json { source })?
    } else {
        output.text()
    };
    print_line_to_stdout(&printed)?;
    Ok(())
}
