//! `rex job show`: prints one job's whole record, with its sub-agent runs, so
//! the builder and the feedback skill read the job from one command.

use std::path::Path;
use std::time::SystemTime;

use serde_json::{Value, json};

use super::JobError;
use super::input::non_empty_var;
use super::record::{self, DeliveryRow, FeedbackRow, JobRecord, RoleRow};
use super::value::value_name;
use crate::capture::{self, JobRuns, Run, RunState, TokenTotal};
use crate::commands::stdout::print_to_stdout;
use crate::oms::Level;
use crate::store::status::Shown;
use crate::store::{JobId, Store, StoreError, StorePaths};

/// Arguments of `rex job show`.
#[derive(clap::Args)]
#[command(
    after_help = "status is the shown status: running for a planned job with a sub-agent open. stored_status is the status as stored."
)]
pub struct ShowOptions {
    /// The job to show [default: the open job of $CLAUDE_CODE_SESSION_ID].
    job: Option<JobId>,

    /// Print the full record as one JSON line.
    #[arg(long)]
    json: bool,
}

/// The job to show: the one named, or the open job of a session.
enum Target {
    Job(JobId),
    OpenJobOf(String),
}

/// # Errors
/// - [`JobError::NoJobArgument`] if no job is given and no session is set.
/// - [`JobError::Store`]: the job or the session's open job is not found, or
///   the store fails.
pub(super) fn run(cwd: &Path, options: ShowOptions) -> Result<(), JobError> {
    let target = match options.job {
        Some(job) => Target::Job(job),
        None => Target::OpenJobOf(
            non_empty_var("CLAUDE_CODE_SESSION_ID").ok_or(JobError::NoJobArgument)?,
        ),
    };
    let mut store = Store::open(&StorePaths::discover(cwd)?)?;
    let job = match target {
        Target::Job(job) => job,
        Target::OpenJobOf(session) => {
            store
                .open_job_for_session(&session)?
                .ok_or_else(|| StoreError::NoOpenJob {
                    session,
                    store: store.paths().dir.clone(),
                })?
        }
    };

    let view = capture::job_runs(&mut store, &job, SystemTime::now())?;
    let record = record::read(store.conn(), store.paths(), &job)?;
    let shown = Shown::of(record.status, view.open);
    let text = if options.json {
        format!("{}\n", json_record(&record, shown, &view))
    } else {
        plain_text(&record, shown, &view)
    };
    print_to_stdout(&text)?;
    Ok(())
}

fn plain_text(record: &JobRecord, shown: Shown, view: &JobRuns) -> String {
    let mut lines = scalar_lines(record, shown, view);
    lines.extend(record.roles.iter().map(role_line));
    for (number, delivery) in record.deliveries.iter().enumerate() {
        lines.push(delivery_line(number + 1, delivery));
    }
    lines.extend(view.runs.iter().map(run_line));
    lines.extend(long_field_lines(record));
    lines.push(String::new());
    lines.join("\n")
}

/// One `key=value` line for each single value, `outcome` last.
fn scalar_lines(record: &JobRecord, shown: Shown, view: &JobRuns) -> Vec<String> {
    let scalars = [
        ("job", record.id.clone()),
        ("status", shown.to_string()),
        ("stored_status", record.status.to_string()),
        ("type", record.work_type.clone()),
        ("size", record.size.clone()),
        ("consequence", record.consequence.clone()),
        ("languages", record.languages.clone()),
        (
            "duration",
            or_none(record.pref_duration.as_deref()).to_owned(),
        ),
        ("budget", or_none(record.pref_budget.as_deref()).to_owned()),
        (
            "autonomy",
            or_none(record.pref_autonomy.as_deref()).to_owned(),
        ),
        ("merge_agreed", yes_no(record.merge_agreed).to_owned()),
        ("session", record.session_id.clone()),
        ("asked_by", record.asked_by.clone()),
        (
            "builder_model",
            or_none(record.builder_model.as_deref()).to_owned(),
        ),
        (
            "builder_effort",
            or_none(record.builder_effort.as_deref()).to_owned(),
        ),
        (
            "orchestrator",
            or_none(record.orchestrator_name.as_deref()).to_owned(),
        ),
        (
            "orchestrator_version",
            number_or_none(record.orchestrator_version),
        ),
        ("skills", or_none(record.skills.as_deref()).to_owned()),
        ("created_at", record.created_at.clone()),
        ("updated_at", record.updated_at.clone()),
        ("tokens", plain_total(view.tokens)),
        (
            "tokens_provisional",
            yes_no(is_provisional(view.tokens)).to_owned(),
        ),
        ("incomplete", yes_no(view.incomplete).to_owned()),
        ("outcome", record.outcome.clone().unwrap_or_default()),
    ];
    scalars
        .into_iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect()
}

