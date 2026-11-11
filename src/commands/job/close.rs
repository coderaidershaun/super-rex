//! `rex job close`: records how the job ended, with one delivery for each
//! repository it changed, and moves the job to its closed status.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use rusqlite::{OptionalExtension, Transaction, params};
use serde::Deserialize;
use serde_json::json;

use super::JobError;
use super::input::read_text;
use super::missed::SkillCheck;
use super::value::value_name;
use crate::capture;
use crate::commands::stdout::print_to_stdout;
use crate::store::status::{self, Closed, Event};
use crate::store::{JobId, Store, StoreError, StorePaths};

/// Arguments of `rex job close`.
#[derive(clap::Args)]
#[command(
    after_help = "--status delivered is refused while any delivery of the job is blocked, also one kept from an earlier close. A later close of a blocked job updates the repositories it names and keeps the others."
)]
pub struct CloseOptions {
    /// The job to close.
    job: JobId,

    /// The status to give.
    #[arg(long, value_enum)]
    status: CloseStatus,

    /// What happened, in one line.
    #[arg(long, value_parser = one_line)]
    outcome: String,

    /// One entry per repository the job changed, as JSON, or - for standard
    /// input. Leave it out when no repository changed.
    #[arg(long, value_name = "PATH")]
    deliveries_file: Option<PathBuf>,

    /// Print the result as one JSON line.
    #[arg(long)]
    json: bool,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum CloseStatus {
    Delivered,
    Blocked,
    Failed,
    Cancelled,
}

/// One repository's delivery, as the deliveries file gives it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    repo: String,
    branch: Option<String>,
    pr: Option<String>,
    merge_agreed: Option<bool>,
    merge_commit: Option<String>,
    outcome: Outcome,
    note: Option<String>,
}

/// How one repository's work ended.
#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum Outcome {
    /// One squash commit is on main.
    Merged,
    /// No merge was agreed, and the pull request waits for a person.
    PrOpen,
    /// No merge was agreed, and no pull request is possible, so the branch is kept.
    KeptLocal,
    /// A merge was agreed but could not be completed; the note says what the
    /// user must do.
    Blocked,
}

/// A delivery checked and resolved to its repository's main checkout.
struct Delivery {
    root: PathBuf,
    entry: Entry,
    is_agreed: bool,
}

/// # Errors
/// - [`JobError::ReadFile`], [`JobError::InvalidFile`] and the delivery
///   variants for a deliveries file that cannot be read or is not valid.
/// - [`JobError::DeliveredWhileBlocked`] for `--status delivered` while a
///   delivery is blocked.
/// - [`JobError::Store`]: the job is not found, is already closed, or the store fails.
pub(super) fn run(cwd: &Path, options: CloseOptions) -> Result<(), JobError> {
    let mut store = Store::open(&StorePaths::discover(cwd)?)?;
    let view = capture::job_runs(&mut store, &options.job, SystemTime::now())?;

    let job_agreed: Option<bool> = store
        .conn()
        .query_row(
            "SELECT merge_agreed FROM job WHERE id = ?1",
            [options.job.as_str()],
            |row| row.get(0),
        )
        .optional()?;
    let job_agreed = job_agreed.ok_or_else(|| StoreError::JobNotFound {
        job: options.job.to_string(),
        store: store.paths().dir.clone(),
    })?;
    let deliveries = match &options.deliveries_file {
        Some(path) => read_deliveries(cwd, path, job_agreed)?,
        None => Vec::new(),
    };

    let recorded = store.write(|tx| record(tx, &options, &deliveries))?;
    let check = SkillCheck::read(store.conn(), &options.job, &view)?;
    print_to_stdout(&output_text(&options, &recorded, &check))?;
    Ok(())
}

