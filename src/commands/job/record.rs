//! Reads one job's stored record, from all the tables that hold a part of it,
//! for `rex job show` to print.

use rusqlite::types::Type;
use rusqlite::{Connection, OptionalExtension, Row};

use super::JobError;
use super::plan::stored_level;
use crate::oms::Inputs;
use crate::store::status::Status;
use crate::store::{JobId, StoreError, StorePaths};

/// The `job` row, and the rows of the other tables that belong to it.
pub(super) struct JobRecord {
    pub(super) id: String,
    pub(super) status: Status,
    pub(super) request: String,
    pub(super) asked_by: String,
    pub(super) class_version: i64,
    pub(super) work_type: String,
    pub(super) size: String,
    pub(super) consequence: String,
    pub(super) languages: String,
    pub(super) pref_duration: Option<String>,
    pub(super) pref_budget: Option<String>,
    pub(super) pref_autonomy: Option<String>,
    pub(super) merge_agreed: bool,
    pub(super) session_id: String,
    pub(super) builder_model: Option<String>,
    pub(super) builder_effort: Option<String>,
    pub(super) experience_recalled: Option<String>,
    pub(super) breakdown: Option<String>,
    pub(super) orchestrator_name: Option<String>,
    pub(super) orchestrator_version: Option<i64>,
    pub(super) skills: Option<String>,
    pub(super) outcome: Option<String>,
    pub(super) created_at: String,
    pub(super) updated_at: String,
    pub(super) transitions: Vec<Transition>,
    pub(super) roles: Vec<RoleRow>,
    pub(super) deliveries: Vec<DeliveryRow>,
    pub(super) feedback: Vec<FeedbackRow>,
}

pub(super) struct Transition {
    pub(super) from: Option<String>,
    pub(super) to: String,
    pub(super) event: String,
    pub(super) at: String,
}

/// A role and its OMS record. `agent` and the fields after it are `None` for a
/// role run as the session.
pub(super) struct RoleRow {
    pub(super) role: String,
    pub(super) runs_as_session: bool,
    pub(super) inputs: Inputs,
    pub(super) agent: Option<String>,
    pub(super) demand: Option<String>,
    pub(super) rules: Option<String>,
    pub(super) conflict: Option<String>,
    pub(super) oms_version: Option<i64>,
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

/// # Errors
/// [`StoreError::JobNotFound`] if the store has no such job, or
/// [`StoreError::Sqlite`] if a read fails.
pub(super) fn read(
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
        "SELECT role, runs_as_session, in_duration, in_budget, in_intelligence, in_autonomy,
                in_reliability, in_consequence, in_verification, agent, demand, rules, conflict,
                oms_version
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
    let status: String = row.get("status")?;
    let status = status.parse().map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error))
    })?;
    Ok(JobRecord {
        id: row.get("id")?,
        status,
        request: row.get("request")?,
        asked_by: row.get("asked_by")?,
        class_version: row.get("class_version")?,
        work_type: row.get("work_type")?,
        size: row.get("size")?,
        consequence: row.get("consequence")?,
        languages: row.get("languages")?,
        pref_duration: row.get("pref_duration")?,
        pref_budget: row.get("pref_budget")?,
        pref_autonomy: row.get("pref_autonomy")?,
        merge_agreed: row.get("merge_agreed")?,
        session_id: row.get("session_id")?,
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
    })
}

fn role_row(row: &Row<'_>) -> rusqlite::Result<RoleRow> {
    Ok(RoleRow {
        role: row.get(0)?,
        runs_as_session: row.get(1)?,
        inputs: Inputs {
            duration: stored_level(row, 2)?,
            budget: stored_level(row, 3)?,
            intelligence: stored_level(row, 4)?,
            autonomy: stored_level(row, 5)?,
            reliability: stored_level(row, 6)?,
            consequence: stored_level(row, 7)?,
            verification: stored_level(row, 8)?,
        },
        agent: row.get(9)?,
        demand: row.get(10)?,
        rules: row.get(11)?,
        conflict: row.get(12)?,
        oms_version: row.get(13)?,
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
