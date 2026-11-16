//! Proves the live loader over the shared fixture: its time form, what it loads,
//! a stale session, a half-written file, and the removal of week-old snapshots.

use super::*;

const FIXTURE: &str = include_str!("../../../fixtures/shared/live/snapshot.json");

/// `written_at` of the fixture, in milliseconds.
const WRITTEN_MS: u64 = 1_794_775_270_000;

#[test]
fn epoch_ms_reads_the_snapshot_time_form() {
    assert_eq!(epoch_ms("1970-01-01T00:00:00.000Z"), Some(0));
    assert_eq!(epoch_ms("2026-11-15T20:41:10.000Z"), Some(WRITTEN_MS));
    assert_eq!(
        epoch_ms("2024-02-29T00:00:00.001Z"),
        Some(1_709_164_800_001)
    );
    assert_eq!(epoch_ms("2026-11-15 20:41:10"), None);
    assert_eq!(epoch_ms("2026-13-15T20:41:10.000Z"), None);
}

#[test]
fn fixture_loads_its_tree_jobs_runs_and_questions() {
    let part = Snapshot::parse(FIXTURE).unwrap().part(WRITTEN_MS);

    assert_eq!(part.trees.len(), 1);
    let tree = &part.trees[0];
    assert_eq!(tree.tree.id, "t0geh4lh3i");
    assert_eq!(tree.origin, Origin::Live);
    assert_eq!(tree.master_run.as_ref().unwrap().agent_id, "amaster01");
    assert_eq!(tree.questions.len(), 1, "the scope question has no job");

    let ids: Vec<_> = part
        .jobs
        .iter()
        .map(|bundle| bundle.job.id.as_str())
        .collect();
    assert_eq!(ids, ["0geh5fafkb", "0gegkccde2"]);
    let tree_job = &part.jobs[0];
    assert_eq!(tree_job.job.tree.as_deref(), Some("t0geh4lh3i"));
    assert_eq!(tree_job.builder_run().unwrap().agent_id, "abuilder01");
    let shape: Vec<_> = tree_job
        .runs_below(None)
        .iter()
        .map(|(run, depth)| (run.agent_id.as_str(), *depth))
        .collect();
    assert_eq!(
        shape,
        [
            ("adecision01", 1),
            ("alead01", 1),
            ("aworker01", 2),
            ("ahelper01", 3)
        ]
    );
    let solo = &part.jobs[1];
    assert!(solo.builder_run().is_none(), "the main session builds");
    assert_eq!(solo.runs_below(None)[0].0.agent_id, "asolo01");

    let runs = 1 + part
        .jobs
        .iter()
        .map(|bundle| bundle.runs.len())
        .sum::<usize>();
    assert_eq!(runs, 7, "the master's run and the six of the jobs");
    let questions = tree.questions.len() + tree_job.questions.len();
    assert_eq!(questions, 2);
    assert!(!tree_job.questions[0].is_answered);
    assert!(tree.questions[0].is_answered);

    let worker = &tree_job.runs[3];
    assert_eq!(
        worker.duration_ms,
        Some(180_000),
        "an open run counts up to now"
    );
    assert_eq!(tree_job.job.spent.unwrap().nanodollars, 3_250_000_000);
}

#[test]
fn stale_written_at_marks_the_session_ended() {
    let snapshot = Snapshot::parse(FIXTURE).unwrap();
    assert_eq!(
        snapshot.part(WRITTEN_MS + LIVE_MS).jobs[0].origin,
        Origin::Live
    );
    assert_eq!(
        snapshot.part(WRITTEN_MS + LIVE_MS + 1).jobs[0].origin,
        Origin::Ended
    );
}

#[test]
fn half_written_file_keeps_the_last_good_read() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("53f1c255.json");
    let mut reader = LiveReader::default();
    fs::write(&path, FIXTURE).unwrap();
    assert_eq!(reader.read(dir.path()).unwrap().len(), 1);

    let changed = FIXTURE.replace("\"status\": \"planned\"", "\"status\": \"running\"");
    fs::write(&path, &changed[..changed.len() / 2]).unwrap();
    let kept = reader.read(dir.path()).unwrap();
    assert_eq!(
        kept.len(),
        1,
        "the half-written file gives its last good read"
    );
    assert_eq!(kept[0].jobs[0].status, "planned");

    fs::write(&path, &changed).unwrap();
    assert_eq!(
        reader.read(dir.path()).unwrap()[0].jobs[0].status,
        "running"
    );

    fs::remove_file(&path).unwrap();
    assert!(
        reader.read(dir.path()).unwrap().is_empty(),
        "a removed file is gone"
    );
    assert!(reader.read(&dir.path().join("none")).unwrap().is_empty());
}

#[test]
fn remove_old_removes_only_week_old_snapshots() {
    let dir = tempfile::TempDir::new().unwrap();
    let old = dir.path().join("old.json");
    let new = dir.path().join("new.json");
    let broken = dir.path().join("broken.json");
    fs::write(&old, FIXTURE).unwrap();
    fs::write(
        &new,
        FIXTURE.replace("2026-11-15T20:41:10.000Z", "2026-11-22T20:41:09.000Z"),
    )
    .unwrap();
    fs::write(&broken, "{").unwrap();
    let other_version = dir.path().join("newer-mod.json");
    fs::write(
        &other_version,
        FIXTURE.replace("\"version\": 1,", "\"version\": 2,"),
    )
    .unwrap();
    let not_a_snapshot = dir.path().join("notes.txt");
    fs::write(&not_a_snapshot, FIXTURE).unwrap();

    remove_old(dir.path(), WRITTEN_MS + KEEP_MS + 1).unwrap();

    assert!(!old.exists());
    assert!(new.exists());
    assert!(broken.exists());
    assert!(
        other_version.exists(),
        "a file of another version is not ours"
    );
    assert!(not_a_snapshot.exists(), "only .json files are snapshots");
}

#[test]
fn a_file_of_another_version_or_not_json_is_not_read() {
    assert!(Snapshot::parse(&FIXTURE.replace("\"version\": 1,", "\"version\": 2,")).is_none());
    let dir = tempfile::TempDir::new().unwrap();
    fs::write(dir.path().join("garbage.json"), "not json at all").unwrap();
    fs::write(dir.path().join("session.json"), FIXTURE).unwrap();

    let snapshots = LiveReader::default().read(dir.path()).unwrap();

    assert_eq!(snapshots.len(), 1, "only the snapshot loads");
}
