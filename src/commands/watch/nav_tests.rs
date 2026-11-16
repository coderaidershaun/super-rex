//! Proves the walk through `rex watch` over the shared live fixture: each key on
//! each screen, the breadcrumb and the level rail at each step, and a refresh.

use super::*;
use crate::commands::watch::data::{Origin, WatchData};
use crate::commands::watch::live::Snapshot;
use crate::commands::watch::view::{breadcrumb, rail_text};

const FIXTURE: &str = include_str!("../../../fixtures/shared/live/snapshot.json");

/// `written_at` of the fixture, in milliseconds.
const WRITTEN_MS: u64 = 1_794_775_270_000;

fn fixture_data() -> WatchData {
    let part = Snapshot::parse(FIXTURE).unwrap().part(WRITTEN_MS);
    WatchData::merge(vec![part], None)
}

fn press(nav: &Nav, keys: &[Key], data: &WatchData) -> Nav {
    keys.iter()
        .fold(nav.clone(), |nav, key| on_key(&nav, *key, data).0)
}

fn place(nav: &Nav, data: &WatchData) -> (String, String, Option<Node>) {
    (
        breadcrumb(data, nav).join(" › "),
        rail_text(data, nav),
        nav.selected_node().cloned(),
    )
}

fn run(id: &str) -> Option<Node> {
    Some(Node::Run(id.into()))
}

#[test]
fn walks_down_a_tree_to_a_helper_and_back_up() {
    let data = fixture_data();
    let home = Nav::default().refreshed(&data);
    let rows: Vec<_> = data.rows(None).into_iter().map(|row| row.node).collect();
    assert_eq!(
        rows,
        [
            Node::Tree("t0geh4lh3i".into()),
            Node::Job("0gegkccde2".into())
        ],
        "home holds the tree and the job of no tree, newest first"
    );
    assert_eq!(
        place(&home, &data),
        ("home".into(), String::new(), Some(rows[0].clone()))
    );

    let tree = press(&home, &[Key::In], &data);
    assert_eq!(
        place(&tree, &data),
        (
            "home › tree t0geh4lh3i".into(),
            "[master] › builder › lead › worker › helper".into(),
            Some(Node::Job("0geh5fafkb".into()))
        )
    );

    let job = press(&tree, &[Key::In], &data);
    assert_eq!(
        place(&job, &data),
        (
            "home › tree t0geh4lh3i › job 0geh5fafkb".into(),
            "master › [builder] › lead › worker › helper".into(),
            run("adecision01")
        )
    );
    let indents: Vec<_> = data
        .rows(job.current())
        .iter()
        .map(|row| row.indent)
        .collect();
    assert_eq!(
        indents,
        [0, 0, 1, 2],
        "the job screen shows every agent below, indented"
    );

    let lead = press(&job, &[Key::Down, Key::In], &data);
    assert_eq!(
        place(&lead, &data),
        (
            "home › tree t0geh4lh3i › job 0geh5fafkb › lead engineer-lead".into(),
            "master › builder › [lead] › worker › helper".into(),
            run("aworker01")
        )
    );

    let worker = press(&lead, &[Key::In], &data);
    assert_eq!(
        place(&worker, &data).1,
        "master › builder › lead › [worker] › helper"
    );
    assert!(
        place(&worker, &data)
            .0
            .ends_with("› lead engineer-lead › worker engineer T1")
    );

    let helper = press(&worker, &[Key::In], &data);
    assert_eq!(
        place(&helper, &data).1,
        "master › builder › lead › worker › [helper]"
    );
    assert_eq!(helper.selected_node(), None, "the helper started no agent");
    assert_eq!(
        press(&helper, &[Key::In, Key::Down], &data),
        helper,
        "nothing to go into"
    );

    let back = press(&helper, &[Key::Out], &data);
    assert_eq!(back.at, worker.at);
    assert_eq!(
        back.selected_node(),
        run("ahelper01").as_ref(),
        "the row just left"
    );
    let up_twice = press(&helper, &[Key::Out, Key::Out, Key::Out], &data);
    assert_eq!(up_twice.at, job.at);
    assert_eq!(up_twice.selected_node(), run("alead01").as_ref());
    assert_eq!(press(&helper, &[Key::Home], &data).at, Vec::<Node>::new());
    assert_eq!(
        press(&helper, &[Key::Home], &data).selected_node(),
        Some(&Node::Tree("t0geh4lh3i".into()))
    );
    assert_eq!(press(&home, &[Key::Out], &data), home, "home is the top");
}

#[test]
fn a_deep_row_opens_with_its_whole_path() {
    let data = fixture_data();
    let job = press(&Nav::default(), &[Key::In, Key::In], &data);
    let worker = press(&job, &[Key::Down, Key::Down, Key::In], &data);
    assert_eq!(
        breadcrumb(&data, &worker),
        [
            "home",
            "tree t0geh4lh3i",
            "job 0geh5fafkb",
            "lead engineer-lead",
            "worker engineer T1"
        ]
    );
    assert_eq!(
        press(&worker, &[Key::Out], &data).current(),
        run("alead01").as_ref()
    );
}

