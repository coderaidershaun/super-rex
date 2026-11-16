//! The closed list of event kinds that `rex ingest` knows, each with the type
//! its `data` is read into.

use serde_json::Value;

use super::batch::read_data;
use super::data::{
    Decision, JobAmend, JobClose, JobDoneCheck, JobLimit, JobOpen, JobPlan, JobWake, Mark,
    MergeAnswer, MergeDone, Prompt, QuestionAnswer, QuestionAsk, Rebind, RunEnd, RunMark, RunStart,
    SessionStart, SessionTranscript, SkillOpen, TreeClose, TreeLink, TreeNote, TreeOpen, TreeState,
};

/// The `data` of one event, read into the type of its kind.
#[derive(Debug)]
pub(super) enum Data {
    SessionStart,
    SessionTranscript(SessionTranscript),
    Prompt,
    JobOpen(JobOpen),
    JobPlan(JobPlan),
    JobAmend(JobAmend),
    JobClose(JobClose),
    JobMergeAnswer(MergeAnswer),
    JobLimit(JobLimit),
    JobDegraded(Mark),
    JobRebind(Rebind),
    JobDoneCheck,
    JobWake,
    Decision(Decision),
    RunStart(RunStart),
    RunMark(RunMark),
    RunEnd(RunEnd),
    SkillOpen(SkillOpen),
    QuestionAsk(QuestionAsk),
    QuestionAnswer(QuestionAnswer),
    TreeOpen(TreeOpen),
    TreeLink(TreeLink),
    TreeNote,
    TreeState(TreeState),
    TreeClose(TreeClose),
    MergeDone,
}

impl Data {
    /// `data` read as the type of `kind`.
    ///
    /// # Errors
    /// The reason, for a kind not on the closed list or data that does not fit it.
    pub(super) fn read(kind: &str, data: &Value) -> Result<Data, String> {
        Ok(match kind {
            "session.start" => checked::<SessionStart>(data, Data::SessionStart)?,
            "session.transcript" => Data::SessionTranscript(read_data(data)?),
            "prompt" => checked::<Prompt>(data, Data::Prompt)?,
            "job.open" => Data::JobOpen(read_data(data)?),
            "job.plan" => Data::JobPlan(read_data(data)?),
            "job.amend" => Data::JobAmend(read_data(data)?),
            "job.close" => Data::JobClose(read_data(data)?),
            "job.merge_answer" => Data::JobMergeAnswer(read_data(data)?),
            "job.limit" => Data::JobLimit(read_data(data)?),
            "job.degraded" => Data::JobDegraded(read_data(data)?),
            "job.rebind" => Data::JobRebind(read_data(data)?),
            "job.done_check" => checked::<JobDoneCheck>(data, Data::JobDoneCheck)?,
            "job.wake" => checked::<JobWake>(data, Data::JobWake)?,
            "decision" => Data::Decision(read_data(data)?),
            "run.start" => Data::RunStart(read_data(data)?),
            "run.mark" => Data::RunMark(read_data(data)?),
            "run.end" => Data::RunEnd(read_data(data)?),
            "skill.open" => Data::SkillOpen(read_data(data)?),
            "question.ask" => {
                let ask: QuestionAsk = read_data(data)?;
                if !(2..=4).contains(&ask.options.len()) {
                    return Err(format!(
                        "a question has 2 to 4 options, got {}",
                        ask.options.len()
                    ));
                }
                Data::QuestionAsk(ask)
            }
            "question.answer" => Data::QuestionAnswer(read_data(data)?),
            "tree.open" => Data::TreeOpen(read_data(data)?),
            "tree.link" => Data::TreeLink(read_data(data)?),
            "tree.note" => checked::<TreeNote>(data, Data::TreeNote)?,
            "tree.state" => Data::TreeState(read_data(data)?),
            "tree.close" => Data::TreeClose(read_data(data)?),
            "merge.done" => checked::<MergeDone>(data, Data::MergeDone)?,
            other => return Err(format!("kind {other:?} is not one rex ingest knows")),
        })
    }

    pub(super) fn kind(&self) -> &'static str {
        match self {
            Data::SessionStart => "session.start",
            Data::SessionTranscript(_) => "session.transcript",
            Data::Prompt => "prompt",
            Data::JobOpen(_) => "job.open",
            Data::JobPlan(_) => "job.plan",
            Data::JobAmend(_) => "job.amend",
            Data::JobClose(_) => "job.close",
            Data::JobMergeAnswer(_) => "job.merge_answer",
            Data::JobLimit(_) => "job.limit",
            Data::JobDegraded(_) => "job.degraded",
            Data::JobRebind(_) => "job.rebind",
            Data::JobDoneCheck => "job.done_check",
            Data::JobWake => "job.wake",
            Data::Decision(_) => "decision",
            Data::RunStart(_) => "run.start",
            Data::RunMark(_) => "run.mark",
            Data::RunEnd(_) => "run.end",
            Data::SkillOpen(_) => "skill.open",
            Data::QuestionAsk(_) => "question.ask",
            Data::QuestionAnswer(_) => "question.answer",
            Data::TreeOpen(_) => "tree.open",
            Data::TreeLink(_) => "tree.link",
            Data::TreeNote => "tree.note",
            Data::TreeState(_) => "tree.state",
            Data::TreeClose(_) => "tree.close",
            Data::MergeDone => "merge.done",
        }
    }

    /// True for a kind whose event must name its job.
    pub(super) fn needs_job(&self) -> bool {
        matches!(
            self,
            Data::JobOpen(_)
                | Data::JobPlan(_)
                | Data::JobAmend(_)
                | Data::JobClose(_)
                | Data::JobMergeAnswer(_)
                | Data::JobLimit(_)
                | Data::JobDegraded(_)
                | Data::JobRebind(_)
                | Data::JobDoneCheck
                | Data::JobWake
                | Data::Decision(_)
        )
    }
}

/// `data` checked against `T`, the type of its kind, for a kind that is kept
/// in its event row only.
fn checked<T: serde::de::DeserializeOwned>(data: &Value, kept: Data) -> Result<Data, String> {
    read_data::<T>(data)?;
    Ok(kept)
}
