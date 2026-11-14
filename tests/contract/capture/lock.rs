//! A second writer that holds the store's write lock while an agent finishes: a
//! read that cannot get the lock writes nothing, and the next read gives the full history.

use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use rusqlite::ErrorCode;
use super_rex::capture::{CaptureError, RunState};
use super_rex::store::{Store, StoreError};

use super::fixture::*;

/// Starts a second writer that holds the store's write lock until the sender it
/// gives back is used.
fn hold_write_lock(fixture: &Fixture) -> (mpsc::Sender<()>, thread::JoinHandle<()>) {
    let (locked, is_locked) = mpsc::channel();
    let (release, is_released) = mpsc::channel::<()>();
    let paths = fixture.store.paths().clone();
    let holder = thread::spawn(move || {
        let mut other = Store::open(&paths).unwrap();
        other
            .write(|_| {
                locked.send(()).unwrap();
                is_released.recv().unwrap();
                Ok::<_, StoreError>(())
            })
            .unwrap();
    });
    is_locked.recv().unwrap();
    (release, holder)
}

#[test]
fn held_writer_lock_loses_nothing() {
    let mut fixture = Fixture::new();
    fixture.open_job(JOB_A, BEFORE_SESSION).unwrap();
    fixture.append(&fixture.lines.clone());
    let (release, holder) = hold_write_lock(&fixture);

    // A read that cannot get the lock in time fails and writes nothing. The store
    // waits 10 seconds; this connection is set to wait only a moment.
    fixture
        .store
        .conn()
        .busy_timeout(Duration::from_millis(50))
        .unwrap();
    match fixture.try_read(JOB_A, AFTER_SESSION) {
        Err(CaptureError::Store(StoreError::Sqlite {
            source: rusqlite::Error::SqliteFailure(failure, _),
            ..
        })) if failure.code == ErrorCode::DatabaseBusy => {}
        other => panic!("a read that cannot get the lock must fail, got {other:?}"),
    }
    let written: i64 = fixture
        .store
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM sub_agent_run WHERE tokens_read_at IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(written, 0);

    // With the store's own wait, a read waits for the other writer and then succeeds.
    fixture
        .store
        .conn()
        .busy_timeout(Duration::from_secs(10))
        .unwrap();
    let held_for = Duration::from_millis(300);
    let releaser = thread::spawn(move || {
        thread::sleep(held_for);
        release.send(()).unwrap();
    });
    let asked_at = Instant::now();
    let runs = fixture.read(JOB_A, AFTER_SESSION);
    assert!(
        asked_at.elapsed() >= held_for / 2,
        "the read did not wait for the lock"
    );
    releaser.join().unwrap();
    holder.join().unwrap();

    assert_eq!(runs.runs.len(), 6);
    assert_eq!(runs.finished, 6);
    let plain = run(&runs, PLAIN);
    assert_eq!(plain.state, RunState::Finished);
    assert_eq!(
        plain.duration_ms,
        Some(6_856),
        "PLAIN's start and end were taken in"
    );
}
