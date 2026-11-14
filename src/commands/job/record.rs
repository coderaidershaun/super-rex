//! Reads one job's stored record, from all the tables that hold a part of it,
//! for `rex job show` and `rex tree show` to print.

use rusqlite::{Connection, OptionalExtension, Row};

use super::JobError;
use super::value::stored_level;
use crate::oms::Inputs;
use crate::store::status::Status;
use crate::store::{JobId, StoreError, StorePaths};

/// The `job` row, and the rows of the other tables that belong to it.
pub(crate) struct JobRecord {
    pub(crate) id: String,
    pub(crate) status: Status,
    pub(crate) request: String,
    pub(crate) origin: String,
    pub(crate) restatement: Option<String>,
    pub(crate) work_type: Option<String>,
    pub(crate) size: Option<String>,
    pub(crate) consequence: Option<String>,
    pub(crate) languages: String,
    pub(crate) pref_duration: Option<String>,
    pub(crate) pref_budget: Option<String>,
    pub(crate) pref_autonomy: Option<String>,
    pub(crate) merge_agreed: bool,
    pub(crate) merge_source: Option<String>,
    pub(crate) merge_question: Option<String>,
    pub(crate) policy_hash: Option<String>,
    pub(crate) session_id: String,
    pub(crate) builder_agent_id: String,
    pub(crate) repo: Option<String>,
    pub(crate) branch: Option<String>,
    pub(crate) base: Option<String>,
    pub(crate) worktree: Option<String>,
    pub(crate) tree_id: Option<String>,
    pub(crate) degraded: Option<String>,
    pub(crate) builder_model: Option<String>,
    pub(crate) builder_effort: Option<String>,
    pub(crate) experience_recalled: Option<String>,
    pub(crate) breakdown: Option<String>,
    pub(crate) orchestrator_name: Option<String>,
    pub(crate) orchestrator_version: Option<i64>,
    pub(crate) skills: Option<String>,
    pub(crate) outcome: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    pub(super) transitions: Vec<Transition>,
    pub(super) roles: Vec<RoleRow>,
    pub(super) deliveries: Vec<DeliveryRow>,
    pub(super) feedback: Vec<FeedbackRow>,
    pub(super) questions: Vec<QuestionRow>,
    pub(super) decisions: Vec<DecisionRow>,
    pub(super) manifest: Vec<ManifestRow>,
}

pub(super) struct Transition {
    pub(super) from: Option<String>,
    pub(super) to: String,
    pub(super) event: String,
    pub(super) at: String,
}

/// A role and what the selection function chose for it.
pub(super) struct RoleRow {
    pub(super) role: String,
    pub(super) inputs: Inputs,
    pub(super) agent: String,
    pub(super) demand: String,
    pub(super) rules: String,
    pub(super) conflict: String,
    pub(super) oms_version: i64,
    /// A comma list, or `None` for no skills.
    pub(super) skills: Option<String>,
    pub(super) source: Option<String>,
}

pub(super) struct DeliveryRow {
    pub(super) repo: String,
    pub(super) branch: Option<String>,
    pub(super) pr: Option<String>,
    pub(super) merge_agreed: bool,
    pub(super) merge_commit: Option<String>,
    pub(super) outcome: String,
    pub(super) note: Option<String>,
    pub(super) recorded_at: String,
}

pub(super) struct FeedbackRow {
    pub(super) id: i64,
    pub(super) given_by: String,
    pub(super) said: String,
    pub(super) verdict: String,
    pub(super) causes: Option<String>,
    pub(super) evidence: Option<String>,
    pub(super) lesson: Option<String>,
    pub(super) changed_skills: Option<String>,
    pub(super) changed_orchestrators: Option<String>,
    pub(super) created_at: String,
}

/// A question parked for the person.
pub(crate) struct QuestionRow {
    pub(crate) id: String,
    pub(crate) tree_id: Option<String>,
    pub(crate) job_id: Option<String>,
    pub(crate) merge_for: Option<String>,
    pub(crate) question: String,
    /// The options as the JSON list they were stored as.
    pub(crate) options: String,
    pub(crate) recommendation: String,
    pub(crate) category: String,
    pub(crate) asked_at: String,
    pub(crate) answer: Option<String>,
    pub(crate) answer_source: Option<String>,
    pub(crate) answered_at: Option<String>,
}

