//! `rex init --with-storage` on a store of rex 0.7: it migrates the store itself,
//! through the code of `rex migrate`, after a backup; a store it must not migrate
//! on its own is left as it is, with the one command to run and why.

use std::fs;
use std::path::PathBuf;

use rusqlite::Connection;

use super::support::{ONE_STEP_FAILED, Repo, repo, rex_init_failed, rex_init_ok};

/// The store of rex 0.7 with two jobs, as SQL.
const VERSION_ONE: &str = include_str!("../../fixtures/store/v1.sql");

/// A transition whose job is not in the store: version 2 refuses to carry it.
/// SQLite as rusqlite builds it checks references, so the check is turned off
/// to plant the row, as a store damaged by hand or by an old bug would hold it.
const BROKEN_ROW: &str = "PRAGMA foreign_keys = OFF; INSERT INTO job_transition (job_id, from_status, to_status, event, at) VALUES ('0gmissing01', NULL, 'created', 'job create', '2026-11-03T10:00:00.000Z');";

fn store_with(repo: &Repo, sql: &str) -> PathBuf {
    fs::create_dir_all(repo.path().join(".rex")).expect("make .rex");
    let db = repo.path().join(".rex/rex.db");
    let conn = Connection::open(&db).expect("open store");
    conn.execute_batch(sql).expect("write store");
    db
}

fn version_of(db: &PathBuf) -> i64 {
    let conn = Connection::open(db).expect("open store");
    conn.query_row("PRAGMA user_version", [], |row| row.get(0))
        .expect("read version")
}

fn jobs_in(db: &PathBuf) -> i64 {
    let conn = Connection::open(db).expect("open store");
    conn.query_row("SELECT COUNT(*) FROM job", [], |row| row.get(0))
        .expect("count jobs")
}

#[test]
fn a_version_one_store_is_migrated_once_and_kept_as_a_backup() {
    let repo = repo();
    let db = store_with(&repo, VERSION_ONE);

    let printed = rex_init_ok(&repo, &["--with-storage"]);

    assert!(
        printed.stdout.contains(
            "rex init: .rex/rex.db migrated from version 1 to 2 (2 jobs), as rex migrate does; the version 1 store is kept in .rex/rex.db.v1-backup\n"
        ),
        "got: {}",
        printed.stdout
    );
    assert_eq!(version_of(&db), 2);
    assert_eq!(jobs_in(&db), 2);
    let backup = repo.path().join(".rex/rex.db.v1-backup");
    assert_eq!(version_of(&backup), 1);
    assert_eq!(jobs_in(&backup), 2);

    let again = rex_init_ok(&repo, &["--with-storage"]);

    assert!(!again.stdout.contains("migrated"), "got: {}", again.stdout);
    assert!(!again.stdout.contains("rex.db"), "got: {}", again.stdout);
}

#[test]
fn a_store_whose_migration_needs_a_choice_is_left_with_the_command_to_run() {
    let repo = repo();
    let db = store_with(&repo, &format!("{VERSION_ONE}\n{BROKEN_ROW}"));
    let before = fs::read(&db).expect("read store");

    let printed = rex_init_failed(&repo, &["--with-storage"]);

    assert_eq!(
        fs::read(&db).expect("read store"),
        before,
        "the store changed"
    );
    assert_eq!(version_of(&db), 1);
    assert!(
        !repo.path().join(".rex/rex.db.v1-backup").exists(),
        "a backup of a store that was not migrated is left behind"
    );
    assert!(
        printed
            .stdout
            .contains("rex init: pre-commit hooks installed\n")
    );
    assert_eq!(
        printed.stderr,
        format!(
            "rex init: failed: .rex/rex.db is version 1, and 1 row in it names a row that is not there, so rex did not migrate it: carrying the row over or dropping it is your choice, and dropping it loses that record; list it with: sqlite3 .rex/rex.db 'PRAGMA foreign_key_check;' then delete it or restore the store from a backup, and run rex init --with-storage again\n{ONE_STEP_FAILED}"
        )
    );
}
