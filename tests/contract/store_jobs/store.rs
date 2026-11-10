//! What the store promises to the lanes that call it: where it is found, that a
//! second writer waits, and that schema version 1 guards its own rules.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use rusqlite::{ErrorCode, Transaction, params};
use super_rex::store::{JobId, Place, Store, StoreError, StorePaths};

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(["-c", "user.name=rex", "-c", "user.email=rex@example.com"])
        .args([
            "-c",
            "commit.gpgsign=false",
            "-c",
            "init.defaultBranch=main",
        ])
        .args(args)
        .current_dir(dir)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_INDEX_FILE")
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?} failed in {}", dir.display());
}

fn new_repo(parent: &Path, name: &str) -> PathBuf {
    let repo = parent.join(name);
    fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q"]);
    git(
        &repo,
        &[
            "commit",
            "-q",
            "--allow-empty",
            "--no-verify",
            "-m",
            "start",
        ],
    );
    repo.canonicalize().unwrap()
}

fn new_store(repo: &Path) -> StorePaths {
    let paths = StorePaths::locate(repo).unwrap();
    Store::create(&paths).unwrap();
    paths
}

fn found_root(cwd: &Path, project_dir: Option<&Path>) -> PathBuf {
    let project_dir = project_dir.map(|dir| dir.as_os_str().to_owned());
    let paths = StorePaths::discover_from(None, project_dir, cwd).unwrap();
    assert_eq!(paths.db, paths.root.join(".rex").join("rex.db"));
    paths.root.canonicalize().unwrap()
}

#[test]
fn finds_main_store_from_worktree_and_subfolder() {
    let temp = tempfile::tempdir().unwrap();
    let main = new_repo(temp.path(), "main");
    new_store(&main);
    let sub_folder = main.join("src").join("deep");
    fs::create_dir_all(&sub_folder).unwrap();
    let worktree = temp.path().join("worktree");
    git(
        &main,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "job",
            worktree.to_str().unwrap(),
        ],
    );
    let worktree_sub_folder = worktree.join("src");
    fs::create_dir_all(&worktree_sub_folder).unwrap();

    assert_eq!(found_root(&main, None), main);
    assert_eq!(found_root(&sub_folder, None), main);
    assert_eq!(found_root(&worktree, None), main);
    assert_eq!(found_root(&worktree_sub_folder, None), main);

    let sibling = new_repo(temp.path(), "sibling");
    new_store(&sibling);
    assert_eq!(found_root(&sibling, None), sibling);
    assert_eq!(found_root(&sibling, Some(&main)), main);

    let relative = StorePaths::discover_from(Some("main/.rex".into()), None, temp.path()).unwrap();
    assert_eq!(
        relative.dir,
        main.join(".rex"),
        "a relative REX_STORE gives absolute paths"
    );
    assert_eq!(relative.root, main);

    let not_a_store = temp.path().join("not-a-store");
    fs::create_dir_all(&not_a_store).unwrap();
    let result = StorePaths::discover_from(
        Some(not_a_store.clone().into_os_string()),
        Some(main.clone().into_os_string()),
        &main,
    );
    let Err(error) = result else {
        panic!("REX_STORE at a folder with no rex.db must fail, not fall through");
    };
    let message = error.to_string();
    let StoreError::NotFound { tried } = error else {
        panic!("expected NotFound, got {message}");
    };
    assert_eq!(tried.len(), 1, "REX_STORE never falls through: {message}");
    assert_eq!(tried[0].place, Place::RexStore);
    assert!(message.starts_with("REX_STORE is set to"), "{message}");
}

type Snapshot = BTreeMap<PathBuf, (Vec<u8>, SystemTime)>;

fn snapshot_files(dir: &Path) -> Snapshot {
    let mut files = Snapshot::new();
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(snapshot_files(&path));
        } else {
            let modified = fs::metadata(&path).unwrap().modified().unwrap();
            files.insert(path.clone(), (fs::read(&path).unwrap(), modified));
        }
    }
    files
}

fn insert_job(
    tx: &Transaction<'_>,
    id: &str,
    session: &str,
    status: &str,
) -> Result<(), StoreError> {
    tx.execute(
        "INSERT INTO job (id, request, asked_by, class_version, work_type, size, consequence,
                          languages, merge_agreed, session_id, status, created_at, updated_at)
         VALUES (?1, 'fix the parser', 'human', 1, 'fix', 'small', 'low', 'rust', 0, ?2, ?3,
                 strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
        params![id, session, status],
    )?;
    Ok(())
}

fn upsert_run(tx: &Transaction<'_>, job: &str, effort: &str) -> Result<(), StoreError> {
    tx.execute(
        "INSERT INTO sub_agent_run (job_id, session_id, agent_id, first_seen_at, stopped_at, effort_ran)
         VALUES (?1, 's1', 'agent-1', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                 strftime('%Y-%m-%dT%H:%M:%fZ', NULL), ?2)
         ON CONFLICT(session_id, agent_id) DO UPDATE SET
             stopped_at = COALESCE(excluded.stopped_at, stopped_at),
             effort_ran = COALESCE(excluded.effort_ran, effort_ran)",
        params![job, effort],
    )?;
    Ok(())
}

fn assert_refused(result: Result<(), StoreError>, what: &str) {
    match result {
        Err(StoreError::Sqlite {
            source: rusqlite::Error::SqliteFailure(failure, _),
            ..
        }) if failure.code == ErrorCode::ConstraintViolation => {}
        other => panic!("the schema must refuse {what}, got {other:?}"),
    }
}

fn count(store: &Store, sql: &str) -> i64 {
    store.conn().query_row(sql, [], |row| row.get(0)).unwrap()
}

