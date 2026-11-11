//! The answer that `rex job plan` prints, as `key=value` lines or one JSON
//! line. The role rows store the same rule and conflict text.

use serde_json::{Value, json};

use crate::oms::{self, Overridden, Rule, Selection};
use crate::store::JobId;

/// A role as planned, with the selection function's answer.
pub(super) struct PlannedRole {
    pub(super) name: String,
    pub(super) selection: Selection,
    pub(super) skills: Vec<String>,
}

pub(super) fn plain_text(
    job: &JobId,
    orchestrator: &str,
    version: i64,
    roles: &[PlannedRole],
) -> String {
    let mut text = format!(
        "job={job}\nstatus=planned\norchestrator={orchestrator}\norchestrator_version={version}\n"
    );
    for role in roles {
        let chosen = &role.selection;
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

pub(super) fn json_text(
    job: &JobId,
    orchestrator: &str,
    version: i64,
    roles: &[PlannedRole],
) -> String {
    let roles: Vec<Value> = roles
        .iter()
        .map(|role| {
            let chosen = &role.selection;
            json!({
                "role": role.name,
                "agent": chosen.agent.to_string(),
                "demand": chosen.demand.to_string(),
                "rules": rule_ids(&chosen.rules),
                "conflict": chosen.conflict.iter().map(|c| c.name()).collect::<Vec<_>>(),
                "oms_version": oms::VERSION,
            })
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

pub(super) fn rule_ids(rules: &[Rule]) -> Vec<&'static str> {
    rules.iter().map(|rule| rule.id()).collect()
}

/// The overridden caps joined by commas, or `none`.
pub(super) fn conflict_text(conflict: &[Overridden]) -> String {
    if conflict.is_empty() {
        return "none".to_owned();
    }
    let names: Vec<&str> = conflict
        .iter()
        .map(|overridden| overridden.name())
        .collect();
    names.join(",")
}
