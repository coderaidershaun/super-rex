//! `rex job show`: prints one job's whole record, with its sub-agent runs, so
//! the builder and the feedback skill read the job from one command.

use std::path::Path;
use std::time::SystemTime;

use serde_json::{Value, json};

use super::JobError;
use super::record::{self, DeliveryRow, FeedbackRow, JobRecord, QuestionRow, RoleRow};
use super::value::value_name;
use crate::capture::{self, Cost, JobRuns, Run, RunState, TokenTotal};
use crate::commands::limit::SpendLimit;
use crate::commands::stdout::print_to_stdout;
use crate::commands::token_text::cost_usd;
use crate::oms::Level;
use crate::store::status::Shown;
use crate::store::{JobId, Store, StoreError, StorePaths};

/// Arguments of `rex job show`.
#[derive(clap::Args)]
#[command(
    after_help = "status is the shown status: running for a planned job with a sub-agent open. stored_status is the status as stored."
)]
pub struct ShowOptions {
    /// The job to show [default: the open job of the main loop of $CLAUDE_CODE_SESSION_ID].
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
    let limit = SpendLimit::of(&store, &job)?;
    let shown = Shown::of(record.status, view.open);
    let text = if options.json {
        format!("{}\n", json_record(&record, shown, &limit, &view))
    } else {
        plain_text(&record, shown, &limit, &view)
    };
    print_to_stdout(&text)?;
    Ok(())
}

