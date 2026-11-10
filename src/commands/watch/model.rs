//! What `rex watch` knows between refreshes: the run tree of one job, and the
//! keys that move the selection. Pure values, so a test needs no terminal.

use std::collections::{HashMap, HashSet};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind};

use super::view::{TreeRow, duration_text, total_text};
use crate::capture::{Run, RunState};

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

fn tree_row(run: &Run, depth: usize) -> TreeRow {
    let dash = || "-".to_owned();
    TreeRow {
        depth,
        role: run.role.clone().unwrap_or_else(dash),
        agent: run
            .agent_type
            .clone()
            .or_else(|| run.model_ran.clone())
            .unwrap_or_else(dash),
        effort: run.effort_ran.clone().unwrap_or_else(dash),
        state: match run.state {
            RunState::Open => "open",
            RunState::Finished => "finished",
            RunState::Unconfirmed => "unconfirmed",
        },
        duration: duration_text(run.duration_ms),
        tokens: total_text(&run.tokens),
        tag_job: run.tag_job.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::{TokenCounts, TokenTotal};

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
            skills_opened: vec![],
            tokens: TokenTotal::Unknown,
        }
    }

    #[test]
    fn tree_nests_by_parent_and_marks_what_can_still_change() {
        let counts = TokenCounts {
            input: 61_000,
            output: 0,
            cache_read: 0,
            cache_creation: 0,
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

        let rows = build_tree(&[later_child, orphan, grandchild, lead, first_child]);
        let shape: Vec<_> = rows.iter().map(|row| row.depth).collect();
        assert_eq!(shape, [0, 1, 2, 1, 0]);
        assert_eq!(rows[0].tokens, "61.0k tokens~");
        assert_eq!(rows[0].duration, "41s");
        assert_eq!(rows[4].tag_job.as_deref(), Some("0g4cbf7k2q"));
    }

    #[test]
    fn tree_keeps_runs_that_only_name_each_other() {
        let rows = build_tree(&[
            run("x", Some("y"), "2026-11-10T10:00:00.000Z"),
            run("y", Some("x"), "2026-11-10T10:00:01.000Z"),
        ]);
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn keys_move_inside_the_rows() {
        let sizes = Sizes {
            jobs: 2,
            tree_rows: 3,
        };
        let (state, _) = on_key(State::default(), Key::Up, sizes);
        assert_eq!(state.job, 0);
        let (state, _) = on_key(state, Key::Down, sizes);
        let (state, _) = on_key(state, Key::Down, sizes);
        assert_eq!(state.job, 1);
        let (state, _) = on_key(state, Key::Enter, sizes);
        assert_eq!(state.screen, Screen::Tree);
        let (state, _) = on_key(state, Key::Down, sizes);
        assert_eq!(state.tree_row, 1);
        let (state, _) = on_key(state, Key::Enter, sizes);
        assert_eq!(state.screen, Screen::List);
        assert_eq!(on_key(state, Key::Quit, sizes).1, Action::Quit);
        let (state, _) = on_key(
            State::default(),
            Key::Enter,
            Sizes {
                jobs: 0,
                tree_rows: 0,
            },
        );
        assert_eq!(state.screen, Screen::List);
    }
}
