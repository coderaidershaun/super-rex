//! Reads the live snapshots that each Claude Code session with the plugin writes
//! to `.rex/live/<session>.json`, into the data of `rex watch`.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::data::{
    Delivery, JobBundle, JobData, Note, Origin, Part, QuestionData, RunData, SpendFrom, TreeBundle,
    TreeData, nano_cost,
};
pub use super::live_form::Snapshot;
use super::live_form::{LiveJob, LiveQuestion, LiveRun, LiveTokens, LiveTree, VERSION};
use crate::capture::TokenCounts;

/// A session whose snapshot is older than this has ended or is idle: the mod
/// rewrites the file at least every 10 seconds while a job or a tree is open.
pub const LIVE_MS: u64 = 30_000;

/// A snapshot older than this is removed when `rex watch` starts.
pub const KEEP_MS: u64 = 7 * 24 * 3_600_000;

impl Snapshot {
    /// The snapshot in `text`; `None` when it is not a whole snapshot of this
    /// version, such as a file the mod is still writing or a file of a newer mod.
    pub fn parse(text: &str) -> Option<Snapshot> {
        serde_json::from_str(text)
            .ok()
            .filter(|snapshot: &Snapshot| snapshot.version == VERSION)
    }

    fn read(path: &Path) -> Option<Snapshot> {
        Snapshot::parse(&fs::read_to_string(path).ok()?)
    }

    /// The snapshot's trees and jobs, marked live while `written_at` is under
    /// 30 seconds old at `now_ms`.
    pub fn part(&self, now_ms: u64) -> Part {
        let written = epoch_ms(&self.written_at).unwrap_or(0);
        let origin = if now_ms.saturating_sub(written) <= LIVE_MS {
            Origin::Live
        } else {
            Origin::Ended
        };
        let run_data = |run: &LiveRun| run_data(run, now_ms);
        let questions_where = |keep: &dyn Fn(&LiveQuestion) -> bool| -> Vec<QuestionData> {
            self.questions
                .iter()
                .filter(|q| keep(q))
                .map(question_data)
                .collect()
        };
        let trees = self
            .trees
            .iter()
            .map(|tree| TreeBundle {
                tree: tree_data(tree),
                master_run: self
                    .runs
                    .iter()
                    .find(|run| Some(&run.agent_id) == tree.master.as_ref())
                    .map(run_data),
                questions: questions_where(&|q| {
                    q.tree.as_deref() == Some(&tree.id) && q.job.is_none()
                }),
                origin,
            })
            .collect();
        let jobs = self
            .jobs
            .iter()
            .map(|job| JobBundle {
                job: job_data(job),
                runs: self
                    .runs
                    .iter()
                    .filter(|run| run.job.as_deref() == Some(&job.id))
                    .map(run_data)
                    .collect(),
                questions: questions_where(&|q| q.job.as_deref() == Some(&job.id)),
                origin,
            })
            .collect();
        Part { trees, jobs }
    }
}

fn tokens(tokens: LiveTokens) -> TokenCounts {
    TokenCounts {
        input: tokens.input,
        output: tokens.output,
        cache_read: tokens.cache_read,
        cache_creation: tokens.cache_creation,
    }
}

fn tree_data(tree: &LiveTree) -> TreeData {
    TreeData {
        id: tree.id.clone(),
        state: tree.state.clone(),
        master: tree.master.clone(),
        charter: tree.charter.clone(),
        spent: Some(nano_cost(tree.spent_nano)),
        spent_from: SpendFrom::Own,
        cap_nano: tree.cap_nano,
        started_at: tree.started_at.clone(),
        closed_at: tree.closed_at.clone(),
        report_file: tree.report_file.clone(),
        notes: tree
            .notes
            .iter()
            .map(|note| Note {
                at: note.at.clone(),
                text: note.text.clone(),
            })
            .collect(),
    }
}

fn job_data(job: &LiveJob) -> JobData {
    JobData {
        id: job.id.clone(),
        tree: job.tree.clone(),
        builder: job.builder.clone(),
        status: job.status.clone(),
        request: job.request.clone(),
        work_type: job.r#type.clone(),
        size: job.size.clone(),
        consequence: job.consequence.clone(),
        orchestrator: job.orchestrator.clone(),
        merge: job.merge.clone(),
        merge_source: job.merge_source.clone(),
        branch: job.branch.clone(),
        base: job.base.clone(),
        worktree: job.worktree.clone(),
        limit_nano: job.limit_nano,
        spent: Some(nano_cost(job.spent_nano)),
        spent_from: SpendFrom::Own,
        tokens: Some(tokens(job.tokens)),
        degraded: job.degraded.clone(),
        deliveries: job
            .deliveries
            .iter()
            .map(|delivery| Delivery {
                repo: delivery.repo.clone(),
                outcome: delivery.outcome.clone(),
                note: delivery.note.clone(),
            })
            .collect(),
        done_checks: job
            .done_checks
            .iter()
            .map(|check| format!("{} {}", check.check, check.result))
            .collect(),
        created_at: job.created_at.clone(),
    }
}