/// The answer of the close, from every delivery of the job as (outcome, repo),
/// and the skills that the roles' runs never opened.
fn output_text(
    options: &CloseOptions,
    recorded: &[(String, String)],
    check: &SkillCheck,
) -> String {
    let status = value_name(options.status);
    if options.json {
        let deliveries: Vec<_> = recorded
            .iter()
            .map(|(outcome, repo)| json!({"outcome": outcome, "repo": repo}))
            .collect();
        let missed: Vec<_> = check
            .missed
            .iter()
            .map(|missed| json!({"role": missed.role, "skill": missed.skill}))
            .collect();
        let line = json!({
            "schema": 1,
            "job": options.job.as_str(),
            "status": status,
            "deliveries": deliveries,
            "missed_skills": missed,
            "roles_not_run": check.roles_not_run,
            "missed_skills_incomplete": check.is_incomplete,
        });
        return format!("{line}\n");
    }
    let mut text = format!(
        "job={}\nstatus={status}\ndeliveries={}\n",
        options.job,
        recorded.len()
    );
    for (number, (outcome, repo)) in recorded.iter().enumerate() {
        text.push_str(&format!("delivery.{}={outcome} {repo}\n", number + 1));
    }
    if check.has_assigned {
        text.push_str(&format!("missed_skills={}\n", check.missed.len()));
    }
    for (number, missed) in check.missed.iter().enumerate() {
        text.push_str(&format!(
            "missed_skill.{}={} {}\n",
            number + 1,
            missed.role,
            missed.skill
        ));
    }
    for (number, role) in check.roles_not_run.iter().enumerate() {
        text.push_str(&format!("role_not_run.{}={role}\n", number + 1));
    }
    if check.is_incomplete {
        text.push_str("missed_skills_incomplete=yes\n");
    }
    text
}