fn paths_in(dir: &Path) -> StorePaths {
    fs::create_dir_all(dir).unwrap();
    StorePaths {
        root: dir.parent().unwrap().to_path_buf(),
        dir: dir.to_path_buf(),
        db: dir.join("rex.db"),
        event_log: dir.join("events.jsonl"),
        orchestrators: dir.join("orchestrators"),
    }
}

fn insert_meta(tx: &Transaction<'_>, key: &str) -> Result<(), StoreError> {
    tx.execute("INSERT INTO meta (key, value) VALUES (?1, 'x')", [key])?;
    Ok(())
}

#[test]
fn second_writer_waits_instead_of_failing() {
    let temp = tempfile::tempdir().unwrap();
    let repo = new_repo(temp.path(), "repo");
    let paths = new_store(&repo);
    let mut first = Store::open(&paths).unwrap();
    let mut second = Store::open(&paths).unwrap();
    let hold = Duration::from_millis(300);

    let (lock_held, wait_for_lock) = mpsc::channel();
    let holder = thread::spawn(move || {
        first.write(|tx| {
            insert_meta(tx, "test.first")?;
            lock_held.send(()).unwrap();
            thread::sleep(hold);
            Ok::<(), StoreError>(())
        })
    });
    wait_for_lock.recv().unwrap();

    let started = Instant::now();
    // Read before the write, as `rex job create` does: a transaction that only
    // takes the write lock later would fail at once instead of waiting.
    second
        .write(|tx| {
            let _: i64 = tx.query_row("SELECT count(*) FROM meta", [], |row| row.get(0))?;
            insert_meta(tx, "test.second")
        })
        .unwrap();
    let waited = started.elapsed();
    holder.join().unwrap().unwrap();

    assert!(
        waited >= hold / 2,
        "the second writer did not wait for the first: {waited:?}"
    );
    assert_eq!(
        count(&second, "SELECT count(*) FROM meta WHERE key LIKE 'test.%'"),
        2
    );
}

#[test]
fn schema_v1_applies_once_and_guards_its_invariants() {
    let temp = tempfile::tempdir().unwrap();
    let repo = temp.path().join("repo");
    fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q"]);
    let paths = StorePaths::locate(&repo).unwrap();

    let store = Store::create(&paths).unwrap();
    let version: i64 = store
        .conn()
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(version, 1);
    let tables = count(
        &store,
        "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name IN
         ('job', 'job_transition', 'role', 'delivery', 'orchestrator', 'sub_agent_run',
          'session_tokens', 'feedback', 'meta')",
    );
    assert_eq!(tables, 9);
    assert!(paths.orchestrators.is_dir());
    drop(store);

    let before = snapshot_files(&paths.dir);
    drop(Store::create(&paths).unwrap());
    drop(Store::open(&paths).unwrap());
    assert_eq!(
        snapshot_files(&paths.dir),
        before,
        "a store at version 1 must be left as it was"
    );
    for leftover in ["rex.db-wal", "rex.db-shm"] {
        assert!(
            !paths.dir.join(leftover).exists(),
            "{leftover} left after the store closed"
        );
    }

    let mut store = Store::open(&paths).unwrap();
    assert_refused(
        store.write(|tx| insert_job(tx, "job0000001", "s1", "running")),
        "a stored running",
    );

    store
        .write(|tx| insert_job(tx, "job0000001", "s1", "created"))
        .unwrap();
    let second_open_job = store.write(|tx| {
        insert_job(tx, "job0000003", "s2", "created")?;
        insert_job(tx, "job0000002", "s1", "planned")
    });
    assert_refused(second_open_job, "a second open job for one session");
    assert_eq!(
        count(&store, "SELECT count(*) FROM job WHERE id = 'job0000003'"),
        0,
        "a failed write must roll back"
    );
    let open_job = store.open_job_for_session("s1").unwrap();
    assert_eq!(open_job, Some("job0000001".parse::<JobId>().unwrap()));

    store
        .write(|tx| insert_job(tx, "job0000002", "s1", "delivered"))
        .unwrap();
    store
        .write(|tx| upsert_run(tx, "job0000001", "high"))
        .unwrap();
    assert_refused(
        store.write(|tx| {
            tx.execute("UPDATE sub_agent_run SET job_id = 'job0000002'", [])?;
            Ok(())
        }),
        "an update of a run's job",
    );
    assert_refused(
        store.write(|tx| {
            tx.execute(
                "INSERT OR REPLACE INTO sub_agent_run (job_id, session_id, agent_id, first_seen_at)
                 VALUES ('job0000002', 's1', 'agent-1', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
                [],
            )?;
            Ok(())
        }),
        "a replace of a run under another job",
    );

    store
        .write(|tx| upsert_run(tx, "job0000002", "max"))
        .unwrap();
    let (job, effort): (String, String) = store
        .conn()
        .query_row("SELECT job_id, effort_ran FROM sub_agent_run", [], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .unwrap();
    assert_eq!(
        (job.as_str(), effort.as_str()),
        ("job0000001", "max"),
        "the upsert keeps the first job"
    );
    drop(store);

    let empty = paths_in(&temp.path().join("empty").join(".rex"));
    fs::write(&empty.db, b"").unwrap();
    assert!(matches!(
        Store::open(&empty),
        Err(StoreError::NotInitialised { .. })
    ));

    let newer = paths_in(&temp.path().join("newer").join(".rex"));
    rusqlite::Connection::open(&newer.db)
        .unwrap()
        .pragma_update(None, "user_version", 2)
        .unwrap();
    for result in [Store::open(&newer), Store::create(&newer)] {
        assert!(matches!(
            result,
            Err(StoreError::NewerSchema {
                found: 2,
                supported: 1,
                ..
            })
        ));
    }
}
