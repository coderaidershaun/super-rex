//! `rex job plan`: records the plan and asks the selection function for each
//! role's agent, so the record and the function cannot disagree.

mod file;
mod output;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use rusqlite::{OptionalExtension, Transaction, params};

use super::JobError;
use super::input::read_text;
use super::value::{stored_level, value_name};
use crate::capture;
use crate::commands::skills;
use crate::commands::stdout::print_to_stdout;
use crate::oms::{self, Inputs};
use crate::store::orchestrator;
use crate::store::status::{self, Event};
use crate::store::{JobId, Store, StoreError, StorePaths};
use file::{PlanFile, RoleEntry, check};
use output::{PlannedRole, conflict_text, json_text, plain_text, rule_ids};

/// Arguments of `rex job plan`.
#[derive(clap::Args)]
#[command(
    after_help = "Prints job=<id> first, then status=, the orchestrator and its version, and one role.<name>= line per role: the grid agent to start that role with, or session."
)]
pub struct PlanOptions {
    /// The job to plan. It must be created and not yet planned.
    job: JobId,

    /// The plan, as JSON, or - for standard input.
    #[arg(long, value_name = "PATH")]
    plan_file: PathBuf,

    /// Print the result as one JSON line.
    #[arg(long)]
    json: bool,
}

/// # Errors
/// - [`JobError::ReadFile`], [`JobError::InvalidFile`] and the other plan file
///   variants for a plan file that cannot be read or is not valid.
/// - [`JobError::OrchestratorNotFound`] if the orchestrator has no folder.
/// - [`JobError::Store`]: the job is not found, is not `created`, or the store fails.
pub(super) fn run(cwd: &Path, options: PlanOptions) -> Result<(), JobError> {
    let mut store = Store::open(&StorePaths::discover(cwd)?)?;
    capture::take_in(&mut store)?;

    let path = &options.plan_file;
    let text = read_text(cwd, path).map_err(|source| JobError::ReadFile {
        file: "plan",
        path: path.clone(),
        source,
    })?;
    let plan: PlanFile = serde_json::from_str(&text).map_err(|source| JobError::InvalidFile {
        file: "plan",
        path: path.clone(),
        source,
    })?;
    check(&plan, path)?;

    let paths = store.paths().clone();
    let (version, roles) = store.write(|tx| record(tx, &paths, &options.job, &plan))?;

    let text = if options.json {
        json_text(&options.job, &plan.orchestrator, version, &roles)
    } else {
        plain_text(&options.job, &plan.orchestrator, version, &roles)
    };
    print_to_stdout(&text)?;
    Ok(())
}

/// The writes of `rex job plan`, all in the caller's one transaction.
fn record(
    tx: &Transaction<'_>,
    paths: &StorePaths,
    job: &JobId,
    plan: &PlanFile,
) -> Result<(i64, Vec<PlannedRole>), JobError> {
    let preferences: Option<Inputs> = tx
        .query_row(
            "SELECT pref_duration, pref_budget, pref_autonomy FROM job WHERE id = ?1",
            [job.as_str()],
            |row| {
                Ok(Inputs {
                    duration: stored_level(row, 0)?,
                    budget: stored_level(row, 1)?,
                    autonomy: stored_level(row, 2)?,
                    ..Inputs::default()
                })
            },
        )
        .optional()?;
    let preferences = preferences.ok_or_else(|| StoreError::JobNotFound {
        job: job.to_string(),
        store: paths.dir.clone(),
    })?;
    status::apply(tx, job, Event::Plan)?;

    let name = &plan.orchestrator;
    let version = orchestrator_version(tx, paths, name)?;
    let roles = record_roles(tx, job, &plan.roles, preferences)?;

    let skills: BTreeSet<&str> = plan.skills.iter().map(String::as_str).collect();
    let skills = (!skills.is_empty()).then(|| skills.into_iter().collect::<Vec<_>>().join(","));
    tx.execute(
        "UPDATE job SET breakdown = ?1, skills = ?2, orchestrator_name = ?3,
                        orchestrator_version = ?4,
                        updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         WHERE id = ?5",
        params![plan.breakdown, skills, name, version, job.as_str()],
    )?;
    Ok((version, roles))
}

/// Indexes the whole library, so that orchestrators never run also reach the
/// index, and gives the current version of `name`.
fn orchestrator_version(
    tx: &Transaction<'_>,
    paths: &StorePaths,
    name: &str,
) -> Result<i64, JobError> {
    let not_found = || JobError::OrchestratorNotFound {
        name: name.to_owned(),
        store: paths.dir.clone(),
    };
    if !paths.orchestrators.join(name).is_dir() {
        return Err(not_found());
    }
    let versions = orchestrator::index_all(tx, &paths.orchestrators, &skills::describe)?;
    let version = versions
        .iter()
        .find(|indexed| indexed.name == name)
        .ok_or_else(not_found)?
        .version;
    Ok(version)
}

/// Asks the selection function for each role not run as the session, with the
/// job's three preferences, and writes each role row in the order of the file.
fn record_roles(
    tx: &Transaction<'_>,
    job: &JobId,
    entries: &[RoleEntry],
    preferences: Inputs,
) -> Result<Vec<PlannedRole>, JobError> {
    let mut roles = Vec::with_capacity(entries.len());
    for entry in entries {
        let selection = entry.ratings()?.map(|ratings| {
            oms::select(Inputs {
                duration: preferences.duration,
                budget: preferences.budget,
                autonomy: preferences.autonomy,
                ..ratings
            })
        });
        let role = PlannedRole {
            name: entry.role.clone(),
            selection,
            skills: entry.skills.clone(),
        };
        insert_role(tx, job, &role, preferences)?;
        roles.push(role);
    }
    Ok(roles)
}

/// Writes one role row. The job's three preferences and the role's skills are
/// stored on every role, and the selection function's answer only on a role not
/// run as the session.
fn insert_role(
    tx: &Transaction<'_>,
    job: &JobId,
    role: &PlannedRole,
    preferences: Inputs,
) -> rusqlite::Result<()> {
    let selection = role.selection.as_ref();
    let inputs = selection.map(|chosen| chosen.inputs);
    let skills = (!role.skills.is_empty()).then(|| role.skills.join(","));
    tx.execute(
        "INSERT INTO role (job_id, role, runs_as_session, in_duration, in_budget, in_autonomy,
                           in_intelligence, in_reliability, in_consequence, in_verification,
                           agent, demand, rules, conflict, oms_version, skills)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
        params![
            job.as_str(),
            role.name,
            selection.is_none(),
            preferences.duration.map(value_name),
            preferences.budget.map(value_name),
            preferences.autonomy.map(value_name),
            inputs.and_then(|i| i.intelligence).map(value_name),
            inputs.and_then(|i| i.reliability).map(value_name),
            inputs.and_then(|i| i.consequence).map(value_name),
            inputs.and_then(|i| i.verification).map(value_name),
            selection.map(|chosen| chosen.agent.to_string()),
            selection.map(|chosen| chosen.demand.to_string()),
            selection.map(|chosen| rule_ids(&chosen.rules).join(",")),
            selection.map(|chosen| conflict_text(&chosen.conflict)),
            selection.map(|_| oms::VERSION),
            skills,
        ],
    )?;
    Ok(())
}
