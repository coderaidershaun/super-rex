//! The plan file of `rex job plan`: its form, and every check of it that
//! needs no store.

use std::collections::BTreeSet;
use std::path::Path;

use serde::Deserialize;

use crate::commands::job::JobError;
use crate::oms::Level;

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

impl RoleEntry {
    /// The four role inputs in the order intelligence, reliability,
    /// consequence, verification, or `None` for a role run as the session.
    pub(super) fn ratings(&self) -> Result<Option<[Option<Level>; 4]>, JobError> {
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
