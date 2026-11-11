//! The plan file of `rex job plan`: its form, and every check of it that
//! needs no store.

use std::collections::BTreeSet;
use std::path::Path;

use serde::Deserialize;

use crate::commands::job::JobError;
use crate::commands::skills::is_skill_name;
use crate::oms::{Inputs, Level};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PlanFile {
    pub(super) orchestrator: String,
    pub(super) skills: Vec<String>,
    pub(super) breakdown: String,
    pub(super) roles: Vec<RoleEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RoleEntry {
    pub(super) role: String,
    intelligence: Option<Rating>,
    reliability: Option<Rating>,
    consequence: Option<Rating>,
    verification: Option<Rating>,
    /// Each name is also in the job's `skills`.
    #[serde(default)]
    pub(super) skills: Vec<String>,
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

/// Every check of the plan file that needs no store.
pub(super) fn check(plan: &PlanFile, path: &Path) -> Result<(), JobError> {
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
    if let Some(name) = plan.skills.iter().find(|name| !is_skill_name(name)) {
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
        check_role_skills(entry, &plan.skills, &path)?;
    }
    Ok(())
}

/// A role's skills are valid names, each in the job's skills, each given once.
fn check_role_skills(
    entry: &RoleEntry,
    job_skills: &[String],
    path: &Path,
) -> Result<(), JobError> {
    let mut seen = BTreeSet::new();
    for name in &entry.skills {
        let path = path.to_path_buf();
        if !is_skill_name(name) {
            return Err(JobError::BadSkillName {
                name: name.clone(),
                path,
            });
        }
        if !job_skills.contains(name) {
            return Err(JobError::RoleSkillNotInJob {
                role: entry.role.clone(),
                name: name.clone(),
                path,
            });
        }
        if !seen.insert(name.as_str()) {
            return Err(JobError::RoleSkillTwice {
                role: entry.role.clone(),
                name: name.clone(),
                path,
            });
        }
    }
    Ok(())
}

impl RoleEntry {
    /// The role's four inputs, with the job's three left unset.
    pub(super) fn ratings(&self) -> Result<Inputs, JobError> {
        let level = |key: &'static str, rating: Option<Rating>| {
            rating
                .map(Rating::level)
                .ok_or_else(|| JobError::MissingInput {
                    role: self.role.clone(),
                    key,
                })
        };
        Ok(Inputs {
            intelligence: level("intelligence", self.intelligence)?,
            reliability: level("reliability", self.reliability)?,
            consequence: level("consequence", self.consequence)?,
            verification: level("verification", self.verification)?,
            ..Inputs::default()
        })
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