pub(super) struct DecisionRow {
    pub(super) assignment_id: String,
    pub(super) subject: String,
    pub(super) item: String,
    pub(super) value: String,
    pub(super) source: String,
    pub(super) reason: String,
    pub(super) at: String,
}

pub(super) struct ManifestRow {
    pub(super) skill: String,
    pub(super) file: String,
    pub(super) hash: String,
    pub(super) source: String,
}

/// The columns of a question, in the order [`question_row`] reads them.
pub(crate) const QUESTION_COLUMNS: &str = "id, tree_id, job_id, merge_for, question, options, \
     recommendation, category, asked_at, answer, answer_source, answered_at";

/// # Errors
/// [`StoreError::JobNotFound`] if the store has no such job, or
/// [`StoreError::Sqlite`] if a read fails.
pub(crate) fn read(
    conn: &Connection,
    paths: &StorePaths,
    job: &JobId,
) -> Result<JobRecord, JobError> {
    let record = conn
        .query_row("SELECT * FROM job WHERE id = ?1", [job.as_str()], job_row)
        .optional()?;
    let mut record = record.ok_or_else(|| StoreError::JobNotFound {
        job: job.to_string(),
        store: paths.dir.clone(),
    })?;
    record.transitions = rows(
        conn,
        "SELECT from_status, to_status, event, at FROM job_transition WHERE job_id = ?1 ORDER BY id",
        job,
        transition_row,
    )?;
    record.roles = rows(
        conn,
        "SELECT role, in_duration, in_budget, in_intelligence, in_autonomy,
                in_reliability, in_consequence, in_verification, agent, demand, rules, conflict,
                oms_version, skills, source
         FROM role WHERE job_id = ?1 ORDER BY rowid",
        job,
        role_row,
    )?;
    record.deliveries = rows(
        conn,
        "SELECT repo, branch, pr, merge_agreed, merge_commit, outcome, note, recorded_at
         FROM delivery WHERE job_id = ?1 ORDER BY id",
        job,
        delivery_row,
    )?;
    record.feedback = rows(
        conn,
        "SELECT id, given_by, said, verdict, causes, evidence, lesson, changed_skills,
                changed_orchestrators, created_at
         FROM feedback WHERE job_id = ?1 ORDER BY id",
        job,
        feedback_row,
    )?;
    record.questions = rows(
        conn,
        &format!(
            "SELECT {QUESTION_COLUMNS} FROM question WHERE job_id = ?1 OR merge_for = ?1 ORDER BY asked_at, id"
        ),
        job,
        question_row,
    )?;
    record.decisions = rows(
        conn,
        "SELECT assignment_id, subject, item, value, source, reason, at
         FROM decision WHERE job_id = ?1 ORDER BY at, assignment_id",
        job,
        |row| {
            Ok(DecisionRow {
                assignment_id: row.get(0)?,
                subject: row.get(1)?,
                item: row.get(2)?,
                value: row.get(3)?,
                source: row.get(4)?,
                reason: row.get(5)?,
                at: row.get(6)?,
            })
        },
    )?;
    record.manifest = rows(
        conn,
        "SELECT skill, file, hash, source FROM manifest WHERE job_id = ?1 ORDER BY skill, file",
        job,
        |row| {
            Ok(ManifestRow {
                skill: row.get(0)?,
                file: row.get(1)?,
                hash: row.get(2)?,
                source: row.get(3)?,
            })
        },
    )?;
    Ok(record)
}

