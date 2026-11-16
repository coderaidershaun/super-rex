//! Where the person is in `rex watch`: the node in view, the selected row, the
//! keys that move them, and the levels of the run. Pure values, so a test needs no terminal.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::data::{JobBundle, WatchData};

/// A thing a screen shows: a master tree, a job, or one agent run.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Node {
    Tree(String),
    Job(String),
    Run(String),
}

/// How far down a run a node sits, named the same way in every view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Master,
    Builder,
    Lead,
    Worker,
    Helper,
}

impl Level {
    pub fn name(self) -> &'static str {
        match self {
            Level::Master => "master",
            Level::Builder => "builder",
            Level::Lead => "lead",
            Level::Worker => "worker",
            Level::Helper => "helper",
        }
    }

    /// The level of a run that is `depth` levels below its builder.
    fn below_builder(depth: usize) -> Level {
        match depth {
            0 => Level::Builder,
            1 => Level::Lead,
            2 => Level::Worker,
            _ => Level::Helper,
        }
    }
}

/// One row of a screen, `indent` levels below the first rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenRow {
    pub node: Node,
    pub indent: usize,
}

impl WatchData {
    /// The rows of the screen of `at`, the home screen when `None`.
    pub fn rows(&self, at: Option<&Node>) -> Vec<ScreenRow> {
        let row = |node, indent| ScreenRow { node, indent };
        match at {
            None => {
                let trees = self
                    .trees
                    .iter()
                    .map(|bundle| (&bundle.tree.started_at, Node::Tree(bundle.tree.id.clone())));
                let loose_jobs = self
                    .jobs
                    .iter()
                    .filter(|bundle| self.tree_of(&bundle.job).is_none())
                    .map(|bundle| (&bundle.job.created_at, Node::Job(bundle.job.id.clone())));
                let mut items: Vec<_> = trees.chain(loose_jobs).collect();
                items.sort_by(|a, b| b.0.cmp(a.0));
                items.into_iter().map(|(_, node)| row(node, 0)).collect()
            }
            Some(Node::Tree(id)) => self
                .tree_jobs(id)
                .into_iter()
                .map(|bundle| row(Node::Job(bundle.job.id.clone()), 0))
                .collect(),
            Some(Node::Job(id)) => self
                .job(id)
                .map_or_else(Vec::new, |bundle| run_rows(bundle, None)),
            Some(Node::Run(agent)) => self
                .run(agent)
                .map_or_else(Vec::new, |(bundle, _)| run_rows(bundle, Some(agent))),
        }
    }

    pub fn contains(&self, node: &Node) -> bool {
        match node {
            Node::Tree(id) => self.tree(id).is_some(),
            Node::Job(id) => self.job(id).is_some(),
            Node::Run(agent) => self
                .run(agent)
                .is_some_and(|(bundle, run)| run.agent_id != bundle.job.builder),
        }
    }

    pub fn parent(&self, node: &Node) -> Option<Node> {
        match node {
            Node::Tree(_) => None,
            Node::Job(id) => self
                .job(id)
                .and_then(|bundle| self.tree_of(&bundle.job))
                .map(|tree| Node::Tree(tree.tree.id.clone())),
            Node::Run(agent) => self.run(agent).map(|(bundle, run)| {
                bundle.parent_of(run).map_or_else(
                    || Node::Job(bundle.job.id.clone()),
                    |parent| Node::Run(parent.agent_id.clone()),
                )
            }),
        }
    }

    /// The path from home to `node`, `node` last.
    pub fn trail(&self, node: &Node) -> Vec<Node> {
        let mut trail = vec![node.clone()];
        while let Some(parent) = self.parent(trail.last().unwrap_or(node)) {
            // A ring of parents would never end.
            if trail.contains(&parent) {
                break;
            }
            trail.push(parent);
        }
        trail.reverse();
        trail
    }

    pub fn level(&self, node: &Node) -> Level {
        match node {
            Node::Tree(_) => Level::Master,
            Node::Job(_) => Level::Builder,
            Node::Run(agent) => self.run(agent).map_or(Level::Helper, |(bundle, run)| {
                Level::below_builder(bundle.depth_of(run))
            }),
        }
    }

    /// The levels of the run that holds `at`, master first when it is in a tree.
    /// Helper is named only when the run goes that deep.
    pub fn rail(&self, at: &Node) -> Vec<Level> {
        let trail = self.trail(at);
        let has_tree = matches!(trail.first(), Some(Node::Tree(_)));
        let jobs: Vec<&JobBundle> = match trail.first() {
            Some(Node::Tree(id)) => self.tree_jobs(id),
            _ => trail
                .iter()
                .find_map(|node| match node {
                    Node::Job(id) => self.job(id),
                    _ => None,
                })
                .into_iter()
                .collect(),
        };
        let has_helper = jobs.iter().any(|bundle| {
            bundle
                .runs_below(None)
                .iter()
                .any(|(_, depth)| Level::below_builder(*depth) == Level::Helper)
        });
        let mut levels = Vec::new();
        if has_tree {
            levels.push(Level::Master);
        }
        levels.extend([Level::Builder, Level::Lead, Level::Worker]);
        if has_helper || self.level(at) == Level::Helper {
            levels.push(Level::Helper);
        }
        levels
    }
}

