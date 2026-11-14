//! The `data` of each kind of event, with exactly the keys the batch schema
//! names. A key that is only kept in the event row has a name that starts with `_`.

use serde::Deserialize;
use serde_json::Value;

use super::batch::read_data;

/// An enum of fixed texts, read from those texts. With `stored`, it is also
/// written back as them.
macro_rules! text_enum {
    ($name:ident { $($variant:ident = $text:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
        pub(super) enum $name {
            $(#[serde(rename = $text)] $variant),+
        }
    };
    (stored $name:ident { $($variant:ident = $text:literal),+ $(,)? }) => {
        text_enum!($name { $($variant = $text),+ });

        impl $name {
            pub(super) fn as_str(self) -> &'static str {
                match self {
                    $($name::$variant => $text),+
                }
            }
        }
    };
}

text_enum!(OnOff { On = "on", Off = "off" });
text_enum!(stored Origin { Composer = "composer", Bridge = "bridge", Sdk = "sdk", Master = "master" });
text_enum!(stored Level { Low = "low", Medium = "medium", High = "high" });
text_enum!(stored Rating { Low = "low", Medium = "medium", High = "high", None = "none" });
text_enum!(stored WorkType {
    Feature = "feature", Fix = "fix", Refactor = "refactor", Chore = "chore", Research = "research",
    Other = "other",
});
text_enum!(stored Size { Small = "small", Medium = "medium", Large = "large" });
text_enum!(stored LimitSource { Default = "default", Set = "set", Policy = "policy" });
text_enum!(CloseStatus {
    Delivered = "delivered", Blocked = "blocked", Failed = "failed", Cancelled = "cancelled",
    Lost = "lost",
});
text_enum!(stored DeliveryOutcome {
    Merged = "merged", PrOpen = "pr-open", KeptLocal = "kept-local", Blocked = "blocked",
});
text_enum!(stored YesNo { Yes = "yes", No = "no" });
text_enum!(stored MergeSource {
    Dialog = "dialog", PersonPrompt = "person-prompt", SdkPrompt = "sdk-prompt",
    Question = "question", Policy = "policy",
});
text_enum!(stored DecisionSource {
    Rule = "rule", DecisionAgent = "decision-agent", Override = "override", Plan = "plan",
});
text_enum!(stored SkillSource { Main = "main", Plugin = "plugin", User = "user", Worktree = "worktree" });
text_enum!(DoneCheck {
    Fetched = "fetched", Merged = "merged", BaseCurrent = "base_current",
    BranchesGone = "branches_gone", WorktreesGone = "worktrees_gone",
    OutputExists = "output_exists", ChecksOnBase = "checks_on_base",
});
text_enum!(CheckResult { Pass = "pass", Fail = "fail", NotSet = "not-set" });
text_enum!(WakeReason { Stuck = "stuck", Retry = "retry" });
text_enum!(stored ParentSource { Engine = "engine", Mod = "mod" });
text_enum!(stored Assembled { Yes = "yes", No = "no", Late = "late" });
text_enum!(stored Lint { Pass = "pass", PassedMarked = "passed-marked" });
text_enum!(EndReason {
    Answer = "answer", Aborted = "aborted", Refusal = "refusal", Error = "error", Lost = "lost",
});
text_enum!(stored OpenHow { Attached = "attached", RexSkill = "rex_skill", Read = "read" });
text_enum!(stored Category { Merge = "merge", Scope = "scope", Spend = "spend", Other = "other" });
text_enum!(stored AnswerSource { Dialog = "dialog", Pane = "pane", Command = "command" });
text_enum!(stored OpenTreeState { Running = "running", Paused = "paused", Blocked = "blocked" });
text_enum!(stored ClosedTreeState { Done = "done", Blocked = "blocked", Lost = "lost" });

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SessionStart {
    #[serde(rename = "claude_code")]
    pub(super) _claude_code: String,
    #[serde(rename = "plugin")]
    pub(super) _plugin: String,
    #[serde(rename = "storage")]
    pub(super) _storage: OnOff,
    pub(super) transcript_path: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Command {
    #[serde(rename = "name")]
    pub(super) _name: String,
    #[serde(rename = "args")]
    pub(super) _args: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Prompt {
    #[serde(rename = "text")]
    pub(super) _text: String,
    #[serde(rename = "origin")]
    pub(super) _origin: Origin,
    #[serde(rename = "command")]
    pub(super) _command: Option<Command>,
    #[serde(rename = "restatement")]
    pub(super) _restatement: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Owner {
    pub(super) session: String,
    pub(super) builder: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Prefs {
    pub(super) duration: Option<Level>,
    pub(super) budget: Option<Level>,
    pub(super) autonomy: Option<Level>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct JobOpen {
    pub(super) owner: Owner,
    pub(super) origin: Origin,
    pub(super) request: String,
    pub(super) restatement: Option<String>,
    pub(super) repo: String,
    pub(super) branch: String,
    pub(super) base: String,
    pub(super) worktree: Option<String>,
    #[serde(rename = "type")]
    pub(super) work_type: Option<WorkType>,
    pub(super) size: Option<Size>,
    pub(super) consequence: Option<Level>,
    pub(super) langs: Vec<String>,
    pub(super) prefs: Prefs,
    pub(super) limit_nano: u64,
    pub(super) limit_source: LimitSource,
    pub(super) tree: Option<String>,
    #[serde(rename = "depends_on")]
    pub(super) _depends_on: Vec<String>,
    #[serde(rename = "merge_after")]
    pub(super) _merge_after: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct OpenWhen {
    #[serde(rename = "skill")]
    pub(super) _skill: String,
    #[serde(rename = "file")]
    pub(super) _file: Option<String>,
    #[serde(rename = "when")]
    pub(super) _when: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Task {
    #[serde(rename = "id")]
    pub(super) _id: String,
    pub(super) role: String,
    pub(super) skills: Vec<String>,
    #[serde(rename = "open_when")]
    pub(super) _open_when: Vec<OpenWhen>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Role {
    pub(super) role: String,
    pub(super) intelligence: Rating,
    pub(super) reliability: Rating,
    pub(super) consequence: Rating,
    pub(super) verification: Rating,
    pub(super) grid_agent: String,
    #[serde(rename = "conflict")]
    pub(super) _conflict: Option<String>,
    pub(super) oms_version: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ManifestEntry {
    pub(super) skill: String,
    pub(super) file: String,
    pub(super) hash: String,
    pub(super) source: SkillSource,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct JobPlan {
    pub(super) orchestrator: String,
    pub(super) breakdown: String,
    pub(super) tasks: Vec<Task>,
    pub(super) roles: Vec<Role>,
    pub(super) manifest: Vec<ManifestEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct JobAmend {
    pub(super) tasks: Vec<Task>,
    pub(super) roles: Vec<Role>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Delivery {
    pub(super) repo: String,
    pub(super) branch: Option<String>,
    pub(super) pr: Option<String>,
    pub(super) outcome: DeliveryOutcome,
    pub(super) merge_commit: Option<String>,
    pub(super) note: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct JobClose {
    pub(super) status: CloseStatus,
    pub(super) outcome: String,
    pub(super) deliveries: Vec<Delivery>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MergeAnswer {
    pub(super) answer: YesNo,
    pub(super) source: MergeSource,
    pub(super) question: Option<String>,
    pub(super) policy_hash: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct JobLimit {
    pub(super) limit_nano: u64,
    pub(super) source: LimitSource,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Mark {
    pub(super) mark: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Rebind {
    pub(super) owner: Owner,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct JobDoneCheck {
    #[serde(rename = "check")]
    pub(super) _check: DoneCheck,
    #[serde(rename = "result")]
    pub(super) _result: CheckResult,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct JobWake {
    #[serde(rename = "reason")]
    pub(super) _reason: WakeReason,
    #[serde(rename = "count")]
    pub(super) _count: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Decision {
    pub(super) assignment_id: String,
    pub(super) subject: String,
    pub(super) item: String,
    pub(super) value: String,
    pub(super) source: DecisionSource,
    pub(super) reason: String,
    pub(super) table_version: Option<String>,
    pub(super) overlay_version: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RunStart {
    pub(super) agent_id: String,
    pub(super) parent: String,
    pub(super) parent_source: ParentSource,
    pub(super) depth: u64,
    pub(super) agent_type: String,
    pub(super) planned: Option<String>,
    pub(super) role: Option<String>,
    #[serde(rename = "task")]
    pub(super) _task: Option<String>,
    #[serde(rename = "tool_use_id")]
    pub(super) _tool_use_id: Option<String>,
    pub(super) brief_hash: Option<String>,
    pub(super) assembled: Assembled,
    pub(super) lint: Option<Lint>,
    pub(super) skills_attached: Vec<String>,
    pub(super) tree: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RunMark {
    pub(super) agent_id: String,
    pub(super) mark: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Tokens {
    #[serde(rename = "input")]
    pub(super) _input: u64,
    #[serde(rename = "output")]
    pub(super) _output: u64,
    #[serde(rename = "cache_read")]
    pub(super) _cache_read: u64,
    #[serde(rename = "cache_creation")]
    pub(super) _cache_creation: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RunEnd {
    pub(super) agent_id: String,
    #[serde(rename = "reason")]
    pub(super) _reason: EndReason,
    #[serde(rename = "tokens")]
    pub(super) _tokens: Tokens,
    #[serde(rename = "spent_nano")]
    pub(super) _spent_nano: u64,
    pub(super) final_message: String,
    pub(super) transcript_path: Option<String>,
    pub(super) output_file: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SkillOpen {
    pub(super) agent_id: String,
    pub(super) skill: String,
    pub(super) file: String,
    pub(super) hash: String,
    pub(super) how: OpenHow,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct QuestionAsk {
    pub(super) id: String,
    pub(super) tree: Option<String>,
    pub(super) question: String,
    pub(super) options: Vec<String>,
    pub(super) recommendation: String,
    pub(super) category: Category,
    pub(super) merge_for: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct QuestionAnswer {
    pub(super) id: String,
    pub(super) option: String,
    pub(super) source: AnswerSource,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TreeOpen {
    pub(super) id: String,
    pub(super) charter: String,
    pub(super) charter_hash: String,
    pub(super) policy_hash: String,
    pub(super) cap_nano: u64,
    pub(super) session: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TreeLink {
    pub(super) tree: String,
    pub(super) job: String,
    pub(super) agent_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TreeNote {
    #[serde(rename = "tree")]
    pub(super) _tree: String,
    #[serde(rename = "text")]
    pub(super) _text: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TreeState {
    pub(super) tree: String,
    pub(super) state: OpenTreeState,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TreeClose {
    pub(super) tree: String,
    pub(super) state: ClosedTreeState,
    pub(super) report_file: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MergeDone {
    #[serde(rename = "repo")]
    pub(super) _repo: String,
    #[serde(rename = "base")]
    pub(super) _base: String,
    #[serde(rename = "merge_commit")]
    pub(super) _merge_commit: String,
}

/// The `data` of one event, read into the type of its kind.
#[derive(Debug)]
pub(super) enum Data {
    SessionStart(SessionStart),
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
            "session.start" => Data::SessionStart(read_data(data)?),
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
            Data::SessionStart(_) => "session.start",
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
