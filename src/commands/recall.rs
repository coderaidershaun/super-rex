//! `rex recall`: lessons from similar past jobs, and the orchestrators those
//! jobs used. It also records what it found on the job, so the builder does
//! not have to pass it on.

use std::path::Path;

use rusqlite::{OptionalExtension, Transaction, named_params, params};
use serde::Serialize;

use super::skills;
use super::stdout::print_line_to_stdout;
use crate::error::{CommandError, RexError};
use crate::store::orchestrator;
use crate::store::status::Status;
use crate::store::{JobId, Store, StoreError, StorePaths};

/// A job is similar when at least this many of the four fields are equal.
const MIN_SCORE: i64 = 2;
const MAX_LESSONS: i64 = 5;
const MAX_ORCHESTRATORS: usize = 3;

/// Arguments of `rex recall`.
#[derive(clap::Args)]
pub struct RecallOptions {
    /// The job to recall experience for
    #[arg(value_name = "JOB", value_parser = clap::value_parser!(JobId))]
    pub job: JobId,

    /// Print one JSON document instead of lines
    #[arg(long)]
    pub json: bool,
}

/// Failure of `rex recall`.
#[derive(thiserror::Error, Debug)]
pub enum RecallError {
    #[error("cannot write the recall as JSON")]
    Json(#[source] serde_json::Error),

    #[error(transparent)]
    Store(#[from] StoreError),

    #[error(transparent)]
    Rex(#[from] RexError),
}

impl CommandError for RecallError {}

#[derive(Serialize)]
struct Report {
    schema: u32,
    job: String,
    recorded: bool,
    lessons: Vec<Lesson>,
    orchestrators: Vec<Group>,
    unused: Vec<Unused>,
}

#[derive(Serialize)]
struct Lesson {
    feedback: i64,
    job: String,
    score: i64,
    status: String,
    causes: Vec<String>,
    text: String,
}

#[derive(Serialize)]
struct Group {
    name: String,
    version: i64,
    best_match: i64,
    runs: i64,
    accepted: i64,
    missed: i64,
    path: String,
}

impl Group {
    fn empty(paths: &StorePaths, name: String, version: i64) -> Group {
        Group {
            path: skill_path(paths, &name),
            name,
            version,
            best_match: 0,
            runs: 0,
            accepted: 0,
            missed: 0,
        }
    }

    /// Adds one similar job, by its score and stored status.
    fn count(&mut self, score: i64, status: &str) {
        self.best_match = self.best_match.max(score);
        self.runs += 1;
        match status.parse() {
            Ok(Status::Accepted) => self.accepted += 1,
            Ok(Status::Rejected | Status::Analyzed | Status::Failed) => self.missed += 1,
            _ => {}
        }
    }
}

#[derive(Serialize)]
struct Unused {
    name: String,
    version: i64,
    path: String,
    description: String,
}

/// The four fields of the asked job that the score compares.
struct Asked {
    work_type: String,
    size: String,
    consequence: String,
    languages: String,
}

/// # Errors
/// - [`RecallError::Store`] for a missing store, an unknown job, or a failed
///   read or write, including an orchestrator folder that cannot be read.
/// - [`RecallError::Json`] if the document cannot be written.
/// - [`RecallError::Rex`] if the output cannot be written.
pub fn run(cwd: &Path, options: RecallOptions) -> Result<(), RecallError> {
    let paths = StorePaths::discover(cwd)?;
    let mut store = Store::open(&paths)?;
    let (report, json) = store.write(|tx| recall(tx, &paths, &options.job))?;

    if options.json {
        print_line_to_stdout(&json)?;
    } else {
        for line in plain_lines(&report) {
            print_line_to_stdout(&line)?;
        }
    }
    Ok(())
}

/// Indexes the orchestrator folders, builds the report, and stores it on the
/// job while the job is `created`. The index is written before it is read, so
/// a folder added a minute ago shows at once. A job in any other status still
/// gets a fresh index; only storing the report is skipped.
fn recall(
    tx: &Transaction<'_>,
    paths: &StorePaths,
    job: &JobId,
) -> Result<(Report, String), RecallError> {
    let asked = asked_job(tx, paths, job)?;
    if paths.orchestrators.is_dir() {
        orchestrator::index_all(tx, &paths.orchestrators, &skills::describe)?;
    }

    let mut report = Report {
        schema: 1,
        job: job.to_string(),
        recorded: true,
        lessons: lessons(tx, job, &asked)?,
        orchestrators: groups(tx, paths, job, &asked)?,
        unused: unused(tx, paths)?,
    };
    let stored = to_json(&report)?;
    let changed = tx
        .execute(
            "UPDATE job SET experience_recalled = ?1,
                            updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?2 AND status = 'created'",
            params![stored, job.as_str()],
        )
        .map_err(StoreError::from)?;
    if changed == 1 {
        return Ok((report, stored));
    }
    report.recorded = false;
    let json = to_json(&report)?;
    Ok((report, json))
}

fn to_json(report: &Report) -> Result<String, RecallError> {
    serde_json::to_string(report).map_err(RecallError::Json)
}

fn asked_job(tx: &Transaction<'_>, paths: &StorePaths, job: &JobId) -> Result<Asked, StoreError> {
    let asked = tx
        .query_row(
            "SELECT work_type, size, consequence, languages FROM job WHERE id = ?1",
            [job.as_str()],
            |row| {
                Ok(Asked {
                    work_type: row.get(0)?,
                    size: row.get(1)?,
                    consequence: row.get(2)?,
                    languages: row.get(3)?,
                })
            },
        )
        .optional()?;
    asked.ok_or_else(|| StoreError::JobNotFound {
        job: job.to_string(),
        store: paths.dir.clone(),
    })
}

/// Every other job with its score: one point for each of the four fields that
/// is non-empty and equal to the asked job's.
const SCORED_JOBS: &str = "
    SELECT id, created_at, status, orchestrator_name, orchestrator_version,
           (work_type <> '' AND work_type = :work_type)
         + (size <> '' AND size = :size)
         + (consequence <> '' AND consequence = :consequence)
         + (languages <> '' AND languages = :languages) AS score
    FROM job WHERE id <> :job";

fn lessons(tx: &Transaction<'_>, job: &JobId, asked: &Asked) -> Result<Vec<Lesson>, StoreError> {
    let sql = format!(
        "WITH scored AS ({SCORED_JOBS})
         SELECT f.id, s.id, s.score, s.status, f.causes, f.lesson
         FROM feedback f JOIN scored s ON s.id = f.job_id
         WHERE f.lesson IS NOT NULL AND f.lesson <> '' AND s.score >= :min_score
         ORDER BY s.score DESC, s.created_at DESC, f.id DESC
         LIMIT :limit"
    );
    let mut statement = tx.prepare(&sql)?;
    let rows = statement.query_map(
        named_params! {
            ":work_type": asked.work_type, ":size": asked.size,
            ":consequence": asked.consequence, ":languages": asked.languages,
            ":job": job.as_str(), ":min_score": MIN_SCORE, ":limit": MAX_LESSONS,
        },
        |row| {
            let causes: Option<String> = row.get(4)?;
            Ok(Lesson {
                feedback: row.get(0)?,
                job: row.get(1)?,
                score: row.get(2)?,
                status: row.get(3)?,
                causes: causes
                    .iter()
                    .flat_map(|text| text.split(','))
                    .filter(|cause| !cause.is_empty())
                    .map(str::to_owned)
                    .collect(),
                text: row.get(5)?,
            })
        },
    )?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// The similar jobs that ran an orchestrator, grouped by its name, the best
/// groups first.
fn groups(
    tx: &Transaction<'_>,
    paths: &StorePaths,
    job: &JobId,
    asked: &Asked,
) -> Result<Vec<Group>, StoreError> {
    let sql = format!(
        "WITH scored AS ({SCORED_JOBS})
         SELECT orchestrator_name, orchestrator_version, score, status FROM scored
         WHERE orchestrator_name IS NOT NULL AND score >= :min_score
         ORDER BY created_at DESC, id DESC"
    );
    let mut statement = tx.prepare(&sql)?;
    let rows = statement.query_map(
        named_params! {
            ":work_type": asked.work_type, ":size": asked.size,
            ":consequence": asked.consequence, ":languages": asked.languages,
            ":job": job.as_str(), ":min_score": MIN_SCORE,
        },
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
            ))
        },
    )?;

    // Newest job first, so the first job of a name sets its version.
    let mut groups: Vec<Group> = Vec::new();
    for row in rows {
        let (name, version, score, status) = row?;
        match groups.iter_mut().find(|group| group.name == name) {
            Some(group) => group.count(score, &status),
            None => {
                let mut group = Group::empty(paths, name, version);
                group.count(score, &status);
                groups.push(group);
            }
        }
    }
    groups.sort_by(|a, b| {
        (b.best_match, b.runs)
            .cmp(&(a.best_match, a.runs))
            .then_with(|| a.name.cmp(&b.name))
    });
    groups.truncate(MAX_ORCHESTRATORS);
    Ok(groups)
}

/// Each indexed name whose newest version no job names.
fn unused(tx: &Transaction<'_>, paths: &StorePaths) -> Result<Vec<Unused>, StoreError> {
    let mut statement = tx.prepare(
        "SELECT o.name, o.version, o.description FROM orchestrator o
         WHERE o.version = (SELECT MAX(version) FROM orchestrator WHERE name = o.name)
           AND NOT EXISTS (SELECT 1 FROM job j
                           WHERE j.orchestrator_name = o.name
                             AND j.orchestrator_version = o.version)
         ORDER BY o.name",
    )?;
    let rows = statement.query_map([], |row| {
        let name: String = row.get(0)?;
        Ok(Unused {
            path: skill_path(paths, &name),
            name,
            version: row.get(1)?,
            description: row.get(2)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

fn skill_path(paths: &StorePaths, name: &str) -> String {
    paths
        .orchestrators
        .join(name)
        .join("SKILL.md")
        .display()
        .to_string()
}

fn one_line(text: &str) -> String {
    text.replace("\r\n", " ").replace(['\n', '\r'], " ")
}

fn plain_lines(report: &Report) -> Vec<String> {
    let mut lines = vec![
        format!("job={}", report.job),
        format!("recorded={}", if report.recorded { "yes" } else { "no" }),
        format!("lessons={}", report.lessons.len()),
    ];
    for lesson in &report.lessons {
        let causes = if lesson.causes.is_empty() {
            "none".to_owned()
        } else {
            lesson.causes.join(",")
        };
        lines.push(format!(
            "lesson feedback={} job={} score={} status={} causes={causes} text={}",
            lesson.feedback,
            lesson.job,
            lesson.score,
            lesson.status,
            one_line(&lesson.text),
        ));
    }
    lines.push(format!("orchestrators={}", report.orchestrators.len()));
    for group in &report.orchestrators {
        lines.push(format!(
            "orchestrator name={} version={} best_match={} runs={} accepted={} missed={} path={}",
            group.name,
            group.version,
            group.best_match,
            group.runs,
            group.accepted,
            group.missed,
            group.path,
        ));
    }
    lines.push(format!("unused={}", report.unused.len()));
    for unused in &report.unused {
        lines.push(format!(
            "unused name={} version={} path={} description={}",
            unused.name,
            unused.version,
            unused.path,
            one_line(&unused.description),
        ));
    }
    lines
}
