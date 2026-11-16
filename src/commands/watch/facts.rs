//! The facts panel of `rex watch`: what one tree, job or agent holds, as label
//! and value, and the questions that wait on it with the command that answers each.

use super::data::{JobBundle, QuestionData, RunData, SpendFrom, TreeBundle, WatchData};
use super::nav::Node;
use super::view::{
    cost_or_dash, created_text, dash, duration_text, first_line, or_dash, origin_text, short_model,
    spend_text,
};
use crate::capture::{Cost, TokenCounts};
use crate::commands::token_text::{hit_text, short_count};

impl WatchData {
    /// The facts of the node in view, as label and value, in the order shown.
    pub fn facts(&self, node: &Node) -> Vec<(&'static str, String)> {
        match node {
            Node::Tree(id) => self.tree(id).map_or_else(Vec::new, tree_facts),
            Node::Job(id) => self.job(id).map_or_else(Vec::new, job_facts),
            Node::Run(agent) => self
                .run(agent)
                .map_or_else(Vec::new, |(_, run)| run_facts(run)),
        }
    }
}

/// `a, b`, or `none` for an empty list.
fn list_text(items: &[String]) -> String {
    if items.is_empty() {
        "none".to_owned()
    } else {
        items.join(", ")
    }
}

/// `in 1.2k  cache-w 160.0k  cache-r 2.1M  out 30.5k  hit 92%`.
fn tokens_text(tokens: Option<TokenCounts>) -> String {
    tokens.map_or_else(dash, |counts| {
        format!(
            "in {}  cache-w {}  cache-r {}  out {}  hit {}",
            short_count(counts.input),
            short_count(counts.cache_creation),
            short_count(counts.cache_read),
            short_count(counts.output),
            hit_text(&counts)
        )
    })
}

/// `opus-5-5 high  done  $0.400`: the model, effort, state and cost of a run.
fn run_summary(run: &RunData) -> String {
    format!(
        "{} {}  {}  {}",
        short_model(run.model.as_deref()),
        or_dash(run.effort.as_deref()),
        run.state,
        cost_or_dash(run.cost)
    )
}

fn spend_fact(spent: Option<Cost>, from: SpendFrom, limit_nano: Option<u64>) -> String {
    let text = spend_text(spent, from, limit_nano);
    match from {
        SpendFrom::Own => text,
        SpendFrom::LiveTally => format!("{text}  (live tally, not the cost of record)"),
    }
}

/// The tree's spend counts its master and every job against the cap; the
/// master line gives the master's own cost apart.
fn tree_facts(bundle: &TreeBundle) -> Vec<(&'static str, String)> {
    let tree = &bundle.tree;
    let mut facts = vec![
        (
            "state",
            format!("{}  ({})", tree.state, origin_text(bundle.origin)),
        ),
        (
            "charter",
            tree.charter.lines().take(2).collect::<Vec<_>>().join(" / "),
        ),
        (
            "spend",
            spend_fact(tree.spent, tree.spent_from, Some(tree.cap_nano)),
        ),
        (
            "master",
            bundle
                .master_run
                .as_ref()
                .map_or_else(|| or_dash(tree.master.as_deref()), run_summary),
        ),
    ];
    if let Some(report) = &tree.report_file {
        facts.push(("report", report.clone()));
    }
    for note in &tree.notes {
        facts.push(("note", format!("{}  {}", created_text(&note.at), note.text)));
    }
    facts
}

fn job_facts(bundle: &JobBundle) -> Vec<(&'static str, String)> {
    let job = &bundle.job;
    let builder = bundle.builder_run().map_or_else(
        || "main session".to_owned(),
        |run| {
            format!(
                "{} {} {}",
                short_model(run.model.as_deref()),
                or_dash(run.effort.as_deref()),
                run.state
            )
        },
    );
    let deliveries: Vec<String> = job
        .deliveries
        .iter()
        .map(|delivery| match &delivery.note {
            Some(note) => format!("{} {} ({note})", delivery.repo, delivery.outcome),
            None => format!("{} {}", delivery.repo, delivery.outcome),
        })
        .collect();
    vec![
        (
            "status",
            format!("{}  ({})", job.status, origin_text(bundle.origin)),
        ),
        ("request", first_line(&job.request)),
        (
            "orchestrator",
            format!(
                "{}  builder {builder}",
                or_dash(job.orchestrator.as_deref())
            ),
        ),
        (
            "merge",
            format!(
                "{} by {}",
                or_dash(job.merge.as_deref()),
                or_dash(job.merge_source.as_deref())
            ),
        ),
        (
            "spend",
            spend_fact(job.spent, job.spent_from, job.limit_nano),
        ),
        (
            "branch",
            format!(
                "{} on {}  {}",
                or_dash(job.branch.as_deref()),
                or_dash(job.base.as_deref()),
                or_dash(job.worktree.as_deref())
            ),
        ),
        ("delivered", list_text(&deliveries)),
        ("done checks", list_text(&job.done_checks)),
        ("degraded", list_text(&job.degraded)),
        ("tokens", tokens_text(job.tokens)),
    ]
}

