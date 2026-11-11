//! The skills a role was assigned that no run of that role opened, so that
//! `rex job close` shows a rulebook a worker never read instead of hiding it.

use std::collections::BTreeSet;

use rusqlite::Connection;

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
        // A role run as the session is left out: the main session's reads of
        // skills are not recorded, so each of its skills would look missed.
        let mut statement = conn.prepare(
            "SELECT role, skills FROM role
             WHERE job_id = ?1 AND runs_as_session = 0 AND skills IS NOT NULL
             ORDER BY rowid",
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
                .flat_map(|run| run.skills_opened.iter().map(String::as_str))
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
}