/// An environment variable, where an empty value counts as not set.
fn non_empty_var(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

fn plain_text(record: &JobRecord, shown: Shown, limit: &SpendLimit, view: &JobRuns) -> String {
    let mut lines = scalar_lines(record, shown, limit, view);
    lines.extend(record.roles.iter().map(role_line));
    for (number, delivery) in record.deliveries.iter().enumerate() {
        lines.push(delivery_line(number + 1, delivery));
    }
    lines.push(format!(
        "session {} provisional={}",
        token_pairs(view.session_tokens, view.session_cost),
        yes_no(is_provisional(view.session_tokens)),
    ));
    lines.extend(view.runs.iter().map(run_line));
    lines.extend(long_field_lines(record));
    lines.push(String::new());
    lines.join("\n")
}

/// One `key=value` line for each single value, `outcome` last.
fn scalar_lines(
    record: &JobRecord,
    shown: Shown,
    limit: &SpendLimit,
    view: &JobRuns,
) -> Vec<String> {
    let mut scalars = vec![
        ("job", record.id.clone()),
        ("status", shown.to_string()),
        ("stored_status", record.status.to_string()),
        ("origin", record.origin.clone()),
        ("builder", record.builder_agent_id.clone()),
        ("tree", or_none(record.tree_id.as_deref()).to_owned()),
        ("type", or_none(record.work_type.as_deref()).to_owned()),
        ("size", or_none(record.size.as_deref()).to_owned()),
        (
            "consequence",
            or_none(record.consequence.as_deref()).to_owned(),
        ),
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
        ("merge", merge_answer(record).unwrap_or("none").to_owned()),
        (
            "merge_source",
            or_none(record.merge_source.as_deref()).to_owned(),
        ),
        ("session", record.session_id.clone()),
        ("repo", or_none(record.repo.as_deref()).to_owned()),
        ("branch", or_none(record.branch.as_deref()).to_owned()),
        ("base", or_none(record.base.as_deref()).to_owned()),
        ("limit_usd", limit_usd(limit)),
        ("limit_source", limit.source.to_string()),
        ("degraded", or_none(record.degraded.as_deref()).to_owned()),
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
    ];
    scalars.extend(
        TOKEN_SCALARS
            .into_iter()
            .zip(token_values(view.tokens, view.cost)),
    );
    scalars.extend([
        (
            "tokens_provisional",
            yes_no(is_provisional(view.tokens)).to_owned(),
        ),
        ("incomplete", yes_no(view.incomplete).to_owned()),
        ("outcome", record.outcome.clone().unwrap_or_default()),
    ]);
    scalars
        .into_iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect()
}

fn role_line(role: &RoleRow) -> String {
    let skills = or_none(role.skills.as_deref());
    format!(
        "role.{}={} inputs={} demand={} rules={} conflict={} oms_version={} skills={skills}",
        role.role,
        role.agent,
        role.inputs.code(),
        role.demand,
        role.rules,
        role.conflict,
        role.oms_version,
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
        "run.{}={} parent={} role={} agent={} ran={}/{} skills={skills} {} provisional={}",
        run.agent_id,
        state_name(run.state),
        run.parent_agent_id.as_deref().unwrap_or("main"),
        or_none(run.role.as_deref()),
        or_none(run.agent_type.as_deref()),
        or_none(run.model_ran.as_deref()),
        or_none(run.effort_ran.as_deref()),
        token_pairs(run.tokens, run.cost),
        yes_no(is_provisional(run.tokens)),
    )
}

/// The fields that can hold several lines: a `<key>:` line, then each line of
/// the text indented by two spaces.
fn long_field_lines(record: &JobRecord) -> Vec<String> {
    let mut fields = vec![
        ("request".to_owned(), Some(record.request.as_str())),
        ("restatement".to_owned(), record.restatement.as_deref()),
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

fn json_record(record: &JobRecord, shown: Shown, limit: &SpendLimit, view: &JobRuns) -> Value {
    let transitions: Vec<Value> = record
        .transitions
        .iter()
        .map(|row| json!({"from": row.from, "to": row.to, "event": row.event, "at": row.at}))
        .collect();
    let decisions: Vec<Value> = record
        .decisions
        .iter()
        .map(|row| {
            json!({
                "assignment_id": row.assignment_id, "subject": row.subject, "item": row.item,
                "value": row.value, "source": row.source, "reason": row.reason, "at": row.at,
            })
        })
        .collect();
    let manifest: Vec<Value> = record
        .manifest
        .iter()
        .map(|row| json!({"skill": row.skill, "file": row.file, "hash": row.hash, "source": row.source}))
        .collect();
    json!({
        "schema": 2,
        "job": json_job(record, shown, limit),
        "transitions": transitions,
        "roles": record.roles.iter().map(json_role).collect::<Vec<_>>(),
        "deliveries": record.deliveries.iter().map(json_delivery).collect::<Vec<_>>(),
        "runs": view.runs.iter().map(json_run).collect::<Vec<_>>(),
        "questions": record.questions.iter().map(json_question).collect::<Vec<_>>(),
        "decisions": decisions,
        "manifest": manifest,
        "tokens": json_total(view.tokens),
        "session_tokens": json_total(view.session_tokens),
        "cost": json_cost(view.cost),
        "session_cost": json_cost(view.session_cost),
        "incomplete": view.incomplete,
        "feedback": record.feedback.iter().map(json_feedback).collect::<Vec<_>>(),
    })
}

/// The job object that `rex job show --json` and `rex tree show --json` print.
pub(crate) fn json_job(record: &JobRecord, shown: Shown, limit: &SpendLimit) -> Value {
    let orchestrator = match (&record.orchestrator_name, record.orchestrator_version) {
        (Some(name), version) => json!({"name": name, "version": version}),
        _ => Value::Null,
    };
    json!({
        "id": record.id,
        "status": shown.to_string(),
        "stored_status": record.status.to_string(),
        "origin": record.origin,
        "session_id": record.session_id,
        "builder_agent_id": record.builder_agent_id,
        "tree": record.tree_id,
        "request": record.request,
        "restatement": record.restatement,
        "merge": {
            "answer": merge_answer(record),
            "source": record.merge_source,
            "question": record.merge_question,
        },
        "merge_agreed": record.merge_agreed,
        "policy_hash": record.policy_hash,
        "limit_usd": limit.nanodollars as f64 / 1_000_000_000.0,
        "limit_source": limit.source.to_string(),
        "degraded": list(record.degraded.as_deref()),
        "repo": record.repo,
        "branch": record.branch,
        "base": record.base,
        "worktree": record.worktree,
        "work_type": record.work_type,
        "size": record.size,
        "consequence": record.consequence,
        "languages": languages(&record.languages),
        "pref_duration": record.pref_duration,
        "pref_budget": record.pref_budget,
        "pref_autonomy": record.pref_autonomy,
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

/// One question as `rex job show --json` and `rex tree show --json` print it.
pub(crate) fn json_question(row: &QuestionRow) -> Value {
    let options: Value = serde_json::from_str(&row.options).unwrap_or_else(|_| json!([]));
    json!({
        "id": row.id,
        "tree": row.tree_id,
        "job": row.job_id,
        "merge_for": row.merge_for,
        "question": row.question,
        "options": options,
        "recommendation": row.recommendation,
        "category": row.category,
        "asked_at": row.asked_at,
        "answer": row.answer,
        "answer_source": row.answer_source,
        "answered_at": row.answered_at,
    })
}

/// `yes` or `no` once someone has answered, else `None`.
fn merge_answer(record: &JobRecord) -> Option<&'static str> {
    record
        .merge_source
        .as_ref()
        .map(|_| yes_no(record.merge_agreed))
}

/// The stored languages as a list; `none` is the empty list.
fn languages(text: &str) -> Value {
    if text == "none" {
        return json!([]);
    }
    list(Some(text))
}

/// The limit in dollars, with six decimals as every amount `rex` prints.
fn limit_usd(limit: &SpendLimit) -> String {
    cost_usd(&Cost {
        nanodollars: limit.nanodollars,
        unpriced_requests: 0,
    })
}

fn json_role(role: &RoleRow) -> Value {
    let level = |level: Option<Level>| level.map(value_name);
    let inputs = role.inputs;
    json!({
        "role": role.role,
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
        "rules": list(Some(&role.rules)),
        "conflict": match role.conflict.as_str() {
            "none" => json!([]),
            conflict => list(Some(conflict)),
        },
        "oms_version": role.oms_version,
        "skills": list(role.skills.as_deref()),
        "source": role.source,
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
        "cost": json_cost(run.cost),
    })
}

/// A stored comma list as a JSON array; NULL or an empty text, the empty list, is `[]`.
fn list(text: Option<&str>) -> Value {
    let items: Vec<&str> = text
        .filter(|text| !text.is_empty())
        .map_or_else(Vec::new, |text| text.split(',').collect());
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

/// `{"usd": 0.113724, "unpriced_requests": 0}`, or `null` while the tokens are unknown.
fn json_cost(cost: Option<Cost>) -> Value {
    cost.map_or(Value::Null, |cost| {
        json!({
            "usd": cost.dollars(),
            "unpriced_requests": cost.unpriced_requests,
        })
    })
}

/// The keys of a total's figures, in their order.
const TOKEN_KEYS: [&str; 7] = [
    "input",
    "cache_write",
    "cache_read",
    "output",
    "cache_hit",
    "cost_usd",
    "unpriced_requests",
];

/// The same keys as job scalars.
const TOKEN_SCALARS: [&str; 7] = [
    "tokens_input",
    "tokens_cache_write",
    "tokens_cache_read",
    "tokens_output",
    "tokens_cache_hit",
    "cost_usd",
    "cost_unpriced_requests",
];

/// A total's figures in the order of [`TOKEN_KEYS`]: the four counts, the hit %
/// (`90%`, `none` with no prompt token), the cost in dollars and the requests it
/// leaves out for want of a price. All `none` when the total is unknown.
fn token_values(total: TokenTotal, cost: Option<Cost>) -> [String; 7] {
    let (TokenTotal::Provisional(counts) | TokenTotal::Final(counts)) = total else {
        return ["none"; 7].map(str::to_owned);
    };
    let none = || "none".to_owned();
    [
        counts.input.to_string(),
        counts.cache_creation.to_string(),
        counts.cache_read.to_string(),
        counts.output.to_string(),
        counts
            .cache_hit_percent()
            .map_or_else(none, |percent| format!("{percent}%")),
        cost.as_ref().map_or_else(none, cost_usd),
        cost.map_or_else(none, |cost| cost.unpriced_requests.to_string()),
    ]
}

/// `input=30 cache_write=78405 cache_read=762619 output=13483 cache_hit=90%
/// cost_usd=0.441392 unpriced_requests=0`.
fn token_pairs(total: TokenTotal, cost: Option<Cost>) -> String {
    TOKEN_KEYS
        .into_iter()
        .zip(token_values(total, cost))
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join(" ")
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