/// An open run's time runs up to `now_ms`.
fn run_data(run: &LiveRun, now_ms: u64) -> RunData {
    let started = epoch_ms(&run.started_at);
    let ended = run.ended_at.as_deref().map_or(Some(now_ms), epoch_ms);
    RunData {
        agent_id: run.agent_id.clone(),
        parent: run.parent.clone(),
        role: run.role.clone(),
        task: run.task.clone(),
        agent_type: run.agent_type.clone(),
        planned: run.planned.clone(),
        model: run.model.clone(),
        effort: run.effort.clone(),
        state: run.state.clone(),
        started_at: Some(run.started_at.clone()),
        duration_ms: started
            .zip(ended)
            .map(|(started, ended)| ended.saturating_sub(started)),
        tokens: Some(tokens(run.tokens)),
        cost: Some(nano_cost(run.spent_nano)),
        lint: run.lint.clone(),
        skills_attached: run.skills_attached.clone(),
        skills_opened: run.skills_opened.clone(),
        output_file: run.output_file.clone(),
    }
}

fn question_data(question: &LiveQuestion) -> QuestionData {
    QuestionData {
        id: question.id.clone(),
        question: question.question.clone(),
        options: question.options.clone(),
        recommendation: question.recommendation.clone(),
        asked_at: question.asked_at.clone(),
        is_answered: question.answer.is_some(),
    }
}

/// Keeps the last good read of each snapshot file across refreshes.
#[derive(Debug, Default)]
pub struct LiveReader {
    last_good: HashMap<PathBuf, Snapshot>,
}

impl LiveReader {
    /// Every snapshot in `dir`, newest first. A file that does not parse, such
    /// as one the mod is half way through writing, gives its last good read.
    ///
    /// # Errors
    /// The error of listing `dir`, unless it does not exist.
    pub fn read(&mut self, dir: &Path) -> io::Result<Vec<Snapshot>> {
        let mut seen = HashMap::new();
        for path in snapshot_files(dir)? {
            let fresh = Snapshot::read(&path);
            if let Some(snapshot) = fresh.or_else(|| self.last_good.remove(&path)) {
                seen.insert(path, snapshot);
            }
        }
        self.last_good = seen;
        let mut snapshots: Vec<Snapshot> = self.last_good.values().cloned().collect();
        snapshots.sort_by(|a, b| b.written_at.cmp(&a.written_at));
        Ok(snapshots)
    }
}

/// Removes each snapshot in `dir` whose `written_at` is more than 7 days before
/// `now_ms`. Any other file is left alone: one that does not parse may be half
/// written, and one of another version or form is not this binary's to remove.
///
/// # Errors
/// The error of listing `dir` or of removing an old file.
pub fn remove_old(dir: &Path, now_ms: u64) -> io::Result<()> {
    for path in snapshot_files(dir)? {
        let written = Snapshot::read(&path).and_then(|snapshot| epoch_ms(&snapshot.written_at));
        if written.is_some_and(|written| now_ms.saturating_sub(written) > KEEP_MS) {
            fs::remove_file(&path)?;
        }
    }
    Ok(())
}

fn snapshot_files(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut files = Vec::new();
    for entry in entries {
        let path = entry?.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            files.push(path);
        }
    }
    Ok(files)
}

/// `2026-11-15T20:41:10.000Z` as milliseconds since 1970, `None` for any other form.
pub fn epoch_ms(text: &str) -> Option<u64> {
    let number = |range: std::ops::Range<usize>| text.get(range)?.parse::<u64>().ok();
    let bytes = text.as_bytes();
    let separators = [
        (4, b'-'),
        (7, b'-'),
        (10, b'T'),
        (13, b':'),
        (16, b':'),
        (19, b'.'),
        (23, b'Z'),
    ];
    if text.len() != 24 || separators.iter().any(|(at, byte)| bytes[*at] != *byte) {
        return None;
    }
    let (year, month, day) = (number(0..4)?, number(5..7)?, number(8..10)?);
    let (hour, minute, second, milli) = (
        number(11..13)?,
        number(14..16)?,
        number(17..19)?,
        number(20..23)?,
    );
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || year < 1970 {
        return None;
    }
    // Days since 1970 by the civil-from-days method of Howard Hinnant.
    let (year, month) = if month <= 2 {
        (year - 1, month + 9)
    } else {
        (year, month - 3)
    };
    let era = year / 400;
    let year_of_era = year - era * 400;
    let day_of_year = (153 * month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = (era * 146_097 + day_of_era).checked_sub(719_468)?;
    Some((((days * 24 + hour) * 60 + minute) * 60 + second) * 1_000 + milli)
}

#[cfg(test)]
#[path = "live_tests.rs"]
mod tests;
