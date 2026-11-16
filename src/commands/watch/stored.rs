//! Reads the trees, jobs, runs and questions of the store into the data of
//! `rex watch`, the jobs through the function `rex jobs` uses so both agree.

use std::time::SystemTime;

use rusqlite::Connection;

use super::WatchError;
use super::data::{
    Delivery, JobBundle, JobData, Origin, Part, QuestionData, RunData, SpendFrom, TreeBundle,
    TreeData,
};
use crate::capture::{Run, RunState, TokenTotal};
use crate::commands::job::{QUESTION_COLUMNS, read_job, read_question};
use crate::commands::jobs::{JobFilter, load_summaries};
use crate::commands::limit::SpendLimit;
use crate::store::{JobId, Store, StoreError};

/// # Errors
/// [`WatchError::Jobs`], [`WatchError::Job`] or [`WatchError::Store`] when a read fails.
pub fn load(store: &mut Store, now: SystemTime) -> Result<Part, WatchError> {
    let summaries = load_summaries(store, &JobFilter::default(), now)?;
    let questions = all_questions(store.conn())?;
    let mut jobs = Vec::with_capacity(summaries.len());
    for (summary, runs) in &summaries {
        let id: JobId =
            summary
                .id
                .parse()
                .map_err(|source| crate::commands::jobs::JobsError::StoredId {
                    job: summary.id.clone(),
                    source,
                })?;
        let record = read_job(store.conn(), store.paths(), &id)?;
        let limit = SpendLimit::of(store, &id)?;
        let job = JobData {
            id: summary.id.clone(),
            tree: summary.tree.clone(),
            builder: summary.builder_agent_id.clone(),
            status: summary.status.clone(),
            request: summary.request.clone(),
            work_type: summary.work_type.clone(),
            size: summary.size.clone(),
            consequence: summary.consequence.clone(),
            orchestrator: summary.orchestrator.as_ref().map(|o| o.name.clone()),
            merge: record
                .merge_source
                .as_ref()
                .map(|_| if record.merge_agreed { "yes" } else { "no" }.to_owned()),
            merge_source: record.merge_source.clone(),
            branch: record.branch.clone(),
            base: record.base.clone(),
            worktree: record.worktree.clone(),
            limit_nano: Some(limit.nanodollars),
            spent: summary.cost.map(|cost| cost.0),
            spent_from: SpendFrom::Own,
            tokens: summary.tokens.map(|tokens| tokens.counts()),
            degraded: record
                .degraded
                .as_deref()
                .filter(|text| !text.is_empty())
                .map_or_else(Vec::new, |text| {
                    text.split(',').map(str::to_owned).collect()
                }),
            deliveries: summary
                .deliveries
                .iter()
                .map(|delivery| Delivery {
                    repo: delivery.repo.clone(),
                    outcome: delivery.outcome.clone(),
                    note: None,
                })
                .collect(),
            done_checks: Vec::new(),
            created_at: summary.created_at.clone(),
        };
        jobs.push(JobBundle {
            runs: runs.runs.iter().map(run_data).collect(),
            questions: questions
                .iter()
                .filter(|(_, job, _)| job.as_deref() == Some(&summary.id))
                .map(|(_, _, question)| question.clone())
                .collect(),
            job,
            origin: Origin::Store,
        });
    }
    // A tree's spend is added up when the sources are merged, because its
    // master's own spend is only in a snapshot.
    let trees = all_trees(store.conn())?
        .into_iter()
        .map(|tree| TreeBundle {
            questions: questions
                .iter()
                .filter(|(tree_id, job, _)| job.is_none() && tree_id.as_deref() == Some(&tree.id))
                .map(|(_, _, question)| question.clone())
                .collect(),
            tree,
            master_run: None,
            origin: Origin::Store,
        })
        .collect();
    Ok(Part { trees, jobs })
}

fn run_data(run: &Run) -> RunData {
    RunData {
        agent_id: run.agent_id.clone(),
        parent: run.parent_agent_id.clone(),
        role: run.role.clone(),
        task: None,
        agent_type: run.agent_type.clone(),
        planned: None,
        model: run.model_ran.clone(),
        effort: run.effort_ran.clone(),
        state: match run.state {
            RunState::Open => "open",
            RunState::Finished => "done",
            RunState::Unconfirmed => "unconfirmed",
        }
        .to_owned(),
        started_at: run.started_at.clone(),
        duration_ms: run.duration_ms,
        tokens: match run.tokens {
            TokenTotal::Unknown => None,
            TokenTotal::Provisional(counts) | TokenTotal::Final(counts) => Some(counts),
        },
        cost: run.cost,
        lint: None,
        skills_attached: Vec::new(),
        skills_opened: run.skills_opened.clone().unwrap_or_default(),
        output_file: None,
    }
}

fn all_trees(conn: &Connection) -> Result<Vec<TreeData>, StoreError> {
    let mut statement = conn.prepare(
        "SELECT id, state, master_agent_id, charter, cap_nano, started_at, closed_at, report_file \
         FROM tree",
    )?;
    let rows = statement.query_map([], |row| {
        let cap_nano: i64 = row.get(4)?;
        Ok(TreeData {
            id: row.get(0)?,
            state: row.get(1)?,
            master: row.get(2)?,
            charter: row.get(3)?,
            spent: None,
            spent_from: SpendFrom::Own,
            cap_nano: u64::try_from(cap_nano).unwrap_or(0),
            started_at: row.get(5)?,
            closed_at: row.get(6)?,
            report_file: row.get(7)?,
            notes: Vec::new(),
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Every question with its tree id and job id.
type Placed = (Option<String>, Option<String>, QuestionData);

fn all_questions(conn: &Connection) -> Result<Vec<Placed>, StoreError> {
    let mut statement = conn.prepare(&format!(
        "SELECT {QUESTION_COLUMNS} FROM question ORDER BY asked_at, id"
    ))?;
    let rows = statement.query_map([], read_question)?;
    let mut questions = Vec::new();
    for row in rows {
        let row = row?;
        let options: Vec<String> = serde_json::from_str(&row.options).unwrap_or_default();
        questions.push((
            row.tree_id,
            row.job_id.or(row.merge_for),
            QuestionData {
                id: row.id,
                question: row.question,
                options,
                recommendation: row.recommendation,
                asked_at: row.asked_at,
                is_answered: row.answer.is_some(),
            },
        ));
    }
    Ok(questions)
}