#[test]
fn a_job_of_no_tree_has_three_levels() {
    let data = fixture_data();
    let job = press(&Nav::default(), &[Key::Down, Key::In], &data);
    assert_eq!(
        place(&job, &data),
        (
            "home › job 0gegkccde2".into(),
            "[builder] › lead › worker".into(),
            run("asolo01")
        )
    );
    let lead = press(&job, &[Key::In], &data);
    assert_eq!(rail_text(&data, &lead), "builder › [lead] › worker");
}

#[test]
fn moves_stop_at_the_ends_and_help_closes_on_any_key() {
    let data = fixture_data();
    let home = Nav::default().refreshed(&data);
    let last = Some(Node::Job("0gegkccde2".into()));
    assert_eq!(press(&home, &[Key::Up], &data), home);
    assert_eq!(
        press(&home, &[Key::Down, Key::Down], &data).selected_node(),
        last.as_ref()
    );
    assert_eq!(press(&home, &[Key::Down, Key::Up], &data), home);

    let help = press(&home, &[Key::Help], &data);
    assert!(help.is_help_shown);
    assert_eq!(
        press(&help, &[Key::Down], &data),
        home,
        "a key closes help and does nothing else"
    );
    assert_eq!(on_key(&help, Key::Quit, &data).1, Action::Quit);
    assert_eq!(on_key(&home, Key::Quit, &data).1, Action::Quit);
    assert_eq!(press(&home, &[Key::Other], &data), home);
}

#[test]
fn a_refresh_keeps_the_node_and_falls_back_to_its_parent() {
    let data = fixture_data();
    let lead = press(
        &Nav::default(),
        &[Key::In, Key::In, Key::Down, Key::In],
        &data,
    );

    // A new job above the tree moves the rows; the place follows by id.
    let mut grown = data.clone();
    let mut newer = grown.jobs[1].clone();
    newer.job.id = "0gzzzzzzzz".into();
    newer.job.created_at = "2026-11-15T23:00:00.000Z".into();
    newer.runs.clear();
    grown.jobs.insert(0, newer);
    assert_eq!(lead.refreshed(&grown), lead);

    // The worker row is gone: the selection falls back to the screen's first row.
    let mut shrunk = data.clone();
    shrunk.jobs[0]
        .runs
        .retain(|run| run.agent_id != "aworker01" && run.agent_id != "ahelper01");
    let fallen = lead.refreshed(&shrunk);
    assert_eq!(fallen.at, lead.at);
    assert_eq!(
        fallen.selected_node(),
        None,
        "the lead has no agent below it now"
    );

    // The lead in view is gone: the view falls back to its job.
    shrunk.jobs[0].runs.retain(|run| run.agent_id != "alead01");
    let fallen = lead.refreshed(&shrunk);
    assert_eq!(fallen.current(), Some(&Node::Job("0geh5fafkb".into())));
    assert_eq!(fallen.selected_node(), run("adecision01").as_ref());

    // The worker selected on the job screen goes: its lead is selected.
    let on_job = press(
        &Nav::default(),
        &[Key::In, Key::In, Key::Down, Key::Down],
        &data,
    );
    assert_eq!(on_job.selected_node(), run("aworker01").as_ref());
    let mut no_worker = data.clone();
    no_worker.jobs[0]
        .runs
        .retain(|run| run.agent_id != "aworker01" && run.agent_id != "ahelper01");
    assert_eq!(
        on_job.refreshed(&no_worker).selected_node(),
        run("alead01").as_ref()
    );

    // Everything goes: the view falls back home.
    let empty = WatchData::merge(Vec::new(), None);
    assert_eq!(lead.refreshed(&empty), Nav::default());
    assert_eq!(data.jobs[0].origin, Origin::Live);
}

#[test]
fn a_job_whose_tree_is_in_no_snapshot_stands_alone_with_three_levels() {
    let mut part = Snapshot::parse(FIXTURE).unwrap().part(WRITTEN_MS);
    part.trees.clear();
    let data = WatchData::merge(vec![part], None);

    let home: Vec<_> = data.rows(None).into_iter().map(|row| row.node).collect();
    assert!(home.contains(&Node::Job("0geh5fafkb".into())), "{home:?}");
    let job = press(&Nav::default(), &[Key::In], &data);
    assert_eq!(
        place(&job, &data).1,
        "[builder] › lead › worker › helper",
        "no master without its tree"
    );
}

#[test]
fn a_run_whose_parent_is_missing_hangs_below_the_builder() {
    let mut part = Snapshot::parse(FIXTURE).unwrap().part(WRITTEN_MS);
    part.jobs[0].runs.retain(|run| run.agent_id != "alead01");
    let data = WatchData::merge(vec![part], None);

    let worker = Node::Run("aworker01".into());
    assert_eq!(data.level(&worker), Level::Lead);
    assert_eq!(data.parent(&worker), Some(Node::Job("0geh5fafkb".into())));
    let job = press(&Nav::default(), &[Key::In, Key::In], &data);
    let rows: Vec<_> = data
        .rows(job.current())
        .into_iter()
        .map(|row| (row.node, row.indent))
        .collect();
    assert_eq!(
        rows,
        [
            (Node::Run("adecision01".into()), 0),
            (worker, 0),
            (Node::Run("ahelper01".into()), 1)
        ]
    );
}
