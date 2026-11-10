//! `rex feedback`: records what the user said about a job, the root cause and
//! the lesson, and moves the job's status through the one transition table.

use std::fs;
use std::path::{Path, PathBuf};

use clap::{ArgGroup, ValueEnum};
use rusqlite::{Transaction, params};

use super::stdout::print_line_to_stdout;
use crate::error::{CommandError, RexError};
use crate::store::status::{self, Event, Verdict};
use crate::store::{JobId, Store, StoreError, StorePaths};

/// The longest lesson, in characters, because `rex recall` puts lessons into
/// the builder's context.
const LESSON_LIMIT: usize = 600;

/// Arguments of `rex feedback`.
#[derive(clap::Args)]
#[command(
    group(ArgGroup::new("said_source").required(true).args(["said", "said_file"])),
    group(ArgGroup::new("lesson_source").args(["lesson", "lesson_file"]))
)]
pub struct FeedbackOptions {
    /// The job to give feedback on
    #[arg(value_name = "JOB", value_parser = clap::value_parser!(JobId))]
    pub job: JobId,

    /// What the user decided about the job
    #[arg(long, value_enum)]
    pub verdict: VerdictChoice,

    /// What the user said
    #[arg(long, value_name = "TEXT")]
    pub said: Option<String>,

    /// A file that holds what the user said
    #[arg(long, value_name = "PATH")]
    pub said_file: Option<PathBuf>,

    /// Who gave the feedback
    #[arg(long, value_name = "WHO", default_value = "user")]
    pub by: String,

    /// A root cause; repeat for more than one
    #[arg(long, value_enum, requires = "lesson_source")]
    pub cause: Vec<Cause>,

    /// The lesson, at most 600 characters
    #[arg(long, value_name = "TEXT", requires = "cause")]
    pub lesson: Option<String>,

    /// A file that holds the lesson
    #[arg(long, value_name = "PATH", requires = "cause")]
    pub lesson_file: Option<PathBuf>,

    /// What in the record shows each cause
    #[arg(long, value_name = "TEXT")]
    pub evidence: Option<String>,

    /// A skill the analysis changed, by folder name; repeat for more than one
    #[arg(long, value_name = "NAME", requires = "cause")]
    pub changed_skill: Vec<String>,

    /// An orchestrator the analysis changed, by folder name; repeat for more than one
    #[arg(long, value_name = "NAME", requires = "cause")]
    pub changed_orchestrator: Vec<String>,
}

/// The verdict as the command line spells it.
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerdictChoice {
    Accepted,
    Rejected,
}

/// A root cause. A new one needs no change to the database.
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cause {
    /// The requester was wrong
    HumanError,
    /// The agents needed context the requester did not give
    MissingContext,
    /// Agent error: wrong or missing skills for the orchestration
    AgentSkill,
    /// Agent error: the model or effort was wrong
    AgentModelEffort,
    /// Agent error: the shape of the orchestration was wrong
    AgentOrchestration,
}

