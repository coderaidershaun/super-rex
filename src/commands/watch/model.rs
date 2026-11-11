//! What `rex watch` knows between refreshes: the run tree of one job, and the
//! keys that move the selection. Pure values, so a test needs no terminal.

use std::collections::{HashMap, HashSet};
use std::time::SystemTime;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind};

use super::view::{JobListRow, TokenTexts, TreeRow, ViewModel, duration_text};
use crate::capture::{JobRuns, Run, RunState, TokenTotal};
use crate::commands::jobs::{JobFilter, JobSummary, JobsError, load_summaries};
use crate::store::Store;

/// Which of the two screens is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Screen {
    #[default]
    List,
    Tree,
}

/// The selection: a job in the list, and a run in that job's tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct State {
    pub screen: Screen,
    pub job: usize,
    pub tree_row: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Enter,
    Quit,
    Other,
}

impl Key {
    /// Only a key press counts, so a terminal that also reports releases does not move twice.
    pub fn of(event: &KeyEvent) -> Key {
        if event.kind != KeyEventKind::Press {
            return Key::Other;
        }
        match event.code {
            KeyCode::Up => Key::Up,
            KeyCode::Down => Key::Down,
            KeyCode::Enter => Key::Enter,
            KeyCode::Char('q') => Key::Quit,
            _ => Key::Other,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Continue,
    Quit,
}

/// How many rows each screen has now, so the selection stays inside them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sizes {
    pub jobs: usize,
    pub tree_rows: usize,
}

impl State {
    fn clamped(self, sizes: Sizes) -> State {
        State {
            job: self.job.min(sizes.jobs.saturating_sub(1)),
            tree_row: self.tree_row.min(sizes.tree_rows.saturating_sub(1)),
            ..self
        }
    }
}

/// The state after one key. The selection stops at the first and last row.
pub fn on_key(state: State, key: Key, sizes: Sizes) -> (State, Action) {
    let state = state.clamped(sizes);
    let next = match (key, state.screen) {
        (Key::Quit, _) => return (state, Action::Quit),
        (Key::Up, Screen::List) => State {
            job: state.job.saturating_sub(1),
            ..state
        },
        (Key::Down, Screen::List) => State {
            job: state.job + 1,
            ..state
        },
        (Key::Up, Screen::Tree) => State {
            tree_row: state.tree_row.saturating_sub(1),
            ..state
        },
        (Key::Down, Screen::Tree) => State {
            tree_row: state.tree_row + 1,
            ..state
        },
        (Key::Enter, Screen::List) if sizes.jobs > 0 => State {
            screen: Screen::Tree,
            tree_row: 0,
            ..state
        },
        (Key::Enter, Screen::Tree) => State {
            screen: Screen::List,
            ..state
        },
        (Key::Enter, Screen::List) | (Key::Other, _) => state,
    };
    (next.clamped(sizes), Action::Continue)
}

/// What one refresh read: every job, as `rex jobs` lists it, with its runs.
#[derive(Debug)]
pub struct Snapshot {
    jobs: Vec<(JobSummary, JobRuns)>,
}

impl Snapshot {
    /// Reads every job through the function that `rex jobs` uses, so the
    /// screen and `rex jobs --json` show the same jobs, statuses, run counts
    /// and token totals.
    ///
    /// # Errors
    /// [`JobsError`] as [`load_summaries`] gives it.
    pub fn load(store: &mut Store, now: SystemTime) -> Result<Snapshot, JobsError> {
        Ok(Snapshot::of(load_summaries(
            store,
            &JobFilter::default(),
            now,
        )?))
    }

    pub fn of(jobs: Vec<(JobSummary, JobRuns)>) -> Snapshot {
        Snapshot { jobs }
    }

    /// Where the job `id` is in the list, so the selection can follow the job
    /// when a refresh puts a new job above it.
    pub fn position(&self, id: &str) -> Option<usize> {
        self.jobs.iter().position(|(summary, _)| summary.id == id)
    }