/// The writes of `rex job close`, all in the caller's one transaction. Gives
/// every delivery of the job after the call, as (outcome, repo), in the order
/// they were first recorded.
fn record(
    tx: &Transaction<'_>,
    options: &CloseOptions,
    deliveries: &[Delivery],
) -> Result<Vec<(String, String)>, JobError> {
    let job = options.job.as_str();
    // The status check comes first, so a job that cannot be closed this way
    // gets that refusal and its fix, not a delivery error. On any error below
    // the whole write is rolled back.
    status::apply(tx, &options.job, Event::Close(options.status.closed()))?;
    // A close always writes its status row, even blocked to blocked, so this is
    // the time of that row, and every row of this close gets the same time.
    let now: String = tx.query_row("SELECT updated_at FROM job WHERE id = ?1", [job], |row| {
        row.get(0)
    })?;
    for delivery in deliveries {
        upsert_delivery(tx, job, delivery, &now)?;
    }

    // Rows kept from an earlier close count too: a blocked repository keeps the
    // job from being delivered until a close records its new outcome.
    if matches!(options.status, CloseStatus::Delivered) {
        let blocked: Option<String> = tx
            .query_row(
                "SELECT repo FROM delivery WHERE job_id = ?1 AND outcome = 'blocked'
                 ORDER BY id LIMIT 1",
                [job],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(repo) = blocked {
            return Err(JobError::DeliveredWhileBlocked {
                job: job.to_owned(),
                repo,
            });
        }
    }
    tx.execute(
        "UPDATE job SET outcome = ?1, updated_at = ?2 WHERE id = ?3",
        params![options.outcome, now, job],
    )?;

    let mut statement =
        tx.prepare("SELECT outcome, repo FROM delivery WHERE job_id = ?1 ORDER BY id")?;
    let recorded = statement
        .query_map([job], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(recorded)
}

/// Writes one delivery. An upsert, never INSERT OR REPLACE, so a row recorded
/// by an earlier close keeps its id and with it its place in the list.
fn upsert_delivery(
    tx: &Transaction<'_>,
    job: &str,
    delivery: &Delivery,
    now: &str,
) -> rusqlite::Result<()> {
    let entry = &delivery.entry;
    tx.execute(
        "INSERT INTO delivery (job_id, repo, branch, pr, merge_agreed, merge_commit, outcome,
                               note, recorded_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         ON CONFLICT(job_id, repo) DO UPDATE SET
             branch = excluded.branch, pr = excluded.pr,
             merge_agreed = excluded.merge_agreed, merge_commit = excluded.merge_commit,
             outcome = excluded.outcome, note = excluded.note,
             recorded_at = excluded.recorded_at",
        params![
            job,
            delivery.root.to_string_lossy(),
            entry.branch,
            entry.pr,
            delivery.is_agreed,
            entry.merge_commit,
            entry.outcome.name(),
            entry.note,
            now,
        ],
    )?;
    Ok(())
}

/// Reads the deliveries file and checks each entry by the rules of a delivery.
fn read_deliveries(cwd: &Path, path: &Path, job_agreed: bool) -> Result<Vec<Delivery>, JobError> {
    let text = read_text(cwd, path).map_err(|source| JobError::ReadFile {
        file: "deliveries",
        path: path.to_path_buf(),
        source,
    })?;
    let entries: Vec<Entry> =
        serde_json::from_str(&text).map_err(|source| JobError::InvalidFile {
            file: "deliveries",
            path: path.to_path_buf(),
            source,
        })?;

    let mut deliveries: Vec<Delivery> = Vec::with_capacity(entries.len());
    for entry in entries {
        refuse_none_text(&entry, path)?;
        let root = repo_root(cwd, &entry, path)?;
        if deliveries.iter().any(|delivery| delivery.root == root) {
            return Err(JobError::RepoTwice {
                path: path.to_path_buf(),
                root,
            });
        }
        let is_agreed = entry.merge_agreed.unwrap_or(job_agreed);
        check_rules(&root, &entry, is_agreed)?;
        deliveries.push(Delivery {
            root,
            entry,
            is_agreed,
        });
    }
    Ok(deliveries)
}

/// A missing value is `null` or left out, never the word `none`.
fn refuse_none_text(entry: &Entry, path: &Path) -> Result<(), JobError> {
    let none_key = [
        ("pr", &entry.pr),
        ("merge_commit", &entry.merge_commit),
        ("note", &entry.note),
    ]
    .into_iter()
    .find_map(|(key, value)| (value.as_deref() == Some("none")).then_some(key));
    match none_key {
        Some(key) => Err(JobError::NoneText {
            key,
            repo: entry.repo.clone(),
            path: path.to_path_buf(),
        }),
        None => Ok(()),
    }
}

/// The main checkout of the entry's repository, which is what the delivery stores.
fn repo_root(cwd: &Path, entry: &Entry, path: &Path) -> Result<PathBuf, JobError> {
    match StorePaths::locate(&cwd.join(&entry.repo)) {
        Ok(paths) => Ok(paths.root),
        Err(StoreError::NotInGitRepo { .. }) => Err(JobError::RepoNotInGit {
            given: entry.repo.clone(),
            path: path.to_path_buf(),
        }),
        Err(error) => Err(error.into()),
    }
}

/// The rules that tie a delivery's outcome to its other fields.
fn check_rules(root: &Path, entry: &Entry, is_agreed: bool) -> Result<(), JobError> {
    let root = root.to_path_buf();
    let outcome = entry.outcome;
    let agreement_fits = match outcome {
        Outcome::Merged | Outcome::Blocked => is_agreed,
        Outcome::PrOpen | Outcome::KeptLocal => !is_agreed,
    };
    if outcome == Outcome::Merged && !is_agreed {
        return Err(JobError::MergedNotAgreed { root });
    }
    if !agreement_fits {
        return Err(JobError::AgreementMismatch {
            root,
            outcome: outcome.name(),
            is_agreed,
        });
    }
    if outcome == Outcome::Merged && entry.merge_commit.is_none() {
        return Err(JobError::MergedNoCommit { root });
    }
    if outcome == Outcome::PrOpen && entry.pr.is_none() {
        return Err(JobError::PrOpenNoPr { root });
    }
    if outcome != Outcome::Merged && entry.note.is_none() {
        return Err(JobError::NoNote {
            root,
            outcome: outcome.name(),
        });
    }
    Ok(())
}

/// An `--outcome` that the value parser refuses; clap prints it as a usage error.
#[derive(thiserror::Error, Debug)]
enum OutcomeError {
    #[error("--outcome must be one line; put the details in each delivery's \"note\"")]
    NotOneLine,
    #[error("--outcome must not be empty; say in one line what happened")]
    Empty,
}

/// The value parser of `--outcome`: one line, not empty.
fn one_line(text: &str) -> Result<String, OutcomeError> {
    if text.contains(['\n', '\r']) {
        return Err(OutcomeError::NotOneLine);
    }
    if text.trim().is_empty() {
        return Err(OutcomeError::Empty);
    }
    Ok(text.to_owned())
}

impl CloseStatus {
    fn closed(self) -> Closed {
        match self {
            CloseStatus::Delivered => Closed::Delivered,
            CloseStatus::Blocked => Closed::Blocked,
            CloseStatus::Failed => Closed::Failed,
            CloseStatus::Cancelled => Closed::Cancelled,
        }
    }
}

impl Outcome {
    fn name(self) -> &'static str {
        match self {
            Outcome::Merged => "merged",
            Outcome::PrOpen => "pr-open",
            Outcome::KeptLocal => "kept-local",
            Outcome::Blocked => "blocked",
        }
    }
}