#[derive(thiserror::Error, Debug)]
pub enum FeedbackError {
    #[error(
        "--cause, --lesson, --evidence, --changed-skill and --changed-orchestrator need --verdict rejected"
    )]
    AcceptedWithAnalysis,

    #[error("what the user said must not be empty")]
    EmptySaid,

    #[error("the lesson must not be empty")]
    EmptyLesson,

    #[error("the lesson is {chars} characters; the limit is {LESSON_LIMIT}")]
    LessonTooLong { chars: usize },

    #[error("cannot read {}", path.display())]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error(transparent)]
    Store(#[from] StoreError),

    #[error(transparent)]
    Rex(#[from] RexError),
}

impl CommandError for FeedbackError {
    fn is_usage(&self) -> bool {
        matches!(
            self,
            FeedbackError::AcceptedWithAnalysis
                | FeedbackError::EmptySaid
                | FeedbackError::EmptyLesson
                | FeedbackError::LessonTooLong { .. }
        )
    }
}

/// The checked call: every text read, every rule of the command line passed,
/// every list sorted and free of duplicates.
struct Feedback {
    verdict: Verdict,
    given_by: String,
    said: String,
    causes: Vec<String>,
    lesson: Option<String>,
    evidence: Option<String>,
    changed_skills: Vec<String>,
    changed_orchestrators: Vec<String>,
}

/// # Errors
/// - [`FeedbackError::Read`] if a file cannot be read.
/// - [`FeedbackError::AcceptedWithAnalysis`], [`FeedbackError::EmptySaid`],
///   [`FeedbackError::EmptyLesson`] and [`FeedbackError::LessonTooLong`] for a
///   call that breaks a usage rule.
/// - [`FeedbackError::Store`] for a missing store or job, a job with no outcome
///   yet or whose analysis is due, or a failed write.
/// - [`FeedbackError::Rex`] if the output cannot be written.
pub fn run(cwd: &Path, options: FeedbackOptions) -> Result<(), FeedbackError> {
    let job = options.job.clone();
    let feedback = Feedback::check(options)?;

    let paths = StorePaths::discover(cwd)?;
    let mut store = Store::open(&paths)?;
    let (row, change) = store.write(|tx| record(tx, &job, &feedback))?;

    print_line_to_stdout(&format!("job={job}"))?;
    print_line_to_stdout(&format!("feedback={row}"))?;
    print_line_to_stdout(&format!("previous={}", change.previous))?;
    print_line_to_stdout(&format!("status={}", change.current))?;
    Ok(())
}

impl Feedback {
    fn check(options: FeedbackOptions) -> Result<Feedback, FeedbackError> {
        let said = text_of(options.said, options.said_file)?;
        let lesson = match (options.lesson, options.lesson_file) {
            (None, None) => None,
            (text, file) => Some(text_of(text, file)?.unwrap_or_default()),
        };
        let verdict = match options.verdict {
            VerdictChoice::Accepted => Verdict::Accepted,
            VerdictChoice::Rejected => Verdict::Rejected,
        };

        let carries_analysis = !options.cause.is_empty()
            || lesson.is_some()
            || options.evidence.is_some()
            || !options.changed_skill.is_empty()
            || !options.changed_orchestrator.is_empty();
        if verdict == Verdict::Accepted && carries_analysis {
            return Err(FeedbackError::AcceptedWithAnalysis);
        }
        let said = said.unwrap_or_default();
        if said.trim().is_empty() {
            return Err(FeedbackError::EmptySaid);
        }
        if let Some(lesson) = &lesson {
            if lesson.trim().is_empty() {
                return Err(FeedbackError::EmptyLesson);
            }
            let chars = lesson.chars().count();
            if chars > LESSON_LIMIT {
                return Err(FeedbackError::LessonTooLong { chars });
            }
        }

        let causes = options
            .cause
            .iter()
            .filter_map(|cause| cause.to_possible_value())
            .map(|value| value.get_name().to_owned())
            .collect();
        Ok(Feedback {
            verdict,
            given_by: options.by,
            said,
            causes: sorted_unique(causes),
            lesson,
            evidence: options.evidence,
            changed_skills: sorted_unique(options.changed_skill),
            changed_orchestrators: sorted_unique(options.changed_orchestrator),
        })
    }

    /// The call gives a root cause analysis when it names a cause and a lesson.
    fn has_analysis(&self) -> bool {
        !self.causes.is_empty() && self.lesson.is_some()
    }
}

/// The text that came from the command line or from a file. At most one of
/// the two is set; clap enforces it.
fn text_of(text: Option<String>, file: Option<PathBuf>) -> Result<Option<String>, FeedbackError> {
    match file {
        Some(path) => fs::read_to_string(&path)
            .map(Some)
            .map_err(|source| FeedbackError::Read { path, source }),
        None => Ok(text),
    }
}

fn sorted_unique(mut names: Vec<String>) -> Vec<String> {
    names.sort();
    names.dedup();
    names
}

/// `None` for an empty list, so the column is NULL.
fn joined(names: &[String]) -> Option<String> {
    (!names.is_empty()).then(|| names.join(","))
}

/// Moves the status first, so a missing job, a job with no outcome yet and a
/// job whose analysis is due fail before a row is added. A failure anywhere
/// rolls back the whole transaction.
fn record(
    tx: &Transaction<'_>,
    job: &JobId,
    feedback: &Feedback,
) -> Result<(i64, status::Change), StoreError> {
    let change = status::apply(
        tx,
        job,
        Event::Feedback {
            verdict: feedback.verdict,
            has_analysis: feedback.has_analysis(),
        },
    )?;
    // SMELL: the store's `Verdict` has no text form, so the column text is
    // spelled here a second time. Use the store's text once it gives one.
    let verdict = match feedback.verdict {
        Verdict::Accepted => "accepted",
        Verdict::Rejected => "rejected",
    };
    tx.execute(
        "INSERT INTO feedback (job_id, given_by, said, verdict, causes, lesson, evidence,
                               changed_skills, changed_orchestrators, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
        params![
            job.as_str(),
            feedback.given_by,
            feedback.said,
            verdict,
            joined(&feedback.causes),
            feedback.lesson,
            feedback.evidence,
            joined(&feedback.changed_skills),
            joined(&feedback.changed_orchestrators),
        ],
    )?;
    Ok((tx.last_insert_rowid(), change))
}
