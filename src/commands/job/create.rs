//! `rex job create`: opens a job for the session, or gives back the session's
//! open job, so that all the work of one session is recorded under one job.

use std::io::Write;
use std::path::{Path, PathBuf};

use clap::builder::NonEmptyStringValueParser;
use rusqlite::{Transaction, params};
use serde_json::json;

use super::JobError;
use super::input::{non_empty_var, read_text};
use super::value::value_name;
use crate::commands::stdout::print_to_stdout;
use crate::oms::Level;
use crate::store::class::{Consequence, Size, WorkType};
use crate::store::status::{self, Status};
use crate::store::{JobId, Store, StorePaths};

/// The classification scheme that `rex job create` writes.
const CLASS_VERSION: i64 = 1;

/// A clash of two new job IDs is rare, so a few draws are plenty.
const ID_TRIES: u32 = 5;

/// Arguments of `rex job create`.
#[derive(clap::Args)]
#[command(
    after_help = "Prints job=<id> first, then status= and existing=. A job ID is 10 lower-case letters and digits, such as 0g4cbf7k2q. In a session that already has an open job, no job is opened: that job is printed with existing=yes, and the exit code is 0."
)]
pub struct CreateOptions {
    /// File with the request text, or - for standard input.
    #[arg(long, value_name = "PATH")]
    request_file: PathBuf,

    /// Who made the request.
    #[arg(long, value_enum)]
    asked_by: AskedBy,

    /// Whether a merge to main was agreed.
    #[arg(long, value_enum)]
    merge: Answer,

    /// The kind of work.
    #[arg(long = "type", value_enum, value_name = "TYPE")]
    work_type: WorkType,

    /// How big the job is.
    #[arg(long, value_enum)]
    size: Size,

    /// What a mistake would cost.
    #[arg(long, value_enum, value_name = "LEVEL")]
    consequence: Consequence,

    /// A language of the source files the job changes, or none. Repeat for each.
    #[arg(long = "lang", value_name = "LANG", required = true, value_parser = language)]
    languages: Vec<String>,

    /// The Claude Code session ID [default: $CLAUDE_CODE_SESSION_ID].
    #[arg(long, value_parser = NonEmptyStringValueParser::new())]
    session: Option<String>,

    /// The user's stated duration preference. `fast` means low and `long` means high.
    #[arg(long, value_enum, value_name = "LEVEL")]
    duration: Option<DurationLevel>,

    /// The user's stated budget preference.
    #[arg(long, value_enum, value_name = "LEVEL")]
    budget: Option<Level>,

    /// The user's stated autonomy preference.
    #[arg(long, value_enum, value_name = "LEVEL")]
    autonomy: Option<Level>,

    /// The exact model ID of the session that runs the builder.
    #[arg(long, value_name = "ID", value_parser = NonEmptyStringValueParser::new())]
    builder_model: Option<String>,

    /// The effort level that runs the builder [default: $CLAUDE_EFFORT].
    #[arg(long, value_name = "LEVEL", value_parser = NonEmptyStringValueParser::new())]
    builder_effort: Option<String>,

