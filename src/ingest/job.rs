//! The job events of a batch: open, plan, amend, close, and the answers, limits,
//! marks, owners and decisions that change a job's row.

use rusqlite::{ErrorCode, OptionalExtension, params};

use super::batch::Event;
use super::data::{
    CloseStatus, Decision, JobAmend, JobClose, JobLimit, JobOpen, JobPlan, Level, Mark,
    MergeAnswer, Rating, Rebind, Role, Task,
};
use super::{Apply, IngestError};
use crate::oms::{self, Inputs};
use crate::store::integer::signed;
use crate::store::orchestrator;
use crate::store::status::{self, Closed, Event as StatusEvent};
use crate::store::{JobId, StoreError};

/// The version of the classification lists, for the column a job of rex 0.7 had.
const CLASS_VERSION: i64 = 1;

impl Apply<'_> {
    pub(super) fn job_open(&self, event: &Event, data: &JobOpen) -> Result<(), IngestError> {
        let job = event.job()?;
        let inserted = self.tx.execute(
            "INSERT INTO job (id, request, origin, restatement, class_version, work_type, size,
                              consequence, languages, pref_duration, pref_budget, pref_autonomy,
                              merge_agreed, session_id, builder_agent_id, repo, branch, base,
                              worktree, tree_id, limit_nano, limit_source, status, created_at,
                              updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 0, ?13, ?14, ?15, ?16,
                     ?17, ?18, ?19, ?20, ?21, 'created', ?22, ?22)",
            params![
                job.as_str(),
                data.request,
                data.origin.as_str(),
                data.restatement,
                CLASS_VERSION,
                data.work_type.map(|value| value.as_str()),
                data.size.map(|value| value.as_str()),
                data.consequence.map(Level::as_str),
                comma_list(&data.langs).unwrap_or_else(|| "none".to_owned()),
                data.prefs.duration.map(Level::as_str),
                data.prefs.budget.map(Level::as_str),
                data.prefs.autonomy.map(Level::as_str),
                data.owner.session,
                data.owner.builder,
                data.repo,
                data.branch,
                data.base,
                data.worktree,
                data.tree,
                signed(data.limit_nano),
                data.limit_source.as_str(),
                event.at,
            ],
        );
        fits(event, inserted, || {
            format!(
                "job {job} is in the store already, or its owner ({}, {}) has an open job",
                data.owner.session, data.owner.builder
            )
        })?;
        self.tx
            .execute(
                "INSERT INTO job_transition (job_id, from_status, to_status, event, at)
                 VALUES (?1, NULL, 'created', 'job open', ?2)",
                params![job.as_str(), event.at],
            )
            .map_err(StoreError::from)?;
        Ok(())
    }

    pub(super) fn job_plan(&self, event: &Event, data: &JobPlan) -> Result<(), IngestError> {
        let job = event.job()?;
        self.move_status(event, job, StatusEvent::Plan)?;
        let orchestrator = self.orchestrator_version(&data.orchestrator)?;
        let skills = comma_list(data.tasks.iter().flat_map(|task| task.skills.iter()));
        self.update_job(
            event,
            "UPDATE job SET breakdown = ?2, orchestrator_name = ?3, orchestrator_version = ?4,
                            skills = ?5, updated_at = ?6
             WHERE id = ?1",
            params![
                job.as_str(),
                data.breakdown,
                data.orchestrator,
                orchestrator,
                skills,
                event.at
            ],
        )?;
        self.upsert_roles(event, job, &data.roles, &data.tasks)?;
        for entry in &data.manifest {
            self.tx
                .execute(
                    "INSERT INTO manifest (job_id, skill, file, hash, source) VALUES (?1, ?2, ?3, ?4, ?5)
                     ON CONFLICT (job_id, skill, file) DO UPDATE SET hash = excluded.hash, source = excluded.source",
                    params![job.as_str(), entry.skill, entry.file, entry.hash, entry.source.as_str()],
                )
                .map_err(StoreError::from)?;
        }
        Ok(())
    }

    /// Replaces the rows of the roles the amendment names and keeps the rest.
    pub(super) fn job_amend(&self, event: &Event, data: &JobAmend) -> Result<(), IngestError> {
        let job = event.job()?;
        self.upsert_roles(event, job, &data.roles, &data.tasks)?;
        let role_skills: Vec<Option<String>> = {
            let mut statement = self
                .tx
                .prepare("SELECT skills FROM role WHERE job_id = ?1")
                .map_err(StoreError::from)?;
            let rows = statement
                .query_map([job.as_str()], |row| row.get(0))
                .map_err(StoreError::from)?;
            rows.collect::<Result<_, _>>().map_err(StoreError::from)?
        };
        let skills = comma_list(
            role_skills
                .iter()
                .flatten()
                .flat_map(|list| list.split(',')),
        );
        self.update_job(
            event,
            "UPDATE job SET skills = ?2, updated_at = ?3 WHERE id = ?1",
            params![job.as_str(), skills, event.at],
        )
    }

    pub(super) fn job_close(&self, event: &Event, data: &JobClose) -> Result<(), IngestError> {
        let job = event.job()?;
        let closed = match data.status {
            CloseStatus::Delivered => Closed::Delivered,
            CloseStatus::Blocked => Closed::Blocked,
            CloseStatus::Failed => Closed::Failed,
            CloseStatus::Cancelled => Closed::Cancelled,
            CloseStatus::Lost => Closed::Lost,
        };
        self.move_status(event, job, StatusEvent::Close(closed))?;
        self.update_job(
            event,
            "UPDATE job SET outcome = ?2, updated_at = ?3 WHERE id = ?1",
            params![job.as_str(), data.outcome, event.at],
        )?;
        for delivery in &data.deliveries {
            let written = self.tx.execute(
                "INSERT INTO delivery (job_id, repo, branch, pr, merge_agreed, merge_commit, outcome, note, recorded_at)
                 SELECT ?1, ?2, ?3, ?4, merge_agreed, ?5, ?6, ?7, ?8 FROM job WHERE id = ?1
                 ON CONFLICT (job_id, repo) DO UPDATE SET
                     branch = excluded.branch, pr = excluded.pr, merge_agreed = excluded.merge_agreed,
                     merge_commit = excluded.merge_commit, outcome = excluded.outcome,
                     note = excluded.note, recorded_at = excluded.recorded_at",
                params![
                    job.as_str(),
                    delivery.repo,
                    delivery.branch,
                    delivery.pr,
                    delivery.merge_commit,
                    delivery.outcome.as_str(),
                    delivery.note,
                    event.at,
                ],
            );
            fits(event, written, || {
                format!(
                    "the delivery to {} is {} with no {}",
                    delivery.repo,
                    delivery.outcome.as_str(),
                    if delivery.outcome.as_str() == "merged" {
                        "merge_commit"
                    } else {
                        "pr"
                    }
                )
            })?;
        }
        Ok(())
    }

    pub(super) fn job_merge_answer(
        &self,
        event: &Event,
        data: &MergeAnswer,
    ) -> Result<(), IngestError> {
        let job = event.job()?;
        self.update_job(
            event,
            "UPDATE job SET merge_agreed = ?2, merge_source = ?3, merge_question = ?4,
                            policy_hash = ?5, updated_at = ?6
             WHERE id = ?1",
            params![
                job.as_str(),
                data.answer.as_str() == "yes",
                data.source.as_str(),
                data.question,
                data.policy_hash,
                event.at
            ],
        )
    }

    pub(super) fn job_limit(&self, event: &Event, data: &JobLimit) -> Result<(), IngestError> {
        let job = event.job()?;
        self.update_job(
            event,
            "UPDATE job SET limit_nano = ?2, limit_source = ?3, updated_at = ?4 WHERE id = ?1",
            params![
                job.as_str(),
                signed(data.limit_nano),
                data.source.as_str(),
                event.at
            ],
        )
    }

    /// Adds the mark to the job's list, once.
    pub(super) fn job_degraded(&self, event: &Event, data: &Mark) -> Result<(), IngestError> {
        let job = event.job()?;
        self.update_job(
            event,
            "UPDATE job SET degraded = CASE
                 WHEN degraded IS NULL OR degraded = '' THEN ?2
                 WHEN instr(',' || degraded || ',', ',' || ?2 || ',') > 0 THEN degraded
                 ELSE degraded || ',' || ?2 END,
               updated_at = ?3
             WHERE id = ?1",
            params![job.as_str(), data.mark, event.at],
        )
    }

    pub(super) fn job_rebind(&self, event: &Event, data: &Rebind) -> Result<(), IngestError> {
        let job = event.job()?;
        self.update_job(
            event,
            "UPDATE job SET session_id = ?2, builder_agent_id = ?3, updated_at = ?4 WHERE id = ?1",
            params![
                job.as_str(),
                data.owner.session,
                data.owner.builder,
                event.at
            ],
        )
    }

    /// The first decision of an assignment is kept; a second of one id is the same answer.
    pub(super) fn decision(&self, event: &Event, data: &Decision) -> Result<(), IngestError> {
        let job = event.job()?;
        let inserted = self.tx.execute(
            "INSERT INTO decision (assignment_id, job_id, subject, item, value, source, reason,
                                   table_version, overlay_version, at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT (assignment_id) DO NOTHING",
            params![
                data.assignment_id,
                job.as_str(),
                data.subject,
                data.item,
                data.value,
                data.source.as_str(),
                data.reason,
                data.table_version,
                data.overlay_version,
                event.at,
            ],
        );
        fits(event, inserted, || format!("no job {job} in the store"))?;
        Ok(())
    }

    /// Writes one row for each role, with what the selection function gives for
    /// the job's preferences and the role's ratings, and the skills of its tasks.
    /// The agent is the one the plugin chose, which a cap of a master may hold
    /// below the function's; the rest is the function's own, so the record and
    /// the function never disagree.
    fn upsert_roles(
        &self,
        event: &Event,
        job: &JobId,
        roles: &[Role],
        tasks: &[Task],
    ) -> Result<(), IngestError> {
        let prefs: Option<(Option<String>, Option<String>, Option<String>)> = self
            .tx
            .query_row(
                "SELECT pref_duration, pref_budget, pref_autonomy FROM job WHERE id = ?1",
                [job.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(StoreError::from)?;
        let Some((duration, budget, autonomy)) = prefs else {
            return Err(event.does_not_fit(format!("no job {job} in the store")));
        };
        for role in roles {
            let inputs = Inputs {
                duration: stored_level(duration.as_deref()),
                budget: stored_level(budget.as_deref()),
                autonomy: stored_level(autonomy.as_deref()),
                intelligence: rating(role.intelligence),
                reliability: rating(role.reliability),
                consequence: rating(role.consequence),
                verification: rating(role.verification),
            };
            let chosen = oms::select(inputs);
            let rules: Vec<&str> = chosen.rules.iter().map(|rule| rule.id()).collect();
            let conflict: Vec<&str> = chosen.conflict.iter().map(|cap| cap.name()).collect();
            let conflict = if conflict.is_empty() {
                "none".to_owned()
            } else {
                conflict.join(",")
            };
            let skills = comma_list(
                tasks
                    .iter()
                    .filter(|task| task.role == role.role)
                    .flat_map(|task| task.skills.iter()),
            );
            self.tx
                .execute(
                    "INSERT INTO role (job_id, role, in_duration, in_budget, in_autonomy,
                                       in_intelligence, in_reliability, in_consequence,
                                       in_verification, agent, demand, rules, conflict,
                                       oms_version, skills, source)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, 'plan')
                     ON CONFLICT (job_id, role) DO UPDATE SET
                         in_intelligence = excluded.in_intelligence,
                         in_reliability = excluded.in_reliability,
                         in_consequence = excluded.in_consequence,
                         in_verification = excluded.in_verification,
                         agent = excluded.agent, demand = excluded.demand, rules = excluded.rules,
                         conflict = excluded.conflict, oms_version = excluded.oms_version,
                         skills = excluded.skills",
                    params![
                        job.as_str(),
                        role.role,
                        duration,
                        budget,
                        autonomy,
                        rating_text(role.intelligence),
                        rating_text(role.reliability),
                        rating_text(role.consequence),
                        rating_text(role.verification),
                        role.grid_agent,
                        chosen.demand.to_string(),
                        rules.join(","),
                        conflict,
                        signed(role.oms_version),
                        skills,
                    ],
                )
                .map_err(StoreError::from)?;
        }
        Ok(())
    }

    /// Moves the job's status by the one transition table, at the event's time.
    fn move_status(
        &self,
        event: &Event,
        job: &JobId,
        change: StatusEvent,
    ) -> Result<(), IngestError> {
        match status::apply_at(self.tx, job, change, &event.at) {
            Ok(_) => Ok(()),
            Err(error @ (StoreError::Refused { .. } | StoreError::JobNotFound { .. })) => {
                Err(event.does_not_fit(error.to_string()))
            }
            Err(error) => Err(error.into()),
        }
    }

    /// The version of the orchestrator folder `name` in the library, indexed
    /// now; `None` when the library has no such folder.
    fn orchestrator_version(&self, name: &str) -> Result<Option<i64>, IngestError> {
        if !self.orchestrators.join(name).is_dir() {
            return Ok(None);
        }
        let indexed = orchestrator::index(self.tx, self.orchestrators, name, self.describe)?;
        Ok(Some(indexed.version))
    }

    /// Runs `sql` on the job of `event`, and refuses the event when no job has its ID.
    fn update_job(
        &self,
        event: &Event,
        sql: &str,
        params: impl rusqlite::Params,
    ) -> Result<(), IngestError> {
        let job = event.job()?;
        let changed = fits(event, self.tx.execute(sql, params), || {
            format!("the change does not fit job {job}")
        })?;
        if changed == 0 {
            return Err(event.does_not_fit(format!("no job {job} in the store")));
        }
        Ok(())
    }
}

