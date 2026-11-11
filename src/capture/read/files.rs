//! What one read learns from the files of each run: the size and age of its
//! transcript, its metadata file, and a parse of the transcript when the token rule
//! asks for one. From these come the run's state, the columns a later source fills,
//! and the run's token reading.

use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use super::changes::Changes;
use super::state::{self, Activity};
use super::{Read, millis};
use crate::capture::recovery::{Metadata, metadata_name, read_metadata};
use crate::capture::rows::{Fill, Reading, RunReading, StoredRun};
use crate::capture::runs::RunState;
use crate::capture::tags::Tags;
use crate::capture::tokens::{RunActivity, Step, TotalCheck};
use crate::capture::transcripts::{TranscriptFacts, read_transcript};

/// How much a read must know of each run's files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Need {
    /// Every total is final: only what the state needs.
    State,
    /// Also the metadata file, for the columns it is a source of.
    LaterSources,
}

/// What this read saw of one run's files.
pub(super) struct RunFiles {
    transcript: Option<PathBuf>,
    /// Size of the transcript now; `None` when there is no file.
    bytes: Option<u64>,
    modified_ms: Option<u64>,
    metadata: Option<Metadata>,
}

/// Where each run stands at one read, by agent ID.
pub(super) struct Standing {
    pub(super) states: HashMap<String, RunState>,
    /// The newest activity of each run and of the runs below it, in milliseconds
    /// since the epoch. A run with no known activity is left out.
    pub(super) last_active_ms: HashMap<String, u64>,
}

fn activities<'a>(runs: &'a [StoredRun], files: &[RunFiles]) -> Vec<Activity<'a>> {
    runs.iter()
        .zip(files)
        .map(|(run, files)| Activity {
            agent_id: &run.agent_id,
            parent_agent_id: run.parent_agent_id.as_deref(),
            is_finished: run.stopped_at.is_some() || files.says_finished(),
            last_active_ms: run.started_ms.max(files.modified_ms),
        })
        .collect()
}

/// One run as this read sees it.
struct Seen<'a> {
    run: &'a StoredRun,
    files: &'a RunFiles,
    state: RunState,
}

impl Read<'_> {
    pub(super) fn observe(&mut self, runs: &[StoredRun], need: Need) -> Vec<RunFiles> {
        runs.iter()
            .map(|run| {
                let transcript = run.transcript_path.as_deref().map(PathBuf::from);
                let (bytes, modified_ms) = transcript
                    .as_deref()
                    .map_or((None, None), |path| self.size_and_time(path));
                let metadata = match &transcript {
                    Some(path) if need == Need::LaterSources && run.needs_metadata() => {
                        let path = path.with_file_name(metadata_name(&run.agent_id));
                        read_metadata(&path).unwrap_or_else(|_| {
                            self.has_unreadable_file = true;
                            None
                        })
                    }
                    _ => None,
                };
                RunFiles {
                    transcript,
                    bytes,
                    modified_ms,
                    metadata,
                }
            })
            .collect()
    }

    /// The size and modification time of a file, both `None` when it is absent.
    pub(super) fn size_and_time(&mut self, path: &Path) -> (Option<u64>, Option<u64>) {
        match fs::metadata(path) {
            Ok(metadata) => {
                let modified = metadata.modified().ok().map(millis);
                (Some(metadata.len()), modified)
            }
            Err(error) if error.kind() == ErrorKind::NotFound => (None, None),
            Err(_) => {
                self.has_unreadable_file = true;
                (None, None)
            }
        }
    }

    pub(super) fn standing(&self, runs: &[StoredRun], files: &[RunFiles]) -> Standing {
        let activities = activities(runs, files);
        let last_active_ms = state::last_activities(&activities);
        let states = state::states(&activities, &last_active_ms, self.now_ms);
        Standing {
            states: runs
                .iter()
                .map(|run| run.agent_id.clone())
                .zip(states)
                .collect(),
            last_active_ms,
        }
    }

    /// For each run whose total is not final: the token step, and the columns its
    /// metadata file and transcript fill.
    pub(super) fn run_changes(
        &mut self,
        runs: &[StoredRun],
        files: &[RunFiles],
        states: &HashMap<String, RunState>,
    ) -> Changes {
        let mut changes = Changes::default();
        let derived_depths = derived_depths(runs, files);
        for ((run, files), derived_depth) in runs.iter().zip(files).zip(derived_depths) {
            if run.total.is_final {
                continue;
            }
            let seen = Seen {
                run,
                files,
                state: states.get(&run.agent_id).copied().unwrap_or(RunState::Open),
            };
            let mut fill = seen.metadata_fill(derived_depth);
            let step = self.token_step(&seen);
            let is_parse_due = step == Step::ReadAgain || seen.needs_stop_time();
            // A final total is never read again, so a stop time that only the
            // transcript gives must be taken first.
            if step == Step::Final && !seen.needs_stop_time() {
                let judged = run.total.judged();
                changes
                    .settled
                    .extend(judged.map(|judged| (run.id, judged)));
            } else if is_parse_due
                && let Some(path) = files.transcript.as_deref()
                && let Some(facts) = self.parse(path)
            {
                self.transcript_fill(&mut fill, &seen, &facts);
                changes.readings.push((run.id, self.reading_of(&facts)));
            }
            if !fill.is_empty() {
                changes.fills.push((run.id, fill));
            }
        }
        changes
    }

    fn token_step(&self, seen: &Seen<'_>) -> Step {
        TotalCheck {
            stored: seen.run.total.reading(),
            transcript_bytes: seen.files.bytes,
            closed_at: self.closed_at(),
            run: match seen.state {
                RunState::Open => RunActivity::Open,
                RunState::Finished | RunState::Unconfirmed => RunActivity::NotOpen,
            },
            now: self.now,
        }
        .next_step()
    }

    /// A transcript that cannot be read counts as absent for this read.
    fn parse(&mut self, path: &Path) -> Option<TranscriptFacts> {
        read_transcript(path).unwrap_or_else(|_| {
            self.has_unreadable_file = true;
            None
        })
    }

    fn reading_of(&self, facts: &TranscriptFacts) -> RunReading {
        RunReading {
            reading: Reading {
                tokens: facts.tokens,
                cost: facts.cost,
                bytes: facts.bytes,
                damaged_lines: facts.damaged_lines,
                read_ms: self.now_ms,
            },
            skills_opened: facts.skills_opened.join(","),
            tool_uses: facts.tool_uses,
            lines_added: facts.lines_added,
            lines_removed: facts.lines_removed,
        }
    }

    /// The transcript is a later source of these columns: each is filled only while
    /// it is NULL, the tags only while the run has no brief from a call, and the
    /// final message only once the run is finished.
    fn transcript_fill(&self, fill: &mut Fill, seen: &Seen<'_>, facts: &TranscriptFacts) {
        let run = seen.run;
        if seen.needs_stop_time() {
            fill.stopped_at = facts.last_at.clone();
        }
        if run.brief.is_none() {
            fill.brief = facts.first_user_text.clone();
            let tags = facts
                .first_user_text
                .as_deref()
                .map(Tags::of_brief)
                .unwrap_or_default();
            fill.role = tags.role.filter(|_| run.role.is_none());
            fill.tag_job = tags
                .job
                .filter(|tag| run.tag_job.is_none() && tag != self.job.as_str());
        }
        fill.model_ran = facts.model.clone().filter(|_| run.model_ran.is_none());
        fill.effort_ran = facts.effort.clone().filter(|_| run.effort_ran.is_none());
        if run.final_message.is_none() && seen.state == RunState::Finished {
            fill.final_message = facts.last_assistant_text.clone();
        }
    }
}

