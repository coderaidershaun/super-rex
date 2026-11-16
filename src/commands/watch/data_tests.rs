//! Proves the merge of `rex watch`: which source wins a tree or a job, the order,
//! and the spend a store row takes from a snapshot when the store has no cost.

use super::*;

const T: &str = "2026-11-15T20:00:00.000Z";

fn job(id: &str, created_at: &str, origin: Origin) -> JobBundle {
    JobBundle {
        job: JobData {
            id: id.into(),
            tree: None,
            builder: "main".into(),
            status: "planned".into(),
            request: format!("request from {origin:?}"),
            work_type: None,
            size: None,
            consequence: None,
            orchestrator: None,
            merge: None,
            merge_source: None,
            branch: None,
            base: None,
            worktree: None,
            limit_nano: None,
            spent: None,
            spent_from: SpendFrom::Own,
            tokens: None,
            degraded: Vec::new(),
            deliveries: Vec::new(),
            done_checks: Vec::new(),
            created_at: created_at.into(),
        },
        runs: Vec::new(),
        questions: Vec::new(),
        origin,
    }
}

/// A job of the tree `tree` that has spent `spent` nanodollars, when known.
fn tree_job(id: &str, tree: &str, spent: Option<u64>, origin: Origin) -> JobBundle {
    let mut bundle = job(id, T, origin);
    bundle.job.tree = Some(tree.into());
    bundle.job.spent = spent.map(nano_cost);
    bundle
}

fn master_run(spent: u64) -> RunData {
    RunData {
        agent_id: "amaster".into(),
        parent: None,
        role: Some("master".into()),
        task: None,
        agent_type: Some("super-rex:master".into()),
        planned: None,
        model: None,
        effort: None,
        state: "done".into(),
        started_at: Some(T.into()),
        duration_ms: None,
        tokens: None,
        cost: Some(nano_cost(spent)),
        lint: None,
        skills_attached: Vec::new(),
        skills_opened: Vec::new(),
        output_file: None,
    }
}

fn tree(id: &str, origin: Origin, master: Option<RunData>, spent: Option<u64>) -> TreeBundle {
    TreeBundle {
        tree: TreeData {
            id: id.into(),
            state: "done".into(),
            master: Some("amaster".into()),
            charter: "charter".into(),
            spent: spent.map(nano_cost),
            spent_from: SpendFrom::Own,
            cap_nano: 150_000_000_000,
            started_at: T.into(),
            closed_at: None,
            report_file: None,
            notes: Vec::new(),
        },
        master_run: master,
        questions: Vec::new(),
        origin,
    }
}

fn part(jobs: Vec<JobBundle>) -> Part {
    Part {
        trees: Vec::new(),
        jobs,
    }
}

#[test]
fn merge_prefers_live_then_store_then_ended() {
    let ended = part(vec![
        job("0ga", T, Origin::Ended),
        job("0gb", T, Origin::Ended),
        job("0gc", T, Origin::Ended),
    ]);
    let live = part(vec![job("0ga", T, Origin::Live)]);
    let store = part(vec![
        job("0ga", T, Origin::Store),
        job("0gb", T, Origin::Store),
    ]);

    let data = WatchData::merge(vec![ended, live], Some(store));

    let origin = |id: &str| data.job(id).unwrap().origin;
    assert_eq!(origin("0ga"), Origin::Live, "a live session wins");
    assert_eq!(
        origin("0gb"),
        Origin::Store,
        "the store beats an ended session"
    );
    assert_eq!(origin("0gc"), Origin::Ended, "an ended session is kept");
    assert_eq!(data.sources.text(), "live + store");
}

#[test]
fn two_live_sessions_with_one_job_show_the_newest_snapshot() {
    let mut newer = job("0ga", T, Origin::Live);
    newer.job.status = "running".into();
    let older = job("0ga", T, Origin::Live);

    let data = WatchData::merge(vec![part(vec![newer]), part(vec![older])], None);

    assert_eq!(data.jobs.len(), 1, "one job, not two");
    assert_eq!(data.jobs[0].job.status, "running");
}