/// The result of a write, with a broken rule of the store turned into a
/// refusal of the event that `reason` explains.
pub(super) fn fits<T>(
    event: &Event,
    result: rusqlite::Result<T>,
    reason: impl FnOnce() -> String,
) -> Result<T, IngestError> {
    match result {
        Ok(value) => Ok(value),
        Err(rusqlite::Error::SqliteFailure(failure, _))
            if failure.code == ErrorCode::ConstraintViolation =>
        {
            Err(event.does_not_fit(reason()))
        }
        Err(error) => Err(StoreError::from(error).into()),
    }
}

/// The names sorted, without repeats, joined by commas; `None` when there are none.
pub(super) fn comma_list<S: AsRef<str>>(names: impl IntoIterator<Item = S>) -> Option<String> {
    let mut names: Vec<String> = names
        .into_iter()
        .map(|name| name.as_ref().to_owned())
        .filter(|name| !name.is_empty())
        .collect();
    names.sort_unstable();
    names.dedup();
    if names.is_empty() {
        None
    } else {
        Some(names.join(","))
    }
}

fn rating(rating: Rating) -> Option<oms::Level> {
    match rating {
        Rating::Low => Some(oms::Level::Low),
        Rating::Medium => Some(oms::Level::Medium),
        Rating::High => Some(oms::Level::High),
        Rating::None => None,
    }
}

fn rating_text(rating: Rating) -> Option<&'static str> {
    match rating {
        Rating::None => None,
        other => Some(other.as_str()),
    }
}

fn stored_level(text: Option<&str>) -> Option<oms::Level> {
    match text? {
        "low" => Some(oms::Level::Low),
        "medium" => Some(oms::Level::Medium),
        "high" => Some(oms::Level::High),
        _ => None,
    }
}
