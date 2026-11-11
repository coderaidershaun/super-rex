//! The skills a role was assigned that no run of that role opened, so that
//! `rex job close` shows a rulebook a worker never read instead of hiding it.

use std::collections::BTreeSet;

use rusqlite::Connection;
use serde_json::{Value, json};

use crate::capture::JobRuns;
use crate::store::JobId;

/// What the close found about the skills assigned to roles.
pub(super) struct SkillCheck {
    /// False when no role has skills to check, and the lists are then empty.
    pub(super) has_assigned: bool,
    /// In plan order of the roles, then in the order of each role's skills.
    pub(super) missed: Vec<MissedSkill>,
    /// Roles with skills and no run at all, whose skills are not listed one by one.
    pub(super) roles_not_run: Vec<String>,
    /// The run record may miss facts, so the lists may be wrong.
    pub(super) is_incomplete: bool,
}

pub(super) struct MissedSkill {
    pub(super) role: String,
    pub(super) skill: String,
}

impl SkillCheck {
    /// Compares each role's skills with the skills its runs opened.
    ///
    /// # Errors
    /// The error of SQLite if the read of the roles fails.
    pub(super) fn read(conn: &Connection, job: &JobId, view: &JobRuns) -> rusqlite::Result<Self> {
        let mut statement = conn.prepare(
            "SELECT role, skills FROM role WHERE job_id = ?1 AND skills IS NOT NULL ORDER BY rowid",
        )?;
        let assigned = statement
            .query_map([job.as_str()], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut check = SkillCheck {
            has_assigned: !assigned.is_empty(),
            missed: Vec::new(),
            roles_not_run: Vec::new(),
            is_incomplete: !assigned.is_empty() && (view.incomplete || view.damaged_log_lines > 0),
        };
        for (role, skills) in assigned {
            let runs: Vec<_> = view
                .runs
                .iter()
                .filter(|run| run.role.as_deref() == Some(role.as_str()))
                .collect();
            if runs.is_empty() {
                check.roles_not_run.push(role);
                continue;
            }
            let opened: BTreeSet<&str> = runs
                .iter()
                .flat_map(|run| run.skills_opened.iter().flatten().map(String::as_str))
                .collect();
            for skill in skills.split(',').filter(|skill| !opened.contains(skill)) {
                check.missed.push(MissedSkill {
                    role: role.clone(),
                    skill: skill.to_owned(),
                });
            }
        }
        Ok(check)
    }
    /// The `missed_skills=` and `role_not_run.` lines of the text answer.
    pub(super) fn push_text(&self, text: &mut String) {
        if self.has_assigned {
            text.push_str(&format!("missed_skills={}\n", self.missed.len()));
        }
        for (number, missed) in self.missed.iter().enumerate() {
            text.push_str(&format!(
                "missed_skill.{}={} {}\n",
                number + 1,
                missed.role,
                missed.skill
            ));
        }
        for (number, role) in self.roles_not_run.iter().enumerate() {
            text.push_str(&format!("role_not_run.{}={role}\n", number + 1));
        }
    }

    pub(super) fn missed_json(&self) -> Value {
        self.missed
            .iter()
            .map(|missed| json!({"role": missed.role, "skill": missed.skill}))
            .collect()
    }
}
