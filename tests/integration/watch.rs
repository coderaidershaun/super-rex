//! `rex watch --once` run as a person or an agent runs it: from live snapshots
//! alone, from the store alone, from neither, and with a week-old snapshot.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as Process;

use super::store_jobs::fixture::{
    SESSION, event, ingest, job_open, new_repo, new_store, new_worktree, rex, stderr_of, stdout_of,
    storage_on,
};

const FIXTURE: &str = include_str!("../../fixtures/shared/live/snapshot.json");
const FIXTURE_WRITTEN_AT: &str = "2026-11-15T20:41:10.000Z";

/// The time `seconds_ago` before now, in the form of a snapshot.
fn time_ago(seconds_ago: u64) -> String {
    let output = Process::new("python3")
        .args([
            "-I",
            "-c",
            &format!(
                "import datetime as d; t = d.datetime.now(d.timezone.utc) - d.timedelta(seconds={seconds_ago}); \
                 print(t.strftime('%Y-%m-%dT%H:%M:%S.000Z'))"
            ),
        ])
        .output()
        .unwrap();
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

/// Writes the shared fixture as the snapshot of one session, written `seconds_ago`.
fn write_snapshot(repo: &Path, seconds_ago: u64) -> PathBuf {
    let dir = repo.join(".rex/live");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("53f1c255-4567-4e00-ab35-553d90ce69d1.json");
    fs::write(
        &path,
        FIXTURE.replace(FIXTURE_WRITTEN_AT, &time_ago(seconds_ago)),
    )
    .unwrap();
    path
}

#[test]
fn once_with_only_a_snapshot_prints_all_four_levels_and_the_question() {
    let temp = tempfile::tempdir().unwrap();
    let repo = new_repo(temp.path(), "shop");
    write_snapshot(&repo, 0);
    let sub_folder = repo.join("src");
    fs::create_dir_all(&sub_folder).unwrap();

    let text = stdout_of(&mut rex(&sub_folder, &["watch", "--once"]));

    assert!(
        text.starts_with("rex watch  source live  ·  1 tree  ·  2 jobs  ·  1 question waiting\n"),
        "{text}"
    );
    for line in [
        "├─ master t0geh4lh3i  running  live  opus-5-5",
        "│  └─ builder 0geh5fafkb  planned  live",
        "│     ├─ ? q1-553d90ce69d1  Merge job 0geh5fafkb (rex/0geh5fafkb) to main of /home/ana/code/shop?  \
         options 1 Merge (recommended)  2 Keep the branch  answer with /rex answer q1-553d90ce69d1 <option number>",
        "│     ├─ lead decision-maker  done  opus-5-5  1m38s  $0.700  hit 90%",
        "│     └─ lead engineer-lead  open",
        "│        └─ worker engineer T1  open  sonnet-5-5",
        "│           └─ helper model-low-effort-low  done  haiku-5-5  41s  $0.100",
        "└─ builder 0gegkccde2  delivered  live",
        "   └─ lead engineer T1  done  haiku-5-5  44s  $0.200",
    ] {
        assert!(
            text.lines().any(|shown| shown.starts_with(line)),
            "no line {line:?} in:\n{text}"
        );
    }
    assert!(
        !text.contains("q2-553d90ce69d1"),
        "an answered question does not wait"
    );
}

#[test]
fn once_from_a_job_worktree_reads_the_main_checkouts_snapshots() {
    let temp = tempfile::tempdir().unwrap();
    let repo = new_repo(temp.path(), "shop");
    write_snapshot(&repo, 0);
    let worktree = new_worktree(&repo, temp.path(), "0geh5fafkb");

    let text = stdout_of(&mut rex(&worktree, &["watch", "--once"]));

    assert!(
        text.starts_with("rex watch  source live  ·  1 tree  ·  2 jobs"),
        "{text}"
    );
}

#[test]
fn once_marks_a_stale_session_ended() {
    let temp = tempfile::tempdir().unwrap();
    let repo = new_repo(temp.path(), "shop");
    write_snapshot(&repo, 120);

    let text = stdout_of(&mut rex(&repo, &["watch", "--once"]));

    assert!(
        text.contains("├─ master t0geh4lh3i  running  ended"),
        "{text}"
    );
}

#[test]
fn once_with_only_a_store_prints_its_jobs() {
    let temp = tempfile::tempdir().unwrap();
    let repo = new_repo(temp.path(), "shop");
    new_store(&repo, "pipeline");
    storage_on(&repo);
    ingest(
        &repo,
        SESSION,
        1,
        &[event(
            "job.open",
            "2026-11-13T20:00:02.000Z",
            Some("0gaq820001"),
            job_open(SESSION, "main", &repo, "0gaq820001"),
        )],
    );

    let text = stdout_of(&mut rex(&repo, &["watch", "--once"]));

    assert!(
        text.starts_with("rex watch  source store  ·  0 trees  ·  1 job"),
        "{text}"
    );
    assert!(
        text.contains("└─ builder 0gaq820001  created  store"),
        "{text}"
    );
    assert!(text.contains("A request."), "{text}");
}

#[test]
fn once_with_neither_says_no_job_has_run() {
    let temp = tempfile::tempdir().unwrap();
    let repo = new_repo(temp.path(), "shop");

    let text = stdout_of(&mut rex(&repo, &["watch", "--once"]));

    assert!(text.contains("No job has run here yet."), "{text}");
}

#[test]
fn a_week_old_snapshot_is_removed() {
    let temp = tempfile::tempdir().unwrap();
    let repo = new_repo(temp.path(), "shop");
    let old = write_snapshot(&repo, 8 * 24 * 3_600);

    let text = stdout_of(&mut rex(&repo, &["watch", "--once"]));

    assert!(!old.exists(), "the old snapshot is still there");
    assert!(text.contains("No job has run here yet."), "{text}");
}

#[test]
fn without_once_and_without_a_terminal_it_exits_1() {
    let temp = tempfile::tempdir().unwrap();
    let repo = new_repo(temp.path(), "shop");

    let stderr = stderr_of(&mut rex(&repo, &["watch"]), 1);

    assert!(stderr.contains("rex watch needs a terminal"), "{stderr}");
}