impl RunFiles {
    fn says_finished(&self) -> bool {
        self.metadata.as_ref().is_some_and(|meta| meta.is_finished)
    }
}

impl Seen<'_> {
    /// A run whose stop line and completed call were lost has its stop time only
    /// from its transcript's last record, once its metadata file says it finished.
    fn needs_stop_time(&self) -> bool {
        self.run.stopped_at.is_none() && self.files.says_finished()
    }

    /// The metadata file is the first source of `depth`, and a later source of the
    /// parent, the agent type and whether the run is in the background.
    fn metadata_fill(&self, derived_depth: Option<u32>) -> Fill {
        let run = self.run;
        let Some(meta) = &self.files.metadata else {
            return Fill {
                derived_depth,
                ..Fill::default()
            };
        };
        Fill {
            spawn_depth: meta.spawn_depth.filter(|depth| run.depth != Some(*depth)),
            derived_depth: derived_depth.filter(|_| meta.spawn_depth.is_none()),
            parent_agent_id: meta
                .parent_agent_id
                .clone()
                .filter(|_| run.parent_agent_id.is_none()),
            agent_type: meta.agent_type.clone().filter(|_| run.agent_type.is_none()),
            background: meta.background.filter(|_| run.background.is_none()),
            ..Fill::default()
        }
    }
}

impl StoredRun {
    /// The metadata file is a source of these columns, and the only sign that a run
    /// whose stop line was lost has finished.
    fn needs_metadata(&self) -> bool {
        self.depth.is_none()
            || self.agent_type.is_none()
            || self.background.is_none()
            || self.stopped_at.is_none()
    }
}

/// For a run with no depth from its metadata file: one more than its parent's, when
/// the parent is a run of the job whose depth is known, and 1 for a run that the main
/// session started.
fn derived_depths(runs: &[StoredRun], files: &[RunFiles]) -> Vec<Option<u32>> {
    let mut depths: HashMap<&str, u32> = runs
        .iter()
        .zip(files)
        .filter_map(|(run, files)| {
            let from_metadata = files.metadata.as_ref().and_then(|meta| meta.spawn_depth);
            let depth = run.depth.or(from_metadata)?;
            Some((run.agent_id.as_str(), depth))
        })
        .collect();
    let mut derived = vec![None; runs.len()];
    // Each pass gives a depth to the runs one level further down the tree.
    for _ in 0..runs.len() {
        let mut is_changed = false;
        for (run, slot) in runs.iter().zip(derived.iter_mut()) {
            if depths.contains_key(run.agent_id.as_str()) {
                continue;
            }
            let parent_depth = match run.parent_agent_id.as_deref() {
                Some(parent) => depths.get(parent).copied(),
                // A run with no parent was started by the main session only if the call
                // that started it was seen, and that call is what sets `background`.
                None if run.background.is_some() => Some(0),
                None => None,
            };
            if let Some(depth) = parent_depth.and_then(|depth| depth.checked_add(1)) {
                depths.insert(&run.agent_id, depth);
                *slot = Some(depth);
                is_changed = true;
            }
        }
        if !is_changed {
            break;
        }
    }
    derived
}
