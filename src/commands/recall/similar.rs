//! The queries of `rex recall`: the jobs that are similar to the asked job,
//! their lessons and orchestrators, and the orchestrators that never ran.

use rusqlite::{OptionalExtension, Row, Transaction, named_params};

use super::report::{Group, Lesson, Spent, Unused, skill_path};
use crate::capture::{Cost, TokenCounts};
use crate::store::integer::unsigned;
use crate::store::{JobId, StoreError, StorePaths};

/// A job is similar when at least this many of the four fields are equal.
const MIN_SCORE: i64 = 2;
const MAX_LESSONS: i64 = 5;
const MAX_ORCHESTRATORS: usize = 3;

/// The four fields of the asked job that the score compares.
pub(super) struct Asked {
    work_type: String,
    size: String,
    consequence: String,
    languages: String,
}

pub(super) fn asked_job(
    tx: &Transaction<'_>,
    paths: &StorePaths,
    job: &JobId,
) -> Result<Asked, StoreError> {
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

/// `scored`: every other job with its score, one point for each of the four
/// fields that is non-empty and equal to the asked job's. Each job also has the
/// cost and tokens of its main session and runs, all NULL when none is stored.
const SCORED_JOBS: &str = "
    WITH spent AS (
        SELECT job_id, SUM(cost_nanodollars) AS nanodollars,
               SUM(cost_unpriced_requests) AS unpriced, SUM(tokens_input) AS input,
               SUM(tokens_output) AS output, SUM(tokens_cache_read) AS cache_read,
               SUM(tokens_cache_creation) AS cache_creation
        FROM (SELECT job_id, cost_nanodollars, cost_unpriced_requests, tokens_input,
                     tokens_output, tokens_cache_read, tokens_cache_creation
              FROM sub_agent_run
              UNION ALL
              SELECT job_id, cost_nanodollars, cost_unpriced_requests, tokens_input,
                     tokens_output, tokens_cache_read, tokens_cache_creation
              FROM session_tokens)
        GROUP BY job_id),
    scored AS (
        SELECT j.id, j.created_at, j.status, j.orchestrator_name, j.orchestrator_version,
               (j.work_type <> '' AND j.work_type = :work_type)
             + (j.size <> '' AND j.size = :size)
             + (j.consequence <> '' AND j.consequence = :consequence)
             + (j.languages <> '' AND j.languages = :languages) AS score,
               s.nanodollars, s.unpriced, s.input, s.output, s.cache_read, s.cache_creation
        FROM job j LEFT JOIN spent s ON s.job_id = j.id
        WHERE j.id <> :job)";

/// The six spend columns of `scored`, from `first`. `None` when the job has
/// no stored cost.
fn spent_at(row: &Row<'_>, first: usize) -> rusqlite::Result<Option<Spent>> {
    let Some(nanodollars) = unsigned(row, first)? else {
        return Ok(None);
    };
    let count =
        |offset| -> rusqlite::Result<u64> { Ok(unsigned(row, first + offset)?.unwrap_or(0)) };
    Ok(Some(Spent {
        cost: Cost {
            nanodollars,
            unpriced_requests: count(1)?,
        },
        tokens: TokenCounts {
            input: count(2)?,
            output: count(3)?,
            cache_read: count(4)?,
            cache_creation: count(5)?,
        },
        jobs: 1,
    }))
}

pub(super) fn lessons(
    tx: &Transaction<'_>,
    job: &JobId,
    asked: &Asked,
) -> Result<Vec<Lesson>, StoreError> {
    let sql = format!(
        "{SCORED_JOBS}
         SELECT f.id, s.id, s.score, s.status, f.causes, f.lesson, s.nanodollars, s.unpriced,
                s.input, s.output, s.cache_read, s.cache_creation
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
            let spent = spent_at(row, 6)?;
            Ok(Lesson {
                feedback: row.get(0)?,
                job: row.get(1)?,
                score: row.get(2)?,
                status: row.get(3)?,
                cost: spent.map(|spent| spent.cost),
                unpriced_requests: spent.map(|spent| spent.cost.unpriced_requests),
                cache_hit: spent.and_then(|spent| spent.tokens.cache_hit_percent()),
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
pub(super) fn groups(
    tx: &Transaction<'_>,
    paths: &StorePaths,
    job: &JobId,
    asked: &Asked,
) -> Result<Vec<Group>, StoreError> {
    let sql = format!(
        "{SCORED_JOBS}
         SELECT orchestrator_name, orchestrator_version, score, status, nanodollars, unpriced,
                input, output, cache_read, cache_creation
         FROM scored
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
                spent_at(row, 4)?,
            ))
        },
    )?;

    // Newest job first, so the first job of a name sets its version.
    let mut groups: Vec<Group> = Vec::new();
    for row in rows {
        let (name, version, score, status, spent) = row?;
        match groups.iter_mut().find(|group| group.name == name) {
            Some(group) => group.count(score, &status, spent),
            None => {
                let mut group = Group::empty(paths, name, version);
                group.count(score, &status, spent);
                groups.push(group);
            }
        }
    }
    // Of groups with equal results, the cheaper comes first. The count of runs
    // is not a result, so it only breaks a tie of cost.
    groups.sort_by(|a, b| {
        (b.best_match, b.accepted, a.missed)
            .cmp(&(a.best_match, a.accepted, b.missed))
            .then_with(|| a.cost_rank().cmp(&b.cost_rank()))
            .then_with(|| b.runs.cmp(&a.runs))
            .then_with(|| a.name.cmp(&b.name))
    });
    groups.truncate(MAX_ORCHESTRATORS);
    Ok(groups)
}

/// Each indexed name whose newest version no job names.
pub(super) fn unused(tx: &Transaction<'_>, paths: &StorePaths) -> Result<Vec<Unused>, StoreError> {
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