/// The task has its own line only when it says more than the role, as a
/// worker's `T1` does; a lead's task is its role.
fn run_facts(run: &RunData) -> Vec<(&'static str, String)> {
    let mut facts = vec![("role", or_dash(run.role.as_deref()))];
    if let Some(task) = run
        .task
        .as_deref()
        .filter(|task| run.role.as_deref() != Some(*task))
    {
        facts.push(("task", task.to_owned()));
    }
    facts.extend([
        (
            "agent",
            format!(
                "{}  planned {}",
                or_dash(run.agent_type.as_deref()),
                or_dash(run.planned.as_deref())
            ),
        ),
        (
            "ran",
            format!(
                "{} {}",
                short_model(run.model.as_deref()),
                or_dash(run.effort.as_deref())
            ),
        ),
        (
            "state",
            format!("{}  {}", run.state, duration_text(run.duration_ms)),
        ),
        (
            "tokens",
            format!(
                "{}  cost {}",
                tokens_text(run.tokens),
                cost_or_dash(run.cost)
            ),
        ),
        ("brief check", or_dash(run.lint.as_deref())),
        ("skills given", list_text(&run.skills_attached)),
        ("skills opened", list_text(&run.skills_opened)),
        ("report", or_dash(run.output_file.as_deref())),
    ]);
    facts
}

/// The waiting questions of the node in view: a tree lists its own and its
/// jobs', a job lists its own.
pub fn waiting_questions<'a>(data: &'a WatchData, node: &Node) -> Vec<&'a QuestionData> {
    let questions: Vec<&QuestionData> = match node {
        Node::Tree(id) => data
            .tree(id)
            .into_iter()
            .flat_map(|bundle| &bundle.questions)
            .chain(
                data.tree_jobs(id)
                    .into_iter()
                    .flat_map(|bundle| &bundle.questions),
            )
            .collect(),
        Node::Job(id) => data
            .job(id)
            .into_iter()
            .flat_map(|bundle| &bundle.questions)
            .collect(),
        Node::Run(_) => Vec::new(),
    };
    questions
        .into_iter()
        .filter(|question| !question.is_answered)
        .collect()
}

/// `1 Merge (recommended)  2 Keep the branch`.
pub fn options_text(question: &QuestionData) -> String {
    question
        .options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let mark = if *option == question.recommendation {
                " (recommended)"
            } else {
                ""
            };
            format!("{} {option}{mark}", index + 1)
        })
        .collect::<Vec<_>>()
        .join("  ")
}

/// The command the person types in the session to answer.
pub fn answer_command(question: &QuestionData) -> String {
    format!("/rex answer {} <option number>", question.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::watch::live::Snapshot;

    const FIXTURE: &str = include_str!("../../../fixtures/shared/live/snapshot.json");

    fn fact<'a>(facts: &'a [(&'static str, String)], label: &str) -> Option<&'a str> {
        facts
            .iter()
            .find(|(shown, _)| *shown == label)
            .map(|(_, value)| value.as_str())
    }

    fn fixture_data() -> WatchData {
        let snapshot = Snapshot::parse(FIXTURE).unwrap();
        WatchData::merge(vec![snapshot.part(1_794_775_270_000)], None)
    }

    #[test]
    fn a_worker_names_its_task_once_and_a_lead_not_twice() {
        let data = fixture_data();
        let worker = data.facts(&Node::Run("aworker01".into()));
        assert_eq!(fact(&worker, "role"), Some("engineer"));
        assert_eq!(fact(&worker, "task"), Some("T1"));

        let lead = data.facts(&Node::Run("alead01".into()));
        assert_eq!(fact(&lead, "role"), Some("engineer-lead"));
        assert_eq!(fact(&lead, "task"), None, "the task is the role");

        let helper = data.facts(&Node::Run("ahelper01".into()));
        assert_eq!(fact(&helper, "role"), Some("-"));
        assert_eq!(fact(&helper, "task"), None);
    }

    #[test]
    fn a_tree_shows_its_spend_of_the_cap_and_its_masters_own_cost() {
        let data = fixture_data();
        let tree = data.facts(&Node::Tree("t0geh4lh3i".into()));
        assert_eq!(fact(&tree, "spend"), Some("$3.65 of $150"));
        assert_eq!(fact(&tree, "master"), Some("opus-5-5 high  done  $0.400"));
    }
}
