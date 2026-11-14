//! `rex migrate` on real store files: a store of rex 0.7 moves to version 2 with
//! every job kept, a second run changes nothing, and a store of a version no
//! rex knows is refused with not a byte written.

use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde_json::Value;

use super::fixture::{rex, stderr_of, stdout_of};

/// The store of rex 0.7 with two jobs, as SQL.
const VERSION_ONE: &str = include_str!("../../fixtures/store/v1.sql");

/// A Git repo with a `.rex/rex.db` made by `sql`, and the path of that file.
fn repo_with_store(sql: &str) -> (tempfile::TempDir, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir(temp.path().join(".git")).unwrap();
    fs::create_dir(temp.path().join(".rex")).unwrap();
    let db = temp.path().join(".rex/rex.db");
    let conn = Connection::open(&db).unwrap();
    conn.execute_batch(sql).unwrap();
    drop(conn);
    (temp, db)
}

fn json_of(cwd: &Path, args: &[&str]) -> Value {
    serde_json::from_str(&stdout_of(&mut rex(cwd, args))).unwrap()
}

#[test]
fn migrate_of_a_v1_fixture_store_keeps_each_job() {
    let (temp, _db) = repo_with_store(VERSION_ONE);
    let repo = temp.path();

    // Every other command refuses the old store and names the fix.
    let refused = stderr_of(&mut rex(repo, &["jobs", "--json"]), 2);
    assert_eq!(refused, "error: store is version 1; run rex migrate\n");

    assert_eq!(
        stdout_of(&mut rex(repo, &["migrate"])),
        "store=v2 migrated=yes jobs=2\n"
    );
    assert_eq!(
        stdout_of(&mut rex(repo, &["migrate"])),
        "store=v2 migrated=no\n"
    );

    let jobs = json_of(repo, &["jobs", "--json"]);
    let listed: Vec<(&str, &str, &str)> = jobs["jobs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|job| {
            (
                job["id"].as_str().unwrap(),
                job["origin"].as_str().unwrap(),
                job["status"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        listed,
        [
            ("0g4abc0002", "sdk", "created"),
            ("0g4abc0001", "composer", "delivered")
        ]
    );

    let shown = json_of(repo, &["job", "show", "0g4abc0001", "--json"]);
    assert_eq!(shown["job"]["builder_agent_id"], "main");
    assert_eq!(shown["job"]["merge"]["answer"], "yes");
    assert_eq!(shown["deliveries"][0]["merge_commit"], "c0ffee");
    assert_eq!(shown["roles"][0]["agent"], "model-high-effort-high");
    assert_eq!(shown["runs"][0]["agent_id"], "a1");
    assert_eq!(shown["feedback"][0]["verdict"], "accepted");
    assert_eq!(shown["transitions"].as_array().unwrap().len(), 3);

    // A limit a person set moves from the meta table to the job.
    let limit = json_of(repo, &["limit", "0g4abc0002", "--json"]);
    assert_eq!(limit["limit_usd"], 15.0);
    assert_eq!(limit["source"], "set");
}

// A store that no version of rex wrote may hold anything; a migration that
// guessed would damage it beyond repair, so it is refused with not a byte written.
#[test]
fn migrate_refuses_unknown_version_and_writes_nothing() {
    let (temp, db) = repo_with_store("CREATE TABLE notes (text TEXT); PRAGMA user_version = 3;");
    let before = fs::read(&db).unwrap();

    let stderr = stderr_of(&mut rex(temp.path(), &["migrate"]), 2);

    assert_eq!(stderr, "migrate refused: store version 3 is not 1 or 2\n");
    assert_eq!(
        fs::read(&db).unwrap(),
        before,
        "the store file is unchanged"
    );
    let names: Vec<String> = fs::read_dir(temp.path().join(".rex"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["rex.db"], "no file was added beside the store");
}