fn rows<T>(
    conn: &Connection,
    sql: &str,
    job: &JobId,
    each: impl FnMut(&Row<'_>) -> rusqlite::Result<T>,
) -> rusqlite::Result<Vec<T>> {
    let mut statement = conn.prepare(sql)?;
    statement.query_map([job.as_str()], each)?.collect()
}

fn job_row(row: &Row<'_>) -> rusqlite::Result<JobRecord> {
    Ok(JobRecord {
        id: row.get("id")?,
        status: row.get("status")?,
        request: row.get("request")?,
        origin: row.get("origin")?,
        restatement: row.get("restatement")?,
        work_type: row.get("work_type")?,
        size: row.get("size")?,
        consequence: row.get("consequence")?,
        languages: row.get("languages")?,
        pref_duration: row.get("pref_duration")?,
        pref_budget: row.get("pref_budget")?,
        pref_autonomy: row.get("pref_autonomy")?,
        merge_agreed: row.get("merge_agreed")?,
        merge_source: row.get("merge_source")?,
        merge_question: row.get("merge_question")?,
        policy_hash: row.get("policy_hash")?,
        session_id: row.get("session_id")?,
        builder_agent_id: row.get("builder_agent_id")?,
        repo: row.get("repo")?,
        branch: row.get("branch")?,
        base: row.get("base")?,
        worktree: row.get("worktree")?,
        tree_id: row.get("tree_id")?,
        degraded: row.get("degraded")?,
        builder_model: row.get("builder_model")?,
        builder_effort: row.get("builder_effort")?,
        experience_recalled: row.get("experience_recalled")?,
        breakdown: row.get("breakdown")?,
        orchestrator_name: row.get("orchestrator_name")?,
        orchestrator_version: row.get("orchestrator_version")?,
        skills: row.get("skills")?,
        outcome: row.get("outcome")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        transitions: Vec::new(),
        roles: Vec::new(),
        deliveries: Vec::new(),
        feedback: Vec::new(),
        questions: Vec::new(),
        decisions: Vec::new(),
        manifest: Vec::new(),
    })
}

fn role_row(row: &Row<'_>) -> rusqlite::Result<RoleRow> {
    Ok(RoleRow {
        role: row.get(0)?,
        inputs: Inputs {
            duration: stored_level(row, 1)?,
            budget: stored_level(row, 2)?,
            intelligence: stored_level(row, 3)?,
            autonomy: stored_level(row, 4)?,
            reliability: stored_level(row, 5)?,
            consequence: stored_level(row, 6)?,
            verification: stored_level(row, 7)?,
        },
        agent: row.get(8)?,
        demand: row.get(9)?,
        rules: row.get(10)?,
        conflict: row.get(11)?,
        oms_version: row.get(12)?,
        skills: row.get(13)?,
        source: row.get(14)?,
    })
}

fn transition_row(row: &Row<'_>) -> rusqlite::Result<Transition> {
    Ok(Transition {
        from: row.get(0)?,
        to: row.get(1)?,
        event: row.get(2)?,
        at: row.get(3)?,
    })
}

fn delivery_row(row: &Row<'_>) -> rusqlite::Result<DeliveryRow> {
    Ok(DeliveryRow {
        repo: row.get(0)?,
        branch: row.get(1)?,
        pr: row.get(2)?,
        merge_agreed: row.get(3)?,
        merge_commit: row.get(4)?,
        outcome: row.get(5)?,
        note: row.get(6)?,
        recorded_at: row.get(7)?,
    })
}

fn feedback_row(row: &Row<'_>) -> rusqlite::Result<FeedbackRow> {
    Ok(FeedbackRow {
        id: row.get(0)?,
        given_by: row.get(1)?,
        said: row.get(2)?,
        verdict: row.get(3)?,
        causes: row.get(4)?,
        evidence: row.get(5)?,
        lesson: row.get(6)?,
        changed_skills: row.get(7)?,
        changed_orchestrators: row.get(8)?,
        created_at: row.get(9)?,
    })
}

/// One question, from the columns of [`QUESTION_COLUMNS`].
pub(crate) fn question_row(row: &Row<'_>) -> rusqlite::Result<QuestionRow> {
    Ok(QuestionRow {
        id: row.get(0)?,
        tree_id: row.get(1)?,
        job_id: row.get(2)?,
        merge_for: row.get(3)?,
        question: row.get(4)?,
        options: row.get(5)?,
        recommendation: row.get(6)?,
        category: row.get(7)?,
        asked_at: row.get(8)?,
        answer: row.get(9)?,
        answer_source: row.get(10)?,
        answered_at: row.get(11)?,
    })
}
