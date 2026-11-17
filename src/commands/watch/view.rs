//! The texts of `rex watch`: one row of each node, the breadcrumb, the level rail
//! and the tree guides, the same in the terminal and in `--once`.

use ratatui::text::Span;

use super::data::{Origin, QuestionData, RunData, SpendFrom, WatchData, nano_cost};
use super::nav::{Level, Nav, Node};
use crate::capture::{Cost, TokenCounts};
use crate::commands::token_text::{cost_text, hit_text};

/// What the screen says when there is nothing to show.
pub const NOTHING_YET: &str = "No job has run here yet. A job starts when you ask for work in a \
     Claude Code session with the super-rex plugin, or a tree when you type /super-rex:orchestration-master <goal> there.";

/// One row of a screen or one line of `--once`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowText {
    pub level: Level,
    /// The tree id, the job id, or the role and task of a run.
    pub name: String,
    pub state: String,
    /// `live`, `ended` or `store` for a tree or a job, empty for a run.
    pub origin: &'static str,
    /// The model that ran, without `claude-`, else `-`.
    pub model: String,
    pub time: String,
    /// Spend, with the cap or limit after it when there is one.
    pub cost: String,
    pub hit: String,
    /// The first line of the charter or the request, or the agent type of a run
    /// without the plugin's prefix.
    pub detail: String,
    pub waiting: usize,
}

pub fn dash() -> String {
    "-".to_owned()
}

pub fn or_dash(value: Option<&str>) -> String {
    value.map_or_else(dash, str::to_owned)
}

pub fn cost_or_dash(cost: Option<Cost>) -> String {
    cost.map_or_else(dash, |cost| cost_text(&cost))
}

/// `$3.25 of $28.00`, or the spend alone when there is no limit. A store row's
/// spend with a snapshot's live tally in it starts with `≈`, since `~` marks a
/// provisional cost of record elsewhere.
pub fn spend_text(spent: Option<Cost>, from: SpendFrom, limit_nano: Option<u64>) -> String {
    let mark = match from {
        SpendFrom::Own => "",
        SpendFrom::LiveTally => "≈",
    };
    let spent = format!("{mark}{}", cost_or_dash(spent));
    match limit_nano {
        Some(limit) => format!("{spent} of {}", cost_text(&nano_cost(limit))),
        None => spent,
    }
}

fn hit(tokens: Option<TokenCounts>) -> String {
    tokens.map_or_else(dash, |counts| hit_text(&counts))
}

pub fn short_model(model: Option<&str>) -> String {
    model.map_or_else(dash, |model| {
        model.strip_prefix("claude-").unwrap_or(model).to_owned()
    })
}

pub fn origin_text(origin: Origin) -> &'static str {
    match origin {
        Origin::Live => "live",
        Origin::Store => "store",
        Origin::Ended => "ended",
    }
}

pub fn first_line(text: &str) -> String {
    text.lines().next().unwrap_or_default().to_owned()
}