fn role_line(role: &RoleRow) -> String {
    let skills = or_none(role.skills.as_deref());
    let Some(agent) = &role.agent else {
        return format!("role.{}=session skills={skills}", role.role);
    };
    format!(
        "role.{}={agent} inputs={} demand={} rules={} conflict={} oms_version={} skills={skills}",
        role.role,
        role.inputs.code(),
        or_none(role.demand.as_deref()),
        or_none(role.rules.as_deref()),
        or_none(role.conflict.as_deref()),
        number_or_none(role.oms_version),
    )
}

fn delivery_line(number: usize, delivery: &DeliveryRow) -> String {
    format!(
        "delivery.{number}={} branch={} pr={} merge_agreed={} merge_commit={} repo={}",
        delivery.outcome,
        or_none(delivery.branch.as_deref()),
        or_none(delivery.pr.as_deref()),
        yes_no(delivery.merge_agreed),
        or_none(delivery.merge_commit.as_deref()),
        delivery.repo,
    )
}

fn run_line(run: &Run) -> String {
    let skills = match run.skills_opened.as_deref() {
        None | Some([]) => "none".to_owned(),
        Some(skills) => skills.join(","),
    };
    format!(
        "run.{}={} parent={} role={} agent={} ran={}/{} skills={skills} tokens={} provisional={}",
        run.agent_id,
        state_name(run.state),
        run.parent_agent_id.as_deref().unwrap_or("main"),
        or_none(run.role.as_deref()),
        or_none(run.agent_type.as_deref()),
        or_none(run.model_ran.as_deref()),
        or_none(run.effort_ran.as_deref()),
        plain_total(run.tokens),
        yes_no(is_provisional(run.tokens)),
    )
}

/// The fields that can hold several lines: a `<key>:` line, then each line of
/// the text indented by two spaces.
fn long_field_lines(record: &JobRecord) -> Vec<String> {
    let mut fields = vec![
        ("request".to_owned(), Some(record.request.as_str())),
        ("breakdown".to_owned(), record.breakdown.as_deref()),
        (
            "experience_recalled".to_owned(),
            record.experience_recalled.as_deref(),
        ),
    ];
    for (number, delivery) in record.deliveries.iter().enumerate() {
        if let Some(note) = &delivery.note {
            fields.push((format!("delivery.{}.note", number + 1), Some(note)));
        }
    }
    let mut lines = Vec::new();
    for (key, value) in fields {
        lines.push(format!("{key}:"));
        for line in value.unwrap_or_default().lines() {
            lines.push(format!("  {line}"));
        }
    }
    lines
}

fn json_record(record: &JobRecord, shown: Shown, view: &JobRuns) -> Value {
    let transitions: Vec<Value> = record
        .transitions
        .iter()
        .map(|row| json!({"from": row.from, "to": row.to, "event": row.event, "at": row.at}))
        .collect();
    json!({
        "schema": 1,
        "job": json_job(record, shown),
        "transitions": transitions,
        "roles": record.roles.iter().map(json_role).collect::<Vec<_>>(),
        "deliveries": record.deliveries.iter().map(json_delivery).collect::<Vec<_>>(),
        "runs": view.runs.iter().map(json_run).collect::<Vec<_>>(),
        "tokens": json_total(view.tokens),
        "session_tokens": json_total(view.session_tokens),
        "incomplete": view.incomplete,
        "damaged_log_lines": view.damaged_log_lines,
        "feedback": record.feedback.iter().map(json_feedback).collect::<Vec<_>>(),
    })
}

fn json_job(record: &JobRecord, shown: Shown) -> Value {
    let orchestrator = match (&record.orchestrator_name, record.orchestrator_version) {
        (Some(name), Some(version)) => json!({"name": name, "version": version}),
        _ => Value::Null,
    };
    json!({
        "id": record.id,
        "status": shown.to_string(),
        "stored_status": record.status.to_string(),
        "request": record.request,
        "asked_by": record.asked_by,
        "class_version": record.class_version,
        "work_type": record.work_type,
        "size": record.size,
        "consequence": record.consequence,
        "languages": list(Some(record.languages.as_str())),
        "pref_duration": record.pref_duration,
        "pref_budget": record.pref_budget,
        "pref_autonomy": record.pref_autonomy,
        "merge_agreed": record.merge_agreed,
        "session_id": record.session_id,
        "builder_model": record.builder_model,
        "builder_effort": record.builder_effort,
        "experience_recalled": record.experience_recalled,
        "breakdown": record.breakdown,
        "orchestrator": orchestrator,
        "skills": list(record.skills.as_deref()),
        "outcome": record.outcome,
        "created_at": record.created_at,
        "updated_at": record.updated_at,
    })
}

