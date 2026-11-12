//! The skills a run opened that its role was not given, each with the words of
//! its `SKILL.md`, so that `rex job close` shows what agents read beyond their brief.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde_json::{Value, json};

use crate::bundle::{LIBRARY_ROOT, SKILLS_ROOT};
use crate::capture::JobRuns;
use crate::commands::skills::is_skill_name;
use crate::store::{JobId, StorePaths};

/// The role name printed for a run that carries no `Rex-Role` tag.
const NO_ROLE: &str = "-";

/// Skills whose files the builder's own text tells any role to read: the
/// delivery steps of a writer that starts parallel writers, and the rules
/// for the brief of any role that starts an agent.
const BUILDER_SKILLS: &[&str] = &["rex-deliver", "rex-agent-prompting"];

/// Roles that the builder starts outside the plan, each with the skill that
/// its brief names. Keep this in step with the builder roles of the
/// builder's decision text.
const BUILDER_ROLE_SKILLS: &[(&str, &str)] = &[
    ("decision-maker", "orchestration-builder"),
    ("decision-challenger", "orchestration-builder"),
    ("skill-writer", "rex-skill-language"),
    ("pr-reviewer", "rex-pr-review"),
];

/// What the close found about skills that runs opened beyond their role's list.
pub(super) struct ExtraSkills {
    /// False when no run's transcript was read, and the list is then empty.
    pub(super) has_read_runs: bool,
    /// Roles in the order of their first run, then skills by name.
    pub(super) extra: Vec<ExtraSkill>,
    /// The run record may miss facts, so the list may be wrong.
    pub(super) is_incomplete: bool,
}

pub(super) struct ExtraSkill {
    /// `None` for a run with no `Rex-Role` tag.
    role: Option<String>,
    skill: String,
    /// The words of the skill's `SKILL.md` now, not of the files the run read,
    /// which the record does not keep. `None` when no skill folder holds one.
    words: Option<usize>,
}

impl ExtraSkills {
    /// Compares the skills each run opened with the skills its role was given:
    /// its skills in the plan, the job's orchestrator, and what the builder's
    /// own text gives that role. A role that is not in the plan has no plan skills.
    ///
    /// # Errors
    /// The error of SQLite if the read of the roles fails.
    pub(super) fn read(
        conn: &Connection,
        job: &JobId,
        view: &JobRuns,
        paths: &StorePaths,
    ) -> rusqlite::Result<Self> {
        let mut statement =
            conn.prepare("SELECT role, skills FROM role WHERE job_id = ?1 AND skills IS NOT NULL")?;
        let given: BTreeMap<String, String> = statement
            .query_map([job.as_str()], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<_, _>>()?;
        let orchestrator: Option<String> = conn.query_row(
            "SELECT orchestrator_name FROM job WHERE id = ?1",
            [job.as_str()],
            |row| row.get(0),
        )?;

        // A role keeps the place of its first run, so the list reads in run order.
        let mut opened_by_role: Vec<(Option<&str>, BTreeSet<&str>)> = Vec::new();
        for run in &view.runs {
            let Some(opened) = &run.skills_opened else {
                continue;
            };
            let role = run.role.as_deref();
            let at = match opened_by_role.iter().position(|(known, _)| *known == role) {
                Some(at) => at,
                None => {
                    opened_by_role.push((role, BTreeSet::new()));
                    opened_by_role.len() - 1
                }
            };
            opened_by_role[at]
                .1
                .extend(opened.iter().map(String::as_str));
        }

        let folders = skill_folders(paths);
        let mut extra = Vec::new();
        for (role, opened) in &opened_by_role {
            let mut role_skills: BTreeSet<&str> = role
                .and_then(|role| given.get(role))
                .map(|skills| skills.split(',').collect())
                .unwrap_or_default();
            role_skills.extend(BUILDER_SKILLS);
            role_skills.extend(orchestrator.as_deref());
            role_skills.extend(
                BUILDER_ROLE_SKILLS
                    .iter()
                    .filter(|(builder_role, _)| Some(*builder_role) == *role)
                    .map(|(_, skill)| *skill),
            );
            for skill in opened.difference(&role_skills) {
                extra.push(ExtraSkill {
                    role: role.map(str::to_owned),
                    skill: (*skill).to_owned(),
                    words: skill_words(&folders, skill),
                });
            }
        }
        let has_read_runs = !opened_by_role.is_empty();
        Ok(ExtraSkills {
            has_read_runs,
            extra,
            is_incomplete: has_read_runs && (view.incomplete || view.damaged_log_lines > 0),
        })
    }

    /// The `extra_skills=` lines of the text answer.
    pub(super) fn push_text(&self, text: &mut String) {
        if !self.has_read_runs {
            return;
        }
        text.push_str(&format!("extra_skills={}\n", self.extra.len()));
        for (number, extra) in self.extra.iter().enumerate() {
            let words = extra
                .words
                .map_or_else(|| "none".to_owned(), |words| words.to_string());
            text.push_str(&format!(
                "extra_skill.{}={} {} skill_md_words={words}\n",
                number + 1,
                extra.role.as_deref().unwrap_or(NO_ROLE),
                extra.skill,
            ));
        }
    }

    pub(super) fn json(&self) -> Value {
        self.extra
            .iter()
            .map(|extra| json!({"role": extra.role, "skill": extra.skill, "skill_md_words": extra.words}))
            .collect()
    }
}

/// The folders that can hold a skill a run opened, in the order Claude Code
/// and the briefs look: the repo's skills, its library, its orchestrators,
/// then the user's skills.
fn skill_folders(paths: &StorePaths) -> Vec<PathBuf> {
    let mut folders = vec![
        paths.root.join(SKILLS_ROOT),
        paths.root.join(LIBRARY_ROOT),
        paths.orchestrators.clone(),
    ];
    if let Some(home) = std::env::var_os("HOME").filter(|home| !home.is_empty()) {
        folders.push(PathBuf::from(home).join(SKILLS_ROOT));
    }
    folders
}

/// The words of the first `SKILL.md` named `skill` in `folders`. Words, not
/// bytes, because the skill rules limit a `SKILL.md` in words.
fn skill_words(folders: &[PathBuf], skill: &str) -> Option<usize> {
    if !is_skill_name(skill) {
        return None;
    }
    folders
        .iter()
        .find_map(|folder| words_of(&folder.join(skill).join("SKILL.md")))
}

fn words_of(path: &Path) -> Option<usize> {
    fs::read_to_string(path)
        .ok()
        .map(|text| text.split_whitespace().count())
}