fn run_rows(bundle: &JobBundle, top: Option<&str>) -> Vec<ScreenRow> {
    bundle
        .runs_below(top)
        .into_iter()
        .map(|(run, depth)| ScreenRow {
            node: Node::Run(run.agent_id.clone()),
            indent: depth - 1,
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    In,
    Out,
    Home,
    Help,
    Quit,
    Other,
}

impl Key {
    /// Only a key press counts, so a terminal that also reports releases does not move twice.
    pub fn of(event: &KeyEvent) -> Key {
        if event.kind != KeyEventKind::Press {
            return Key::Other;
        }
        // `Ctrl-C` reaches the program as a key in raw mode, not as a signal.
        if event.modifiers.contains(KeyModifiers::CONTROL) && event.code == KeyCode::Char('c') {
            return Key::Quit;
        }
        match event.code {
            KeyCode::Up | KeyCode::Char('k') => Key::Up,
            KeyCode::Down | KeyCode::Char('j') => Key::Down,
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => Key::In,
            KeyCode::Esc | KeyCode::Left | KeyCode::Backspace | KeyCode::Char('h') => Key::Out,
            KeyCode::Home | KeyCode::Char('g') => Key::Home,
            KeyCode::Char('?') => Key::Help,
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

/// The node in view and the selected row, each as its path from home, so both
/// survive a refresh that drops them: they fall back to their nearest parent.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Nav {
    /// Empty on the home screen.
    pub at: Vec<Node>,
    /// The path to the selected row; equal to `at` when the screen has no row.
    pub selected: Vec<Node>,
    pub is_help_shown: bool,
}

impl Nav {
    pub fn current(&self) -> Option<&Node> {
        self.at.last()
    }

    /// The selected row, `None` when the screen has none.
    pub fn selected_node(&self) -> Option<&Node> {
        (self.selected.len() > self.at.len())
            .then(|| self.selected.last())
            .flatten()
    }

    /// The same place in new data: a node that is gone gives way to its nearest parent.
    pub fn refreshed(&self, data: &WatchData) -> Nav {
        let at = self
            .at
            .iter()
            .rev()
            .find(|node| data.contains(node))
            .map_or_else(Vec::new, |node| data.trail(node));
        let selected = self
            .selected
            .iter()
            .rev()
            .find(|node| data.contains(node))
            .map(|node| data.trail(node))
            .filter(|trail| trail.len() > at.len() && trail.starts_with(&at));
        let selected = selected.unwrap_or_else(|| first_row(data, &at));
        Nav {
            at,
            selected,
            is_help_shown: self.is_help_shown,
        }
    }
}

/// The path to the first row of the screen of `at`, or `at` when it has none.
fn first_row(data: &WatchData, at: &[Node]) -> Vec<Node> {
    data.rows(at.last())
        .first()
        .map_or_else(|| at.to_vec(), |row| data.trail(&row.node))
}

/// The place after one key. The selection stops at the first and last row.
pub fn on_key(nav: &Nav, key: Key, data: &WatchData) -> (Nav, Action) {
    let nav = nav.refreshed(data);
    if key == Key::Quit {
        return (nav, Action::Quit);
    }
    if nav.is_help_shown {
        let closed = Nav {
            is_help_shown: false,
            ..nav
        };
        return (closed, Action::Continue);
    }
    let rows = data.rows(nav.current());
    let index = nav
        .selected_node()
        .and_then(|node| rows.iter().position(|row| &row.node == node));
    let select = |index: usize| -> Nav {
        Nav {
            selected: rows
                .get(index)
                .map_or_else(|| nav.at.clone(), |row| data.trail(&row.node)),
            ..nav.clone()
        }
    };
    let next = match key {
        Key::Up => select(index.unwrap_or(0).saturating_sub(1)),
        Key::Down => select(index.map_or(0, |index| (index + 1).min(rows.len().saturating_sub(1)))),
        Key::In => match nav.selected_node() {
            Some(_) => {
                let at = nav.selected.clone();
                Nav {
                    selected: first_row(data, &at),
                    at,
                    is_help_shown: false,
                }
            }
            None => nav,
        },
        Key::Out => {
            let mut at = nav.at.clone();
            if at.pop().is_some() {
                Nav {
                    selected: nav.at.clone(),
                    at,
                    is_help_shown: false,
                }
            } else {
                nav
            }
        }
        Key::Home => match nav.at.first() {
            Some(top) => Nav {
                at: Vec::new(),
                selected: vec![top.clone()],
                is_help_shown: false,
            },
            None => nav,
        },
        Key::Help => Nav {
            is_help_shown: true,
            ..nav
        },
        Key::Quit | Key::Other => nav,
    };
    (next, Action::Continue)
}

#[cfg(test)]
#[path = "nav_tests.rs"]
mod tests;