fn json_role(role: &RoleRow) -> Value {
    let level = |level: Option<Level>| level.map(value_name);
    let inputs = role.inputs;
    json!({
        "role": role.role,
        "session": role.runs_as_session,
        "inputs": {
            "duration": level(inputs.duration),
            "budget": level(inputs.budget),
            "autonomy": level(inputs.autonomy),
            "intelligence": level(inputs.intelligence),
            "reliability": level(inputs.reliability),
            "consequence": level(inputs.consequence),
            "verification": level(inputs.verification),
        },
        "agent": role.agent,
        "demand": role.demand,
        "rules": role.rules.as_deref().map(|rules| list(Some(rules))),
        "conflict": role.conflict.as_deref().map(|conflict| match conflict {
            "none" => json!([]),
            _ => list(Some(conflict)),
        }),
        "oms_version": role.oms_version,
        "skills": list(role.skills.as_deref()),
    })
}

fn json_delivery(delivery: &DeliveryRow) -> Value {
    json!({
        "repo": delivery.repo,
        "branch": delivery.branch,
        "pr": delivery.pr,
        "merge_agreed": delivery.merge_agreed,
        "merge_commit": delivery.merge_commit,
        "outcome": delivery.outcome,
        "note": delivery.note,
        "recorded_at": delivery.recorded_at,
    })
}

fn json_feedback(row: &FeedbackRow) -> Value {
    json!({
        "id": row.id,
        "given_by": row.given_by,
        "said": row.said,
        "verdict": row.verdict,
        "causes": list(row.causes.as_deref()),
        "evidence": row.evidence,
        "lesson": row.lesson,
        "changed_skills": list(row.changed_skills.as_deref()),
        "changed_orchestrators": list(row.changed_orchestrators.as_deref()),
        "created_at": row.created_at,
    })
}

fn json_run(run: &Run) -> Value {
    json!({
        "agent_id": run.agent_id,
        "state": state_name(run.state),
        "parent_agent_id": run.parent_agent_id,
        "depth": run.depth,
        "background": run.background,
        "role": run.role,
        "tag_job": run.tag_job,
        "agent_type": run.agent_type,
        "model_ran": run.model_ran,
        "effort_ran": run.effort_ran,
        "started_at": run.started_at,
        "stopped_at": run.stopped_at,
        "duration_ms": run.duration_ms,
        "tool_uses": run.tool_uses,
        "lines_added": run.lines_added,
        "lines_removed": run.lines_removed,
        "brief": run.brief,
        "skills_opened": run.skills_opened,
        "final_message": run.final_message,
        "tokens": json_total(run.tokens),
    })
}

/// A stored comma list as a JSON array; NULL, the empty list, is `[]`.
fn list(text: Option<&str>) -> Value {
    let items: Vec<&str> = text.map_or_else(Vec::new, |text| text.split(',').collect());
    json!(items)
}

fn json_total(total: TokenTotal) -> Value {
    let (counts, is_provisional) = match total {
        TokenTotal::Unknown => return Value::Null,
        TokenTotal::Provisional(counts) => (counts, true),
        TokenTotal::Final(counts) => (counts, false),
    };
    json!({
        "input": counts.input,
        "output": counts.output,
        "cache_read": counts.cache_read,
        "cache_creation": counts.cache_creation,
        "provisional": is_provisional,
    })
}

/// A total in plain text: the sum of the four counts, or `none`.
fn plain_total(total: TokenTotal) -> String {
    match total {
        TokenTotal::Unknown => "none".to_owned(),
        TokenTotal::Provisional(counts) | TokenTotal::Final(counts) => {
            (counts.input + counts.output + counts.cache_read + counts.cache_creation).to_string()
        }
    }
}

/// Only a provisional total may still grow. An unknown total is not provisional, as in `rex jobs --json`.
fn is_provisional(total: TokenTotal) -> bool {
    matches!(total, TokenTotal::Provisional(_))
}

fn or_none(value: Option<&str>) -> &str {
    value.unwrap_or("none")
}

fn number_or_none(value: Option<i64>) -> String {
    value.map_or_else(|| "none".to_owned(), |number| number.to_string())
}

fn yes_no(is_yes: bool) -> &'static str {
    if is_yes { "yes" } else { "no" }
}

fn state_name(state: RunState) -> &'static str {
    match state {
        RunState::Open => "open",
        RunState::Finished => "finished",
        RunState::Unconfirmed => "unconfirmed",
    }
}
