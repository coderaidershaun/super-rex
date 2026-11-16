//! The data of `rex watch` in one form that both the live snapshots and the store
//! fill: trees, jobs, agent runs and questions, merged by tree and job id.

use std::collections::HashMap;

use crate::capture::{Cost, TokenCounts};

/// A sum of nanodollars as a cost. The snapshot and a spend limit hold no
/// count of unpriced requests.
pub fn nano_cost(nanodollars: u64) -> Cost {
    Cost {
        nanodollars,
        unpriced_requests: 0,
    }
}

/// Where a tree or a job was read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Origin {
    /// The snapshot of a session that wrote it in the last 30 seconds.
    Live,
    Store,
    /// The last snapshot of a session that has ended or is idle.
    Ended,
}

/// Whose figure a spend is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SpendFrom {
    /// The figure of the row's own source: the cost of record of the store, or
    /// the live tally of a snapshot.
    #[default]
    Own,
    /// A row of the store that shows, in whole or in part, the live tally of a
    /// snapshot, because the store holds no cost of record for it.
    LiveTally,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub at: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeData {
    pub id: String,
    pub state: String,
    pub master: Option<String>,
    pub charter: String,
    pub spent: Option<Cost>,
    pub spent_from: SpendFrom,
    pub cap_nano: u64,
    pub started_at: String,
    pub closed_at: Option<String>,
    pub report_file: Option<String>,
    pub notes: Vec<Note>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delivery {
    pub repo: String,
    pub outcome: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobData {
    pub id: String,
    pub tree: Option<String>,
    /// The agent id of the builder, or `main` when the main session builds.
    pub builder: String,
    pub status: String,
    pub request: String,
    pub work_type: Option<String>,
    pub size: Option<String>,
    pub consequence: Option<String>,
    pub orchestrator: Option<String>,
    pub merge: Option<String>,
    pub merge_source: Option<String>,
    pub branch: Option<String>,
    pub base: Option<String>,
    pub worktree: Option<String>,
    pub limit_nano: Option<u64>,
    pub spent: Option<Cost>,
    pub spent_from: SpendFrom,
    pub tokens: Option<TokenCounts>,
    pub degraded: Vec<String>,
    pub deliveries: Vec<Delivery>,
    /// Each done check as `check result`, such as `merged pass`.
    pub done_checks: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunData {
    pub agent_id: String,
    pub parent: Option<String>,
    pub role: Option<String>,
    pub task: Option<String>,
    pub agent_type: Option<String>,
    /// The grid agent the plan named for this run.
    pub planned: Option<String>,
    pub model: Option<String>,
    pub effort: Option<String>,
    /// `open`, `done` or `unconfirmed`.
    pub state: String,
    pub started_at: Option<String>,
    pub duration_ms: Option<u64>,
    pub tokens: Option<TokenCounts>,
    pub cost: Option<Cost>,
    /// The result of the brief check.
    pub lint: Option<String>,
    pub skills_attached: Vec<String>,
    pub skills_opened: Vec<String>,
    pub output_file: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionData {
    pub id: String,
    pub question: String,
    pub options: Vec<String>,
    pub recommendation: String,
    pub asked_at: String,
    pub is_answered: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeBundle {
    pub tree: TreeData,
    pub master_run: Option<RunData>,
    /// The questions of the tree that belong to none of its jobs.
    pub questions: Vec<QuestionData>,
    pub origin: Origin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobBundle {
    pub job: JobData,
    /// Every run of the job, the builder's own run among them.
    pub runs: Vec<RunData>,
    pub questions: Vec<QuestionData>,
    pub origin: Origin,
}

/// What one source holds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Part {
    pub trees: Vec<TreeBundle>,
    pub jobs: Vec<JobBundle>,
}

/// Which sources gave the data in view.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Sources {
    pub has_live: bool,
    pub has_store: bool,
}

impl Sources {
    pub fn text(self) -> &'static str {
        match (self.has_live, self.has_store) {
            (true, true) => "live + store",
            (true, false) => "live",
            (false, true) => "store",
            (false, false) => "none",
        }
    }
}

/// Every tree and job in view: trees newest first, jobs newest first.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WatchData {
    pub trees: Vec<TreeBundle>,
    pub jobs: Vec<JobBundle>,
    pub sources: Sources,
}

impl WatchData {
    /// Merges the parts by id. A tree or a job comes from a live snapshot while
    /// one holds it, else from the store, else from the last snapshot of an
    /// ended session. `snapshots` come newest first, so the newest wins a tie.
    ///
    /// The view shows the most it knows: a job of the store with no cost of
    /// record shows the live tally of the newest snapshot that holds a spend for
    /// it, and a tree of the store spends what its jobs show plus its master's
    /// own spend, which only a snapshot holds; a figure with a tally in it is
    /// marked [`SpendFrom::LiveTally`].
    pub fn merge(snapshots: Vec<Part>, store: Option<Part>) -> WatchData {
        let has_store = store.is_some();
        let parts: Vec<Part> = snapshots.into_iter().chain(store).collect();
        let snapshots = &parts[..parts.len() - usize::from(has_store)];
        let mut trees = best_by_id(
            parts.iter().flat_map(|part| &part.trees),
            |bundle| &bundle.tree.id,
            |bundle| bundle.origin,
        );
        let mut jobs = best_by_id(
            parts.iter().flat_map(|part| &part.jobs),
            |bundle| &bundle.job.id,
            |bundle| bundle.origin,
        );
        for bundle in jobs
            .iter_mut()
            .filter(|bundle| bundle.origin == Origin::Store)
        {
            bundle.take_live_tally(snapshots);
        }
        for bundle in trees
            .iter_mut()
            .filter(|bundle| bundle.origin == Origin::Store)
        {
            bundle.add_up_spend(&jobs, snapshots);
        }
        trees.sort_by(|a, b| b.tree.started_at.cmp(&a.tree.started_at));
        jobs.sort_by(|a, b| b.job.created_at.cmp(&a.job.created_at));
        let has_live = trees
            .iter()
            .map(|bundle| bundle.origin)
            .chain(jobs.iter().map(|bundle| bundle.origin))
            .any(|origin| origin != Origin::Store);
        let has_store = has_store
            && (trees.iter().any(|bundle| bundle.origin == Origin::Store)
                || jobs.iter().any(|bundle| bundle.origin == Origin::Store)
                || !has_live);
        WatchData {
            trees,
            jobs,
            sources: Sources {
                has_live,
                has_store,
            },
        }
    }

    pub fn tree(&self, id: &str) -> Option<&TreeBundle> {
        self.trees.iter().find(|bundle| bundle.tree.id == id)
    }

    pub fn job(&self, id: &str) -> Option<&JobBundle> {
        self.jobs.iter().find(|bundle| bundle.job.id == id)
    }

    /// The job that holds the run `agent_id`, and the run.
    pub fn run(&self, agent_id: &str) -> Option<(&JobBundle, &RunData)> {
        self.jobs.iter().find_map(|bundle| {
            bundle
                .runs
                .iter()
                .find(|run| run.agent_id == agent_id)
                .map(|run| (bundle, run))
        })
    }

    /// The jobs of the tree `id`, oldest first.
    pub fn tree_jobs(&self, id: &str) -> Vec<&JobBundle> {
        let mut jobs: Vec<_> = self
            .jobs
            .iter()
            .filter(|bundle| bundle.job.tree.as_deref() == Some(id))
            .collect();
        jobs.sort_by(|a, b| a.job.created_at.cmp(&b.job.created_at));
        jobs
    }

    /// A job's tree, when that tree is in view.
    pub fn tree_of(&self, job: &JobData) -> Option<&TreeBundle> {
        job.tree.as_deref().and_then(|id| self.tree(id))
    }

    /// How many questions wait for an answer, in every tree and job.
    pub fn waiting_count(&self) -> usize {
        let tree_questions = self.trees.iter().flat_map(|bundle| &bundle.questions);
        let job_questions = self.jobs.iter().flat_map(|bundle| &bundle.questions);
        tree_questions
            .chain(job_questions)
            .filter(|question| !question.is_answered)
            .count()
    }
}

/// For each id, the first item of the best origin.
fn best_by_id<'a, T: Clone + 'a>(
    items: impl Iterator<Item = &'a T>,
    id: impl Fn(&T) -> &String,
    origin: impl Fn(&T) -> Origin,
) -> Vec<T> {
    let mut best: HashMap<String, &T> = HashMap::new();
    for item in items {
        let kept = best.entry(id(item).clone()).or_insert(item);
        if origin(item) < origin(kept) {
            *kept = item;
        }
    }
    best.into_values().cloned().collect()
}

impl TreeBundle {
    /// The spend of a tree of the store: what its jobs show, and its master's
    /// own spend from the newest snapshot that holds the master's run.
    fn add_up_spend(&mut self, jobs: &[JobBundle], snapshots: &[Part]) {
        if self.master_run.is_none() {
            self.master_run = snapshots
                .iter()
                .flat_map(|part| &part.trees)
                .filter(|bundle| bundle.tree.id == self.tree.id)
                .find_map(|bundle| bundle.master_run.clone());
        }
        let job_spends = jobs
            .iter()
            .filter(|bundle| bundle.job.tree.as_deref() == Some(&self.tree.id))
            .filter_map(|bundle| {
                let is_tally =
                    bundle.origin != Origin::Store || bundle.job.spent_from == SpendFrom::LiveTally;
                bundle.job.spent.map(|spent| (spent, is_tally))
            });
        let master_spend = self
            .master_run
            .as_ref()
            .and_then(|run| run.cost)
            .map(|spent| (spent, true));
        let mut spent: Option<Cost> = None;
        let mut has_tally = false;
        for (part, is_tally) in job_spends.chain(master_spend) {
            *spent.get_or_insert_with(Cost::default) += part;
            has_tally |= is_tally;
        }
        self.tree.spent = spent;
        self.tree.spent_from = if has_tally {
            SpendFrom::LiveTally
        } else {
            SpendFrom::Own
        };
    }
}

impl JobBundle {
    /// A job of the store with no cost of record takes the spend of the newest
    /// snapshot that holds one for it.
    fn take_live_tally(&mut self, snapshots: &[Part]) {
        if self.job.spent.is_some() {
            return;
        }
        let tally = snapshots
            .iter()
            .flat_map(|part| &part.jobs)
            .filter(|bundle| bundle.job.id == self.job.id)
            .find_map(|bundle| bundle.job.spent);
        if let Some(tally) = tally {
            self.job.spent = Some(tally);
            self.job.spent_from = SpendFrom::LiveTally;
        }
    }

    /// The builder's own run, when the builder is an agent and not the main session.
    pub fn builder_run(&self) -> Option<&RunData> {
        self.runs
            .iter()
            .find(|run| run.agent_id == self.job.builder)
    }

    /// The run's parent below the builder: `None` for a run the builder started.
    pub fn parent_of(&self, run: &RunData) -> Option<&RunData> {
        let parent = run.parent.as_deref()?;
        if parent == self.job.builder || parent == run.agent_id {
            return None;
        }
        self.runs.iter().find(|other| other.agent_id == parent)
    }

    /// How many levels the run is below the builder: 1 for a lead.
    pub fn depth_of(&self, run: &RunData) -> usize {
        let mut depth = 1;
        let mut at = run;
        while let Some(parent) = self.parent_of(at) {
            depth += 1;
            at = parent;
            // Runs that point at each other in a ring would never end.
            if depth > self.runs.len() {
                break;
            }
        }
        depth
    }

    /// The runs below `top` (every run below the builder when `None`), in
    /// tree order, each with its depth below `top`. Siblings come in start order.
    pub fn runs_below(&self, top: Option<&str>) -> Vec<(&RunData, usize)> {
        let mut rows = Vec::new();
        self.push_children(top, 1, &mut rows);
        rows
    }

    fn push_children<'a>(
        &'a self,
        parent: Option<&str>,
        depth: usize,
        rows: &mut Vec<(&'a RunData, usize)>,
    ) {
        let mut children: Vec<&RunData> = self
            .runs
            .iter()
            .filter(|run| run.agent_id != self.job.builder)
            .filter(|run| self.parent_of(run).map(|p| p.agent_id.as_str()) == parent)
            .filter(|run| !rows.iter().any(|(seen, _)| seen.agent_id == run.agent_id))
            .collect();
        children.sort_by_key(|run| {
            (
                run.started_at.is_none(),
                run.started_at.clone(),
                run.agent_id.clone(),
            )
        });
        for child in children {
            rows.push((child, depth));
            self.push_children(Some(&child.agent_id), depth + 1, rows);
        }
    }
}

#[cfg(test)]
#[path = "data_tests.rs"]
mod tests;
