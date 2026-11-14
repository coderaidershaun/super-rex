//! `rex ingest` on a real store: a batch is stored once however often it is sent,
//! a stream that knows another secret is refused, the sequence of a session goes
//! on across its streams, a batch that does not fit changes nothing, and one
//! session's question never takes another's row.

use std::process::Output;

use serde_json::{Value, json};

use super::harness::{World, crate_root};

const OPEN_STREAM: &str = "fixtures/shared/ingest/batch-open-stream.json";
const EXAMPLE: &str = "fixtures/shared/ingest/batch-example.json";
const EMPTY_STREAM: &str = "fixtures/shared/ingest/batch-empty-stream.json";
const WRONG_SECRET: &str = "fixtures/shared/ingest/batch-wrong-secret.json";
const QUESTION: &str = "fixtures/shared/ingest/batch-question.json";

fn batch(path: &str) -> Value {
    serde_json::from_slice(&std::fs::read(crate_root().join(path)).unwrap()).unwrap()
}

fn stdout(output: &Output) -> &str {
    assert!(
        output.status.success(),
        "rex ingest failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::str::from_utf8(&output.stdout).unwrap()
}

fn refused(output: &Output, code: i32) -> String {
    assert_eq!(output.status.code(), Some(code), "{output:?}");
    assert!(output.stdout.is_empty(), "nothing on stdout: {output:?}");
    String::from_utf8(output.stderr.clone()).unwrap()
}

fn count(world: &World, sql: &str) -> i64 {
    world
        .store()
        .conn()
        .query_row(sql, [], |row| row.get(0))
        .unwrap()
}

/// The rows of every table the record keeps, so a refused batch can be shown
/// to have changed none of them.
fn all_rows(world: &World) -> i64 {
    count(
        world,
        "SELECT (SELECT COUNT(*) FROM ingest_event) + (SELECT COUNT(*) FROM ingest_stream)
              + (SELECT COUNT(*) FROM job) + (SELECT COUNT(*) FROM job_transition)",
    )
}

// The plugin sends a batch again whenever it cannot tell that the first send
// arrived, so a batch sent twice must be stored once.
#[test]
fn ingest_same_batch_twice_is_one_record() {
    let world = World::new();
    assert_eq!(stdout(&world.ingest_file(OPEN_STREAM)), "last_sequence=0\n");

    assert_eq!(stdout(&world.ingest_file(EXAMPLE)), "last_sequence=4\n");
    assert_eq!(stdout(&world.ingest_file(EXAMPLE)), "last_sequence=4\n");

    assert_eq!(count(&world, "SELECT COUNT(*) FROM ingest_event"), 4);
    assert_eq!(count(&world, "SELECT COUNT(*) FROM job"), 1);
    assert_eq!(count(&world, "SELECT COUNT(*) FROM job_transition"), 1);
    let (origin, merge_agreed, merge_source): (String, bool, String) = world
        .store()
        .conn()
        .query_row(
            "SELECT origin, merge_agreed, merge_source FROM job WHERE id = '0gaq820001'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(
        (origin.as_str(), merge_agreed, merge_source.as_str()),
        ("composer", true, "person-prompt")
    );
}

// A new process of the plugin opens a new stream; its events go on from the
// session's last number, so no number is used twice and none is skipped.
#[test]
fn sequence_continues_after_resume() {
    let world = World::new();
    stdout(&world.ingest_file(OPEN_STREAM));
    stdout(&world.ingest_file(EXAMPLE));

    assert_eq!(
        stdout(&world.ingest_file(EMPTY_STREAM)),
        "last_sequence=4\n"
    );

    let mut next = batch(EMPTY_STREAM);
    next["events"] = json!([{
        "seq": 5, "at": "2026-11-13T20:00:05.000Z", "kind": "job.limit", "agent": "main",
        "job": "0gaq820001", "data": { "limit_nano": 20_000_000_000_u64, "source": "set" }
    }]);
    assert_eq!(
        stdout(&world.ingest(next.to_string().as_bytes())),
        "last_sequence=5\n"
    );

    let stream: String = world
        .store()
        .conn()
        .query_row("SELECT stream FROM ingest_event WHERE seq = 5", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(stream, "bbbbbbbbbbbbbbbb");
    assert_eq!(count(&world, "SELECT limit_nano FROM job"), 20_000_000_000);

    // A batch that skips a number is refused; so is a number sent again with
    // other content.
    let mut gap = next.clone();
    gap["events"][0]["seq"] = json!(7);
    let before = all_rows(&world);
    assert_eq!(
        refused(&world.ingest(gap.to_string().as_bytes()), 2),
        "ingest refused: gap in seq: expected 6, got 7\n"
    );
    let mut other = next;
    other["events"][0]["data"]["limit_nano"] = json!(1);
    assert_eq!(
        refused(&world.ingest(other.to_string().as_bytes()), 2),
        "ingest refused: conflict at seq 5\n"
    );
    assert_eq!(all_rows(&world), before, "a refused batch changes nothing");
}

// The secret keeps a writer that does not know it from adding to a stream the
// plugin opened.
#[test]
fn ingest_with_a_wrong_secret_is_refused() {
    let world = World::new();
    stdout(&world.ingest_file(OPEN_STREAM));
    stdout(&world.ingest_file(EXAMPLE));
    let before = all_rows(&world);

    let stderr = refused(&world.ingest_file(WRONG_SECRET), 3);

    assert_eq!(
        stderr,
        "ingest refused: wrong secret for session 00000000-0000-4000-8000-000000000001 stream aaaaaaaaaaaaaaaa\n"
    );
    assert_eq!(all_rows(&world), before);
}

// A batch that does not fit the schema, or a store that keeps no record, is
// refused before anything is written, with the exit code the plugin acts on.
#[test]
fn ingest_refuses_a_malformed_batch_and_a_repo_without_storage() {
    let world = World::new();
    stdout(&world.ingest_file(OPEN_STREAM));
    let before = all_rows(&world);

    let mut unknown_key = batch(EXAMPLE);
    unknown_key["events"][2]["data"]["colour"] = json!("red");
    let stderr = refused(&world.ingest(unknown_key.to_string().as_bytes()), 2);
    assert!(
        stderr.starts_with("ingest refused: seq 3 job.open: unknown field `colour`"),
        "{stderr}"
    );

    let mut unknown_kind = batch(EXAMPLE);
    unknown_kind["events"][0]["kind"] = json!("session.pause");
    let stderr = refused(&world.ingest(unknown_kind.to_string().as_bytes()), 2);
    assert_eq!(
        stderr,
        "ingest refused: seq 1 session.pause: kind \"session.pause\" is not one rex ingest knows\n"
    );

    assert_eq!(refused(&world.ingest(b"{"), 2).lines().count(), 1);
    assert_eq!(all_rows(&world), before);

    std::fs::write(
        world.repo().join(".rex/config.json"),
        r#"{"storage": "off"}"#,
    )
    .unwrap();
    assert_eq!(
        refused(&world.ingest_file(EXAMPLE), 4),
        "ingest refused: storage is off in .rex/config.json\n"
    );

    std::fs::write(
        world.repo().join(".rex/config.json"),
        r#"{"storage": "on"}"#,
    )
    .unwrap();
    let no_store = world.store_dir().join("missing");
    let output = world
        .rex(&["ingest"])
        .env("REX_STORE", &no_store)
        .write_stdin(std::fs::read(crate_root().join(EXAMPLE)).unwrap())
        .output()
        .unwrap();
    assert_eq!(
        refused(&output, 4),
        format!(
            "ingest refused: no store at {}; run rex init --with-storage\n",
            no_store.display()
        )
    );
}

fn question_row(world: &World, id: &str) -> (String, Option<String>) {
    world
        .store()
        .conn()
        .query_row(
            "SELECT question, answer FROM question WHERE id = ?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap()
}

/// A batch of another session: its first stream, holding `events`.
fn other_session(session: &str, events: Value) -> Value {
    let mut other = batch(QUESTION);
    other["session"] = json!(session);
    other["stream"] = json!("cccccccccccccccc");
    other["events"] = events;
    other
}

// The store keys questions by id alone, and each session counts its questions
// from one, so the id carries the end of its session's id. A question that
// still reaches the store under an id it holds with other content is refused,
// never dropped, and an answer never replaces another answer.
#[test]
fn a_question_of_one_session_never_takes_the_row_of_another() {
    let world = World::new();
    stdout(&world.ingest_file(OPEN_STREAM));
    stdout(&world.ingest_file(EXAMPLE));
    assert_eq!(stdout(&world.ingest_file(QUESTION)), "last_sequence=5\n");
    let asked = batch(QUESTION)["events"][0].clone();

    let mut own = asked.clone();
    own["seq"] = json!(1);
    own["data"]["id"] = json!("q1-000000000002");
    own["data"]["question"] = json!("Merge job 0gaq820001 again?");
    let mut answer = json!({
        "seq": 2, "at": "2026-11-13T20:00:04.000Z", "kind": "question.answer", "agent": "main",
        "job": "0gaq820001", "data": { "id": "q1-000000000002", "option": "Keep the branch", "source": "pane" }
    });
    let second = other_session(
        "00000000-0000-4000-8000-000000000002",
        json!([own, answer.clone()]),
    );
    assert_eq!(
        stdout(&world.ingest(second.to_string().as_bytes())),
        "last_sequence=2\n"
    );
    assert_eq!(count(&world, "SELECT COUNT(*) FROM question"), 2);
    assert_eq!(
        question_row(&world, "q1-000000000001"),
        (
            "Merge job 0gaq820001 (rex/0gaq820001) to main of /home/person/repo?".to_owned(),
            None
        )
    );
    assert_eq!(
        question_row(&world, "q1-000000000002"),
        (
            "Merge job 0gaq820001 again?".to_owned(),
            Some("Keep the branch".to_owned())
        )
    );

    // A resumed tree's session may ask a question again with the same content.
    let mut again = asked.clone();
    again["seq"] = json!(3);
    let resent = other_session("00000000-0000-4000-8000-000000000002", json!([again]));
    assert_eq!(
        stdout(&world.ingest(resent.to_string().as_bytes())),
        "last_sequence=3\n"
    );

    let before = count(&world, "SELECT COUNT(*) FROM ingest_event");
    let mut clash = asked;
    clash["seq"] = json!(1);
    clash["data"]["question"] = json!("Which parser?");
    let third = other_session("00000000-0000-4000-8000-000000000003", json!([clash]));
    assert_eq!(
        refused(&world.ingest(third.to_string().as_bytes()), 2),
        "ingest refused: seq 1 question.ask: question q1-000000000001 is in the store with other content\n"
    );

    answer["seq"] = json!(4);
    answer["data"]["option"] = json!("Merge");
    let replace = other_session("00000000-0000-4000-8000-000000000002", json!([answer]));
    assert_eq!(
        refused(&world.ingest(replace.to_string().as_bytes()), 2),
        "ingest refused: seq 4 question.answer: question q1-000000000002 is answered Keep the branch already, got Merge\n"
    );
    assert_eq!(count(&world, "SELECT COUNT(*) FROM ingest_event"), before);
    assert_eq!(
        question_row(&world, "q1-000000000002").1.as_deref(),
        Some("Keep the branch")
    );
}