#[test]
fn merge_sorts_newest_first_and_names_its_sources() {
    let store = part(vec![
        job("0ga", "2026-11-15T20:00:00.000Z", Origin::Store),
        job("0gb", "2026-11-15T21:00:00.000Z", Origin::Store),
    ]);
    let data = WatchData::merge(Vec::new(), Some(store));
    let ids: Vec<_> = data
        .jobs
        .iter()
        .map(|bundle| bundle.job.id.as_str())
        .collect();
    assert_eq!(ids, ["0gb", "0ga"]);
    assert_eq!(data.sources.text(), "store");
    assert_eq!(WatchData::merge(Vec::new(), None).sources.text(), "none");
}

// When a tree closes its snapshot stops and the store's row wins; a store that
// has not read the cost of record yet must not turn the spend shown into none.
#[test]
fn a_store_job_with_no_cost_shows_the_newest_snapshots_live_tally() {
    let mut older = job("0ga", T, Origin::Ended);
    older.job.spent = Some(nano_cost(1_000_000_000));
    let mut newer = job("0ga", T, Origin::Ended);
    newer.job.spent = Some(nano_cost(1_736_290_600));
    let mut recorded = job("0gb", T, Origin::Ended);
    recorded.job.spent = Some(nano_cost(9_000_000_000));
    let mut of_record = job("0gb", T, Origin::Store);
    of_record.job.spent = Some(nano_cost(2_000_000_000));
    let store = part(vec![job("0ga", T, Origin::Store), of_record]);

    let data = WatchData::merge(
        vec![part(vec![newer]), part(vec![older, recorded])],
        Some(store),
    );

    let shown = |id: &str| {
        let bundle = data.job(id).unwrap();
        (bundle.origin, bundle.job.spent, bundle.job.spent_from)
    };
    assert_eq!(
        shown("0ga"),
        (
            Origin::Store,
            Some(nano_cost(1_736_290_600)),
            SpendFrom::LiveTally
        )
    );
    assert_eq!(
        shown("0gb"),
        (
            Origin::Store,
            Some(nano_cost(2_000_000_000)),
            SpendFrom::Own
        ),
        "the cost of record beats a tally"
    );
    let spend = |id: &str| {
        let facts = data.facts(&crate::commands::watch::nav::Node::Job(id.into()));
        let fact = facts.into_iter().find(|(label, _)| *label == "spend");
        fact.map(|(_, value)| value)
    };
    assert_eq!(
        spend("0ga").as_deref(),
        Some("≈$1.73  (live tally, not the cost of record)")
    );
    assert_eq!(spend("0gb").as_deref(), Some("$2.00"));
}

#[test]
fn a_store_tree_spends_what_its_jobs_show_and_its_masters_own_spend() {
    let snapshot = Part {
        trees: vec![tree(
            "t0a",
            Origin::Ended,
            Some(master_run(421_243_600)),
            Some(2_157_534_200),
        )],
        jobs: vec![tree_job("0ga", "t0a", Some(1_736_290_600), Origin::Ended)],
    };
    let store = || Part {
        trees: vec![tree("t0a", Origin::Store, None, None)],
        jobs: vec![tree_job("0ga", "t0a", None, Origin::Store)],
    };

    let tally = WatchData::merge(vec![snapshot.clone()], Some(store()));
    let shown = tally.tree("t0a").unwrap();
    assert_eq!(shown.origin, Origin::Store);
    assert_eq!(
        (shown.tree.spent, shown.tree.spent_from),
        (Some(nano_cost(2_157_534_200)), SpendFrom::LiveTally),
        "the job's tally and the master's own spend"
    );
    assert_eq!(shown.master_run, Some(master_run(421_243_600)));

    let mut of_record = store();
    of_record.jobs[0].job.spent = Some(nano_cost(1_900_000_000));
    let mixed = WatchData::merge(vec![snapshot], Some(of_record.clone()));
    assert_eq!(
        mixed.tree("t0a").unwrap().tree.spent,
        Some(nano_cost(2_321_243_600)),
        "the job's cost of record and the master's own spend"
    );

    let alone = WatchData::merge(Vec::new(), Some(of_record));
    let shown = alone.tree("t0a").unwrap();
    assert_eq!(
        (shown.tree.spent, shown.tree.spent_from),
        (Some(nano_cost(1_900_000_000)), SpendFrom::Own),
        "with no snapshot, the jobs' cost of record alone"
    );

    let unknown = WatchData::merge(Vec::new(), Some(store()));
    assert_eq!(
        unknown.tree("t0a").unwrap().tree.spent,
        None,
        "nothing known"
    );
}