    /// Print the result as one JSON line.
    #[arg(long)]
    json: bool,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum AskedBy {
    Human,
    Agent,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum Answer {
    Yes,
    No,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum DurationLevel {
    #[value(alias = "fast")]
    Low,
    Medium,
    #[value(alias = "long")]
    High,
}

/// The job this call gives: one it opened, or the session's open job.
struct Given {
    job: JobId,
    status: Status,
    is_existing: bool,
}

/// The fields of the new job row, checked.
struct NewJob {
    request: String,
    session: String,
    languages: String,
    builder_effort: Option<String>,
}

/// # Errors
/// - [`JobError::NoSession`] and [`JobError::LangNoneMixed`] for a wrong command line.
/// - [`JobError::ReadFile`] and [`JobError::EmptyRequest`] for the request file.
/// - [`JobError::Store`] if the store cannot be found, opened or written.
pub(super) fn run(cwd: &Path, options: CreateOptions) -> Result<(), JobError> {
    let session = options
        .session
        .clone()
        .or_else(|| non_empty_var("CLAUDE_CODE_SESSION_ID"))
        .ok_or(JobError::NoSession)?;
    let languages = language_list(&options.languages)?;
    let request = read_text(cwd, &options.request_file).map_err(|source| JobError::ReadFile {
        file: "request",
        path: options.request_file.clone(),
        source,
    })?;
    if request.trim().is_empty() {
        return Err(JobError::EmptyRequest {
            path: options.request_file,
        });
    }
    let new_job = NewJob {
        request,
        session,
        languages,
        builder_effort: options
            .builder_effort
            .clone()
            .or_else(|| non_empty_var("CLAUDE_EFFORT")),
    };

    let mut store = Store::open(&StorePaths::discover(cwd)?)?;
    let given = store.write(|tx| open_or_give_existing(tx, &options, &new_job))?;

    if given.is_existing {
        // If standard error cannot be written to, there is nowhere left to say so.
        let _ = writeln!(
            std::io::stderr(),
            "note: session {} already has open job {}; no job was opened; this work is recorded under it",
            new_job.session,
            given.job
        );
    }
    print_to_stdout(&output_text(&given, options.json))?;
    Ok(())
}

fn output_text(given: &Given, is_json: bool) -> String {
    if is_json {
        let line = json!({
            "schema": 1,
            "job": given.job.as_str(),
            "status": given.status.to_string(),
            "existing": given.is_existing,
        });
        return format!("{line}\n");
    }
    let existing = if given.is_existing { "yes" } else { "no" };
    format!(
        "job={}\nstatus={}\nexisting={existing}\n",
        given.job, given.status
    )
}

/// The session's open job if it has one, else a new job. Both in one write, so
/// two creates at the same moment in one session give one job.
fn open_or_give_existing(
    tx: &Transaction<'_>,
    options: &CreateOptions,
    new_job: &NewJob,
) -> Result<Given, JobError> {
    if let Some((job, status)) = status::open_job(tx, &new_job.session)? {
        return Ok(Given {
            job,
            status,
            is_existing: true,
        });
    }

    let now: String = tx.query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ', 'now')", [], |row| {
        row.get(0)
    })?;
    let job = insert_job(tx, options, new_job, &now)?;
    tx.execute(
        "INSERT INTO job_transition (job_id, from_status, to_status, event, at)
         VALUES (?1, NULL, 'created', 'job create', ?2)",
        params![job.as_str(), now],
    )?;
    Ok(Given {
        job,
        status: Status::Created,
        is_existing: false,
    })
}

/// Inserts the job row under a new ID, drawing again if the ID is taken.
fn insert_job(
    tx: &Transaction<'_>,
    options: &CreateOptions,
    new_job: &NewJob,
    now: &str,
) -> Result<JobId, JobError> {
    let mut tries = 0;
    loop {
        tries += 1;
        let job = JobId::generate()?;
        let inserted = tx.execute(
            "INSERT INTO job (id, request, asked_by, class_version, work_type, size, consequence,
                              languages, pref_duration, pref_budget, pref_autonomy, merge_agreed,
                              session_id, builder_model, builder_effort, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, 'created', ?16, ?16)",
            params![
                job.as_str(),
                new_job.request,
                value_name(options.asked_by),
                CLASS_VERSION,
                options.work_type.to_string(),
                options.size.to_string(),
                options.consequence.to_string(),
                new_job.languages,
                options.duration.map(value_name),
                options.budget.map(value_name),
                options.autonomy.map(value_name),
                matches!(options.merge, Answer::Yes),
                new_job.session,
                options.builder_model,
                new_job.builder_effort,
                now,
            ],
        );
        match inserted {
            Ok(_) => return Ok(job),
            Err(error) if is_id_clash(&error) && tries < ID_TRIES => continue,
            Err(error) => return Err(error.into()),
        }
    }
}

fn is_id_clash(error: &rusqlite::Error) -> bool {
    matches!(
        error,
        rusqlite::Error::SqliteFailure(failure, _)
            if failure.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_PRIMARYKEY
    )
}

/// A `--lang` value that the value parser refuses; clap prints it as a usage error.
#[derive(thiserror::Error, Debug)]
#[error(
    "{text} is not a language name; use lower-case letters and digits only, such as cpp or csharp"
)]
struct LanguageError {
    text: String,
}

/// The value parser of `--lang`: lower-cased, letters and digits only.
fn language(text: &str) -> Result<String, LanguageError> {
    let name = text.to_lowercase();
    let is_name = !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit());
    if is_name {
        Ok(name)
    } else {
        Err(LanguageError {
            text: text.to_owned(),
        })
    }
}

/// The languages as stored: sorted, without duplicates, joined by commas.
fn language_list(languages: &[String]) -> Result<String, JobError> {
    let mut names = languages.to_vec();
    names.sort();
    names.dedup();
    if names.len() > 1 && names.iter().any(|name| name == "none") {
        return Err(JobError::LangNoneMixed);
    }
    Ok(names.join(","))
}
