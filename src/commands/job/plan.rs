//! `rex job plan`: records the plan and asks the selection function for each
//! role's agent, so the record and the function cannot disagree.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use rusqlite::types::Type;
use rusqlite::{OptionalExtension, Row, Transaction, params};
use serde::Deserialize;
use serde_json::{Value, json};

use super::input::read_text;
use super::{JobError, describe, runs};
use crate::commands::stdout::print_to_stdout;
use crate::oms::{self, Inputs, Level, Overridden, Rule, Selection};
use crate::store::orchestrator;
use crate::store::status::{self, Event};
use crate::store::{JobId, Store, StoreError, StorePaths};

/// Arguments of `rex job plan`.
#[derive(clap::Args)]
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlanFile {
    orchestrator: String,
    skills: Vec<String>,
    breakdown: String,
    roles: Vec<RoleEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RoleEntry {
    role: String,
    #[serde(default)]
    session: bool,
    intelligence: Option<Rating>,
    reliability: Option<Rating>,
    consequence: Option<Rating>,
    verification: Option<Rating>,
}

/// A role input as the plan file writes it.
#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum Rating {
    Low,
    Medium,
    High,
    None,
}

/// A role as planned: its OMS answer, or `None` when it runs as the session.
struct PlannedRole {
    name: String,
    selection: Option<Selection>,
}

