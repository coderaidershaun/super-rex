//! The events of a batch that are not about one job's row: a session's start,
//! sub-agent runs, skill files opened, questions to the person, and master trees.

use rusqlite::{OptionalExtension, params};

use super::batch::Event;
use super::data::{
    QuestionAnswer, QuestionAsk, RunEnd, RunMark, RunStart, SessionStart, SkillOpen, TreeClose,
    TreeLink, TreeOpen,
};
use super::job::{comma_list, fits};
use super::{Apply, IngestError};
use crate::capture::{derived_transcript, remember_main_transcript};
use crate::store::StoreError;
use crate::store::integer::signed;

impl Apply<'_> {
    /// Keeps the session's main transcript, which the cost of record is read from.
    pub(super) fn session_start(&self, data: &SessionStart) -> Result<(), IngestError> {
        if let Some(path) = &data.transcript_path {
            remember_main_transcript(self.tx, self.session, path).map_err(StoreError::from)?;
        }
        Ok(())
    }

    /// A run of a job gets its row; a run that is no job's, such as a master,
    /// is kept in the event row, and a master is named on its tree. A run that
    /// starts again, as a woken teammate does, keeps its first start.
    pub(super) fn run_start(&self, event: &Event, data: &RunStart) -> Result<(), IngestError> {
        let Some(job) = &event.job else {
            if let Some(tree) = &data.tree {
                self.tx
                    .execute(
                        "UPDATE tree SET master_agent_id = ?2 WHERE id = ?1 AND master_agent_id IS NULL",
                        params![tree, data.agent_id],
                    )
                    .map_err(StoreError::from)?;
            }
            return Ok(());
        };
        let parent = (data.parent != "main").then_some(data.parent.as_str());
        let transcript =
            derived_transcript(self.tx, self.session, &data.agent_id).map_err(StoreError::from)?;
        let written = self.tx.execute(
            "INSERT INTO sub_agent_run (job_id, session_id, agent_id, parent_agent_id, depth, role,
                                        agent_type, first_seen_at, started_at, last_started_at,
                                        parent_source, planned_agent, assembled, lint, brief_hash,
                                        skills_attached, transcript_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
             ON CONFLICT (session_id, agent_id) DO UPDATE SET last_started_at = excluded.last_started_at",
            params![
                job.as_str(),
                self.session,
                data.agent_id,
                parent,
                signed(data.depth),
                data.role,
                data.agent_type,
                event.at,
                data.parent_source.as_str(),
                data.planned,
                data.assembled.as_str(),
                data.lint.map(|lint| lint.as_str()),
                data.brief_hash,
                comma_list(&data.skills_attached),
                transcript,
            ],
        );
        fits(event, written, || format!("no job {job} in the store"))?;
        Ok(())
    }

    pub(super) fn run_mark(&self, data: &RunMark) -> Result<(), IngestError> {
        self.tx
            .execute(
                "UPDATE sub_agent_run SET marks = CASE
                     WHEN marks IS NULL OR marks = '' THEN ?3
                     WHEN instr(',' || marks || ',', ',' || ?3 || ',') > 0 THEN marks
                     ELSE marks || ',' || ?3 END
                 WHERE session_id = ?1 AND agent_id = ?2",
                params![self.session, data.agent_id, data.mark],
            )
            .map_err(StoreError::from)?;
        Ok(())
    }

    /// Records the stop. The tokens and the cost of record are read later from
    /// the transcript, which knows the cache writes that the event does not.
    pub(super) fn run_end(&self, event: &Event, data: &RunEnd) -> Result<(), IngestError> {
        self.tx
            .execute(
                "UPDATE sub_agent_run SET
                     stopped_at = ?3, last_stopped_at = ?3, final_message = ?4,
                     transcript_path = COALESCE(?5, transcript_path),
                     output_file = COALESCE(?6, output_file)
                 WHERE session_id = ?1 AND agent_id = ?2",
                params![
                    self.session,
                    data.agent_id,
                    event.at,
                    data.final_message,
                    data.transcript_path,
                    data.output_file
                ],
            )
            .map_err(StoreError::from)?;
        Ok(())
    }

    pub(super) fn skill_open(&self, event: &Event, data: &SkillOpen) -> Result<(), IngestError> {
        self.tx
            .execute(
                "INSERT INTO skill_open (session_id, agent_id, skill, file, hash, how, at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    self.session,
                    data.agent_id,
                    data.skill,
                    data.file,
                    data.hash,
                    data.how.as_str(),
                    event.at
                ],
            )
            .map_err(StoreError::from)?;
        Ok(())
    }

    /// The store keys questions by id alone. The same question asked again,
    /// as a resumed tree may, keeps its row; another question under a held id
    /// is refused rather than dropped, so it cannot take the other's answer.
    pub(super) fn question_ask(
        &self,
        event: &Event,
        data: &QuestionAsk,
    ) -> Result<(), IngestError> {
        let options = serde_json::to_string(&data.options).unwrap_or_else(|_| "[]".to_owned());
        let job = event.job.as_ref().map(|job| job.as_str());
        let inserted = self
            .tx
            .execute(
                "INSERT INTO question (id, tree_id, job_id, merge_for, question, options,
                                       recommendation, category, asked_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT (id) DO NOTHING",
                params![
                    data.id,
                    data.tree,
                    job,
                    data.merge_for,
                    data.question,
                    options,
                    data.recommendation,
                    data.category.as_str(),
                    event.at,
                ],
            )
            .map_err(StoreError::from)?;
        if inserted == 1 {
            return Ok(());
        }
        let is_same: bool = self
            .tx
            .query_row(
                "SELECT tree_id IS ?2 AND job_id IS ?3 AND merge_for IS ?4 AND question = ?5
                        AND options = ?6 AND recommendation = ?7 AND category = ?8
                 FROM question WHERE id = ?1",
                params![
                    data.id,
                    data.tree,
                    job,
                    data.merge_for,
                    data.question,
                    options,
                    data.recommendation,
                    data.category.as_str(),
                ],
                |row| row.get(0),
            )
            .map_err(StoreError::from)?;
        if is_same {
            Ok(())
        } else {
            Err(event.does_not_fit(format!(
                "question {} is in the store with other content",
                data.id
            )))
        }
    }

    /// A question is answered once: the same answer again changes nothing, and
    /// another answer is refused.
    pub(super) fn question_answer(
        &self,
        event: &Event,
        data: &QuestionAnswer,
    ) -> Result<(), IngestError> {
        let changed = self
            .tx
            .execute(
                "UPDATE question SET answer = ?2, answer_source = ?3, answered_at = ?4
                 WHERE id = ?1 AND answer IS NULL",
                params![data.id, data.option, data.source.as_str(), event.at],
            )
            .map_err(StoreError::from)?;
        if changed == 1 {
            return Ok(());
        }
        let kept: Option<Option<String>> = self
            .tx
            .query_row(
                "SELECT answer FROM question WHERE id = ?1",
                [&data.id],
                |row| row.get(0),
            )
            .optional()
            .map_err(StoreError::from)?;
        match kept {
            None => Err(event.does_not_fit(format!("no question {} in the store", data.id))),
            Some(Some(answer)) if answer == data.option => Ok(()),
            Some(answer) => Err(event.does_not_fit(format!(
                "question {} is answered {} already, got {}",
                data.id,
                answer.unwrap_or_default(),
                data.option
            ))),
        }
    }

    pub(super) fn tree_open(&self, event: &Event, data: &TreeOpen) -> Result<(), IngestError> {
        let inserted = self.tx.execute(
            "INSERT INTO tree (id, session_id, charter, charter_hash, policy_hash, state, cap_nano, started_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'running', ?6, ?7)",
            params![
                data.id,
                data.session,
                data.charter,
                data.charter_hash,
                data.policy_hash,
                signed(data.cap_nano),
                event.at
            ],
        );
        fits(event, inserted, || {
            format!("tree {} is in the store already", data.id)
        })?;
        Ok(())
    }

    /// A builder that the master started now owns its job.
    pub(super) fn tree_link(&self, event: &Event, data: &TreeLink) -> Result<(), IngestError> {
        let changed = fits(
            event,
            self.tx.execute(
                "UPDATE job SET tree_id = ?2, builder_agent_id = ?3, updated_at = ?4 WHERE id = ?1",
                params![data.job, data.tree, data.agent_id, event.at],
            ),
            || format!("builder {} has an open job already", data.agent_id),
        )?;
        if changed == 0 {
            return Err(event.does_not_fit(format!("no job {} in the store", data.job)));
        }
        Ok(())
    }

    pub(super) fn tree_state(
        &self,
        tree: &str,
        state: &str,
        closed: Option<(&str, Option<&str>)>,
    ) -> Result<(), IngestError> {
        let (closed_at, report_file) = closed.unzip();
        self.tx
            .execute(
                "UPDATE tree SET state = ?2,
                                 closed_at = COALESCE(?3, closed_at),
                                 report_file = COALESCE(?4, report_file)
                 WHERE id = ?1",
                params![tree, state, closed_at, report_file.flatten()],
            )
            .map_err(StoreError::from)?;
        Ok(())
    }

    pub(super) fn tree_close(&self, event: &Event, data: &TreeClose) -> Result<(), IngestError> {
        self.tree_state(
            &data.tree,
            data.state.as_str(),
            Some((event.at.as_str(), data.report_file.as_deref())),
        )
    }
}