/// `1 tree`, `2 trees`.
pub fn counted(count: usize, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

/// The agent type without the plugin's prefix: `model-low-effort-low`.
pub fn short_agent_type(run: &RunData) -> Option<&str> {
    run.agent_type
        .as_deref()
        .map(|name| name.trim_start_matches("super-rex:"))
}

/// The name after the level word: `engineer T1`, the role, then the task when
/// it says more. A role that is the level word is not said again, so a worker
/// of task T1 is `T1`, and a lead with no task has no name. A run with no role,
/// such as an agent started before the job has a plan, is named by its task;
/// with neither, such as a helper an agent started on its own, by its agent type.
pub fn run_name(run: &RunData, level: Level) -> String {
    let role = run.role.as_deref().filter(|role| *role != level.name());
    let task = run
        .task
        .as_deref()
        .filter(|task| Some(*task) != run.role.as_deref());
    match (role, task) {
        (Some(role), Some(task)) => format!("{role} {task}"),
        (Some(name), None) | (None, Some(name)) => name.to_owned(),
        (None, None) if run.role.is_some() => String::new(),
        (None, None) => short_agent_type(run).unwrap_or(&run.agent_id).to_owned(),
    }
}

/// The level word, then the name when there is one: `worker T1`, `lead`.
pub fn labelled(level: Level, name: &str) -> String {
    if name.is_empty() {
        level.name().to_owned()
    } else {
        format!("{} {name}", level.name())
    }
}

pub fn waiting(questions: &[QuestionData]) -> usize {
    questions
        .iter()
        .filter(|question| !question.is_answered)
        .count()
}

/// `41s`, `2m05s`, `1h02m`. `-` when the run has no duration.
pub fn duration_text(duration_ms: Option<u64>) -> String {
    let Some(milliseconds) = duration_ms else {
        return dash();
    };
    let seconds = milliseconds / 1_000;
    if seconds < 60 {
        format!("{seconds}s")
    } else if seconds < 3_600 {
        format!("{}m{:02}s", seconds / 60, seconds % 60)
    } else {
        format!("{}h{:02}m", seconds / 3_600, seconds % 3_600 / 60)
    }
}

/// `2026-11-10T09:05:00.000Z` as `2026-11-10 09:05`. Other text stays as it is.
pub fn created_text(created_at: &str) -> String {
    match (
        created_at.get(..10),
        created_at.get(10..11),
        created_at.get(11..16),
    ) {
        (Some(date), Some("T"), Some(time)) => format!("{date} {time}"),
        _ => created_at.to_owned(),
    }
}

/// `2026-11-10T09:05:00.000Z` as `11-10 09:05`, short enough for a row.
fn short_created(created_at: &str) -> String {
    let full = created_text(created_at);
    full.get(5..)
        .filter(|_| full != created_at)
        .map_or(full.clone(), str::to_owned)
}

impl WatchData {
    /// The row of `node`, `None` when the data does not hold it.
    pub fn row_text(&self, node: &Node) -> Option<RowText> {
        match node {
            Node::Tree(id) => {
                let bundle = self.tree(id)?;
                let tree = &bundle.tree;
                let master = bundle.master_run.as_ref();
                let jobs = self.tree_jobs(id);
                Some(RowText {
                    level: Level::Master,
                    name: tree.id.clone(),
                    state: tree.state.clone(),
                    origin: origin_text(bundle.origin),
                    model: short_model(master.and_then(|run| run.model.as_deref())),
                    time: short_created(&tree.started_at),
                    cost: spend_text(tree.spent, tree.spent_from, Some(tree.cap_nano)),
                    hit: dash(),
                    detail: first_line(&tree.charter),
                    waiting: waiting(&bundle.questions)
                        + jobs
                            .iter()
                            .map(|job| waiting(&job.questions))
                            .sum::<usize>(),
                })
            }
            Node::Job(id) => {
                let bundle = self.job(id)?;
                let job = &bundle.job;
                Some(RowText {
                    level: Level::Builder,
                    name: job.id.clone(),
                    state: job.status.clone(),
                    origin: origin_text(bundle.origin),
                    model: short_model(bundle.builder_run().and_then(|run| run.model.as_deref())),
                    time: short_created(&job.created_at),
                    cost: spend_text(job.spent, job.spent_from, job.limit_nano),
                    hit: hit(job.tokens),
                    detail: first_line(&job.request),
                    waiting: waiting(&bundle.questions),
                })
            }
            Node::Run(agent) => {
                let (_, run) = self.run(agent)?;
                let level = self.level(node);
                Some(RowText {
                    level,
                    name: run_name(run, level),
                    state: run.state.clone(),
                    origin: "",
                    model: short_model(run.model.as_deref()),
                    time: duration_text(run.duration_ms),
                    cost: cost_or_dash(run.cost),
                    hit: hit(run.tokens),
                    detail: or_dash(short_agent_type(run)),
                    waiting: 0,
                })
            }
        }
    }
}

/// `home › tree t0geh4lh3i › job 0geh5fafkb › lead engineer-lead`.
pub fn breadcrumb(data: &WatchData, nav: &Nav) -> Vec<String> {
    let mut crumbs = vec!["home".to_owned()];
    for node in &nav.at {
        let name = data.row_text(node).map_or_else(dash, |row| row.name);
        let crumb = match node {
            Node::Tree(_) => format!("tree {name}"),
            Node::Job(_) => format!("job {name}"),
            Node::Run(_) => labelled(data.level(node), &name),
        };
        crumbs.push(crumb);
    }
    crumbs
}

/// The crumbs that fit in `width` columns joined by ` › `. The crumbs nearest
/// home give way first, to one `…`, so the node in view always shows.
pub fn fit_crumbs(crumbs: &[String], width: usize) -> Vec<String> {
    let joined_width = |parts: &[String]| {
        parts.iter().map(|part| text_width(part)).sum::<usize>() + 3 * parts.len().saturating_sub(1)
    };
    if joined_width(crumbs) <= width {
        return crumbs.to_vec();
    }
    for first_kept in 1..crumbs.len() {
        let mut parts = vec!["…".to_owned()];
        parts.extend_from_slice(&crumbs[first_kept..]);
        if joined_width(&parts) <= width {
            return parts;
        }
    }
    let last = crumbs
        .last()
        .map_or_else(String::new, |crumb| ellipsis(crumb, width));
    vec![last]
}

/// The levels of the run in view, the current one in brackets, such as
/// `master › [builder] › lead › worker`. Empty on the home screen.
pub fn rail_text(data: &WatchData, nav: &Nav) -> String {
    let Some(at) = nav.current() else {
        return String::new();
    };
    let current = data.level(at);
    data.rail(at)
        .into_iter()
        .map(|level| {
            if level == current {
                format!("[{}]", level.name())
            } else {
                level.name().to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(" › ")
}

/// The guide in front of each row, such as `│  └─ `, from the depth of every
/// row in the order they are shown. A row at depth 0 has none.
pub fn tree_guides(depths: &[usize]) -> Vec<String> {
    (0..depths.len())
        .map(|row| {
            let depth = depths[row];
            (1..=depth)
                .map(
                    |level| match (level == depth, has_later_sibling(depths, row, level)) {
                        (true, true) => "├─ ",
                        (true, false) => "└─ ",
                        (false, true) => "│  ",
                        (false, false) => "   ",
                    },
                )
                .collect()
        })
        .collect()
}

/// Whether a row at `level` comes after `row` before the tree climbs above `level`.
fn has_later_sibling(depths: &[usize], row: usize, level: usize) -> bool {
    depths[row + 1..]
        .iter()
        .take_while(|depth| **depth >= level)
        .any(|depth| *depth == level)
}

/// How many terminal columns `text` takes: a wide character, such as a Chinese
/// one or an emoji, takes two.
pub fn text_width(text: &str) -> usize {
    Span::raw(text).width()
}

/// `text` cut to `width` terminal columns, with `…` as its last when it was
/// wider. A wide character is kept whole or left out, never cut in half.
pub fn ellipsis(text: &str, width: usize) -> String {
    if text_width(text) <= width {
        return text.to_owned();
    }
    if width == 0 {
        return String::new();
    }
    let mut kept = String::new();
    let mut used = 0;
    let mut buffer = [0; 4];
    for character in text.chars() {
        let character_width = text_width(character.encode_utf8(&mut buffer));
        if used + character_width > width - 1 {
            break;
        }
        kept.push(character);
        used += character_width;
    }
    kept.push('…');
    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ellipsis_counts_terminal_columns_and_keeps_wide_characters_whole() {
        assert_eq!(ellipsis("hello", 5), "hello");
        assert_eq!(ellipsis("hello world", 6), "hello…");
        // Each of these takes two columns: four of them fill eight.
        assert_eq!(ellipsis("構築する仕事", 8), "構築す…");
        assert_eq!(text_width(&ellipsis("構築する仕事", 8)), 7);
        assert_eq!(ellipsis("構築する仕事", 6), "構築…");
        assert_eq!(ellipsis("abc", 0), "");
        assert_eq!(ellipsis("abc", 1), "…");
    }

    fn run(role: Option<&str>, task: Option<&str>) -> RunData {
        RunData {
            agent_id: "a1b2c3".into(),
            parent: None,
            role: role.map(str::to_owned),
            task: task.map(str::to_owned),
            agent_type: Some("super-rex:model-high-effort-medium".into()),
            planned: None,
            model: None,
            effort: None,
            state: "open".into(),
            started_at: None,
            duration_ms: None,
            tokens: None,
            cost: None,
            lint: None,
            skills_attached: Vec::new(),
            skills_opened: Vec::new(),
            output_file: None,
        }
    }

    #[test]
    fn a_run_is_named_by_its_role_else_its_task_else_its_agent_type() {
        let lead = Level::Lead;
        assert_eq!(
            run_name(&run(Some("engineer"), Some("T1")), lead),
            "engineer T1"
        );
        assert_eq!(
            run_name(&run(Some("engineer-lead"), Some("engineer-lead")), lead),
            "engineer-lead"
        );
        assert_eq!(
            run_name(&run(Some("engineer-lead"), None), lead),
            "engineer-lead"
        );
        assert_eq!(
            run_name(&run(None, Some("decision-challenger")), lead),
            "decision-challenger",
            "an agent started before the plan has a task and no role"
        );
        assert_eq!(run_name(&run(None, None), lead), "model-high-effort-medium");
    }

    #[test]
    fn a_role_that_is_the_level_word_is_said_once() {
        let label = |role, task, level| labelled(level, &run_name(&run(role, task), level));
        assert_eq!(label(Some("lead"), Some("T5"), Level::Lead), "lead T5");
        assert_eq!(
            label(Some("worker"), Some("T1"), Level::Worker),
            "worker T1"
        );
        assert_eq!(label(Some("helper"), None, Level::Helper), "helper");
        assert_eq!(
            label(Some("lead"), Some("T1"), Level::Worker),
            "worker lead T1",
            "a role that is another level word is still said"
        );
    }

    #[test]
    fn fit_crumbs_gives_up_the_crumbs_nearest_home_first() {
        let crumbs: Vec<String> = [
            "home",
            "tree t0geh4lh3i",
            "job 0geh5fafkb",
            "lead engineer-lead",
        ]
        .map(str::to_owned)
        .into();
        assert_eq!(fit_crumbs(&crumbs, 80), crumbs);
        assert_eq!(
            fit_crumbs(&crumbs, 40),
            ["…", "job 0geh5fafkb", "lead engineer-lead"]
        );
        assert_eq!(fit_crumbs(&crumbs, 22), ["…", "lead engineer-lead"]);
        assert_eq!(fit_crumbs(&crumbs, 10), ["lead engi…"]);
    }
}