/// # Errors
/// - [`JobError::ReadFile`], [`JobError::InvalidFile`] and the other plan file
///   variants for a plan file that cannot be read or is not valid.
/// - [`JobError::OrchestratorNotFound`] if the orchestrator has no folder.
/// - [`JobError::Store`]: the job is not found, is not `created`, or the store fails.
pub(super) fn run(cwd: &Path, options: PlanOptions) -> Result<(), JobError> {
    let mut store = Store::open(&StorePaths::discover(cwd)?)?;
    runs::take_in(&mut store)?;

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

/// Every check of the plan file that needs no store.
fn check(plan: &PlanFile, path: &Path) -> Result<(), JobError> {
    let path = path.to_path_buf();
    if plan.roles.is_empty() {
        return Err(JobError::NoRoles { path });
    }
    if !is_name(&plan.orchestrator) {
        return Err(JobError::BadOrchestratorName {
            name: plan.orchestrator.clone(),
            path,
        });
    }
    if let Some(name) = plan
        .skills
        .iter()
        .find(|name| name.is_empty() || name.contains(|c: char| c == ',' || c.is_whitespace()))
    {
        return Err(JobError::BadSkillName {
            name: name.clone(),
            path,
        });
    }
    if plan.breakdown.trim().is_empty() {
        return Err(JobError::EmptyBreakdown { path });
    }
    let mut seen = BTreeSet::new();
    for entry in &plan.roles {
        if !is_name(&entry.role) {
            return Err(JobError::BadRoleName {
                role: entry.role.clone(),
                path,
            });
        }
        if !seen.insert(entry.role.as_str()) {
            return Err(JobError::RoleTwice {
                path,
                role: entry.role.clone(),
            });
        }
        entry.ratings()?;
    }
    Ok(())
}

/// The writes of `rex job plan`, all in the caller's one transaction.
fn record(
    tx: &Transaction<'_>,
    paths: &StorePaths,
    job: &JobId,
    plan: &PlanFile,
) -> Result<(i64, Vec<PlannedRole>), JobError> {
    let preferences: Option<[Option<Level>; 3]> = tx
        .query_row(
            "SELECT pref_duration, pref_budget, pref_autonomy FROM job WHERE id = ?1",
            [job.as_str()],
            |row| {
                Ok([
                    stored_level(row, 0)?,
                    stored_level(row, 1)?,
                    stored_level(row, 2)?,
                ])
            },
        )
        .optional()?;
    let [duration, budget, autonomy] = preferences.ok_or_else(|| StoreError::JobNotFound {
        job: job.to_string(),
        store: paths.dir.clone(),
    })?;
    status::apply(tx, job, Event::Plan)?;

    let name = &plan.orchestrator;
    let not_found = || JobError::OrchestratorNotFound {
        name: name.clone(),
        store: paths.dir.clone(),
    };
    if !paths.orchestrators.join(name).is_dir() {
        return Err(not_found());
    }
    let versions = orchestrator::index_all(tx, &paths.orchestrators, &describe::description)?;
    let version = versions
        .iter()
        .find(|indexed| &indexed.name == name)
        .ok_or_else(not_found)?
        .version;

    let mut roles = Vec::with_capacity(plan.roles.len());
    for entry in &plan.roles {
        let selection =
            entry
                .ratings()?
                .map(|[intelligence, reliability, consequence, verification]| {
                    oms::select(Inputs {
                        duration,
                        budget,
                        intelligence,
                        autonomy,
                        reliability,
                        consequence,
                        verification,
                    })
                });
        insert_role(
            tx,
            job,
            &entry.role,
            [duration, budget, autonomy],
            selection.as_ref(),
        )?;
        roles.push(PlannedRole {
            name: entry.role.clone(),
            selection,
        });
    }

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

/// Writes one role row. The job's three preferences are stored on every role,
/// and the OMS record only on a role that is not run as the session.
fn insert_role(
    tx: &Transaction<'_>,
    job: &JobId,
    role: &str,
    [duration, budget, autonomy]: [Option<Level>; 3],
    selection: Option<&Selection>,
) -> rusqlite::Result<()> {
    let inputs = selection.map(|chosen| chosen.inputs);
    tx.execute(
        "INSERT INTO role (job_id, role, runs_as_session, in_duration, in_budget, in_autonomy,
                           in_intelligence, in_reliability, in_consequence, in_verification,
                           agent, demand, rules, conflict, oms_version)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        params![
            job.as_str(),
            role,
            selection.is_none(),
            duration.map(level_name),
            budget.map(level_name),
            autonomy.map(level_name),
            inputs.and_then(|i| i.intelligence).map(level_name),
            inputs.and_then(|i| i.reliability).map(level_name),
            inputs.and_then(|i| i.consequence).map(level_name),
            inputs.and_then(|i| i.verification).map(level_name),
            selection.map(|chosen| chosen.agent.to_string()),
            selection.map(|chosen| chosen.demand.to_string()),
            selection.map(|chosen| rule_ids(&chosen.rules).join(",")),
            selection.map(|chosen| conflict_text(&chosen.conflict)),
            selection.map(|_| oms::VERSION),
        ],
    )?;
    Ok(())
}

fn plain_text(job: &JobId, orchestrator: &str, version: i64, roles: &[PlannedRole]) -> String {
    let mut text = format!(
        "job={job}\nstatus=planned\norchestrator={orchestrator}\norchestrator_version={version}\n"
    );
    for role in roles {
        let Some(chosen) = &role.selection else {
            text.push_str(&format!("role.{}=session\n", role.name));
            continue;
        };
        text.push_str(&format!("role.{}={}\n", role.name, chosen.agent));
        if !chosen.conflict.is_empty() {
            text.push_str(&format!(
                "conflict.{}={}\n",
                role.name,
                conflict_text(&chosen.conflict)
            ));
        }
    }
    text
}

fn json_text(job: &JobId, orchestrator: &str, version: i64, roles: &[PlannedRole]) -> String {
    let roles: Vec<Value> = roles
        .iter()
        .map(|role| match &role.selection {
            Some(chosen) => json!({
                "role": role.name,
                "session": false,
                "agent": chosen.agent.to_string(),
                "demand": chosen.demand.to_string(),
                "rules": rule_ids(&chosen.rules),
                "conflict": chosen.conflict.iter().map(|c| c.name()).collect::<Vec<_>>(),
                "oms_version": oms::VERSION,
            }),
            None => json!({
                "role": role.name,
                "session": true,
                "agent": null,
                "demand": null,
                "rules": null,
                "conflict": null,
                "oms_version": null,
            }),
        })
        .collect();
    let line = json!({
        "schema": 1,
        "job": job.as_str(),
        "status": "planned",
        "orchestrator": {"name": orchestrator, "version": version},
        "roles": roles,
    });
    format!("{line}\n")
}

impl RoleEntry {
    /// The four role inputs in the order intelligence, reliability,
    /// consequence, verification, or `None` for a role run as the session.
    fn ratings(&self) -> Result<Option<[Option<Level>; 4]>, JobError> {
        let given = [
            ("intelligence", self.intelligence),
            ("reliability", self.reliability),
            ("consequence", self.consequence),
            ("verification", self.verification),
        ];
        if self.session {
            if given.iter().any(|(_, rating)| rating.is_some()) {
                return Err(JobError::SessionAndInputs {
                    role: self.role.clone(),
                });
            }
            return Ok(None);
        }
        let mut levels = [None; 4];
        for (level, (key, rating)) in levels.iter_mut().zip(given) {
            let rating = rating.ok_or_else(|| JobError::MissingInput {
                role: self.role.clone(),
                key,
            })?;
            *level = rating.level();
        }
        Ok(Some(levels))
    }
}

impl Rating {
    fn level(self) -> Option<Level> {
        match self {
            Rating::Low => Some(Level::Low),
            Rating::Medium => Some(Level::Medium),
            Rating::High => Some(Level::High),
            Rating::None => None,
        }
    }
}

/// A role or orchestrator name: lower-case letters, digits and `-`.
fn is_name(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn rule_ids(rules: &[Rule]) -> Vec<&'static str> {
    rules.iter().map(|rule| rule.id()).collect()
}

/// The stored conflict: the overridden caps joined by commas, or `none`.
fn conflict_text(conflict: &[Overridden]) -> String {
    if conflict.is_empty() {
        return "none".to_owned();
    }
    let names: Vec<&str> = conflict
        .iter()
        .map(|overridden| overridden.name())
        .collect();
    names.join(",")
}

/// The stored text of a rating.
pub(super) fn level_name(level: Level) -> &'static str {
    match level {
        Level::Low => "low",
        Level::Medium => "medium",
        Level::High => "high",
    }
}

/// A stored rating column: `low`, `medium`, `high`, or NULL for none.
pub(super) fn stored_level(row: &Row<'_>, index: usize) -> rusqlite::Result<Option<Level>> {
    let text: Option<String> = row.get(index)?;
    text.map(|text| match text.as_str() {
        "low" => Ok(Level::Low),
        "medium" => Ok(Level::Medium),
        "high" => Ok(Level::High),
        _ => Err(rusqlite::Error::FromSqlConversionFailure(
            index,
            Type::Text,
            format!("{text} is not a rating; one of low, medium, high").into(),
        )),
    })
    .transpose()
}