    /// The job list, and the run tree of the job at `selected`.
    pub fn view(&self, selected: usize) -> ViewModel {
        ViewModel {
            jobs: self
                .jobs
                .iter()
                .map(|(summary, _)| JobListRow::of(summary))
                .collect(),
            tree: self
                .jobs
                .get(selected)
                .map(|(summary, runs)| {
                    let has_session =
                        !runs.runs.is_empty() || runs.session_tokens != TokenTotal::Unknown;
                    let session = has_session.then(|| session_row(summary, runs));
                    session.into_iter().chain(build_tree(&runs.runs)).collect()
                })
                .unwrap_or_default(),
        }
    }
}

/// The runs of one job as tree rows: children under their parent, each level
/// in start order. A run whose parent is not a run of the job is a top-level row.
pub fn build_tree(runs: &[Run]) -> Vec<TreeRow> {
    let known: HashSet<&str> = runs.iter().map(|run| run.agent_id.as_str()).collect();
    let mut children: HashMap<&str, Vec<usize>> = HashMap::new();
    let mut roots = Vec::new();
    for (index, run) in runs.iter().enumerate() {
        match run.parent_agent_id.as_deref() {
            Some(parent) if parent != run.agent_id && known.contains(parent) => {
                children.entry(parent).or_default().push(index);
            }
            _ => roots.push(index),
        }
    }
    let by_start = |index: &usize| {
        let run = &runs[*index];
        (
            run.started_at.is_none(),
            run.started_at.clone(),
            run.agent_id.clone(),
        )
    };
    roots.sort_by_key(by_start);
    for siblings in children.values_mut() {
        siblings.sort_by_key(by_start);
    }

    // Runs that only point at each other have no top-level ancestor. They are
    // visited last, as top-level rows, so that no run is dropped.
    let mut visited = vec![false; runs.len()];
    let mut rows = Vec::with_capacity(runs.len());
    for seed in roots.into_iter().chain(0..runs.len()) {
        let mut stack = vec![(seed, 0_usize)];
        while let Some((index, depth)) = stack.pop() {
            if std::mem::replace(&mut visited[index], true) {
                continue;
            }
            rows.push(tree_row(&runs[index], depth));
            if let Some(kids) = children.get(runs[index].agent_id.as_str()) {
                stack.extend(kids.iter().rev().map(|kid| (*kid, depth + 1)));
            }
        }
    }
    rows
}

/// The main session as the first row, so the rows add up to the job's total.
fn session_row(summary: &JobSummary, runs: &JobRuns) -> TreeRow {
    let dash = || "-".to_owned();
    TreeRow {
        depth: 0,
        role: "session".to_owned(),
        model: summary.builder_model.clone().unwrap_or_else(dash),
        effort: summary.builder_effort.clone().unwrap_or_else(dash),
        state: "-",
        duration: dash(),
        tokens: TokenTexts::of(&runs.session_tokens),
        tag_job: None,
    }
}

fn tree_row(run: &Run, depth: usize) -> TreeRow {
    let dash = || "-".to_owned();
    TreeRow {
        depth,
        role: run.role.clone().unwrap_or_else(dash),
        model: run.model_ran.clone().unwrap_or_else(dash),
        effort: run.effort_ran.clone().unwrap_or_else(dash),
        state: match run.state {
            RunState::Open => "open",
            RunState::Finished => "finished",
            RunState::Unconfirmed => "unconfirmed",
        },
        duration: duration_text(run.duration_ms),
        tokens: TokenTexts::of(&run.tokens),
        tag_job: run.tag_job.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::{TokenCounts, TokenTotal};
    use crate::store::StorePaths;

    fn run(id: &str, parent: Option<&str>, started: &str) -> Run {
        Run {
            agent_id: id.into(),
            parent_agent_id: parent.map(Into::into),
            depth: None,
            role: Some("worker".into()),
            tag_job: None,
            agent_type: Some("model-low-effort-low".into()),
            model_ran: None,
            effort_ran: Some("low".into()),
            background: None,
            state: RunState::Finished,
            started_at: Some(started.into()),
            stopped_at: None,
            duration_ms: Some(41_000),
            tool_uses: None,
            lines_added: None,
            lines_removed: None,
            brief: None,
            final_message: None,
            skills_opened: None,
            tokens: TokenTotal::Unknown,
        }
    }

    fn seed(store: &Store, id: &str, created_at: &str, status: &str) {
        store
            .conn()
            .execute(
                "INSERT INTO job (id, request, asked_by, class_version, work_type, size, \
                 consequence, languages, merge_agreed, session_id, status, created_at, updated_at) \
                 VALUES (?1, 'A request.', 'human', 1, 'fix', 'small', 'low', 'rust', 0, ?1, ?2, \
                 ?3, ?3)",
                (id, status, created_at),
            )
            .unwrap();
    }

    #[test]
    fn watch_model_matches_jobs_listing_and_builds_tree() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        let paths = StorePaths::locate(dir.path()).unwrap();
        let mut store = Store::create(&paths).unwrap();
        seed(&store, "0g4aaaaaa1", "2026-11-10T09:00:00.000Z", "planned");
        seed(
            &store,
            "0g4bbbbbb2",
            "2026-11-10T10:00:00.000Z",
            "delivered",
        );

        // The watch data and the `rex jobs` data come from the same function.
        let now = SystemTime::now();
        let listed = load_summaries(&mut store, &JobFilter::default(), now).unwrap();
        let watched = Snapshot::load(&mut store, now).unwrap();
        assert_eq!(watched.jobs.len(), listed.len());
        for ((shown, runs), (listed, listed_runs)) in watched.jobs.iter().zip(&listed) {
            assert_eq!(shown, listed);
            assert_eq!(runs.open, listed_runs.open);
            assert_eq!(runs.finished, listed_runs.finished);
            assert_eq!(runs.tokens, listed_runs.tokens);
        }
        let view = watched.view(0);
        let ids: Vec<_> = view.jobs.iter().map(|row| row.id.as_str()).collect();
        assert_eq!(ids, ["0g4bbbbbb2", "0g4aaaaaa1"]);
        assert_eq!(view.jobs[1].status, "planned");

        // Hand-built runs: nesting, the token split, the provisional mark and the tag job.
        let counts = TokenCounts {
            input: 30,
            output: 13_483,
            cache_read: 762_619,
            cache_creation: 78_405,
        };
        let mut lead = run("lead", None, "2026-11-10T10:00:00.000Z");
        lead.tokens = TokenTotal::Provisional(counts);
        let mut orphan = run(
            "orphan",
            Some("started-before-the-job"),
            "2026-11-10T10:00:01.000Z",
        );
        orphan.tag_job = Some("0g4cbf7k2q".into());
        let later_child = run("b", Some("lead"), "2026-11-10T10:03:00.000Z");
        let first_child = run("a", Some("lead"), "2026-11-10T10:02:00.000Z");
        let grandchild = run("c", Some("a"), "2026-11-10T10:02:30.000Z");
        let mut hand_built = watched.jobs[0].1.clone();
        hand_built.runs = vec![later_child, orphan, grandchild, lead, first_child];
        // A session with no prompt token has no hit %.
        hand_built.session_tokens = TokenTotal::Final(TokenCounts::default());
        let with_runs = Snapshot::of(vec![(watched.jobs[0].0.clone(), hand_built)]);

        let rows = with_runs.view(0).tree;
        let shape: Vec<_> = rows.iter().map(|row| row.depth).collect();
        assert_eq!(shape, [0, 0, 1, 2, 1, 0], "the session row comes first");
        assert_eq!((rows[0].role.as_str(), rows[0].state), ("session", "-"));
        assert_eq!(rows[0].tokens.hit, "–");
        let lead_tokens = &rows[1].tokens;
        assert_eq!(
            [
                &lead_tokens.input,
                &lead_tokens.cache_write,
                &lead_tokens.cache_read,
                &lead_tokens.output,
                &lead_tokens.hit,
            ],
            ["30", "78.4k", "762.6k", "13.4k~", "90%"]
        );
        assert_eq!(rows[1].duration, "41s");
        assert_eq!(
            rows[2].tokens,
            TokenTexts::default(),
            "unknown tokens show nothing"
        );
        assert_eq!(rows[5].tag_job.as_deref(), Some("0g4cbf7k2q"));

        // Counts are cut, never rounded up, and a hit % of huge counts does not overflow.
        assert_eq!(crate::commands::token_text::short_count(999_999), "999.9k");
        let huge = TokenCounts {
            input: u64::MAX,
            output: 0,
            cache_read: u64::MAX,
            cache_creation: 0,
        };
        assert_eq!(huge.cache_hit_percent(), Some(50));
    }
}
