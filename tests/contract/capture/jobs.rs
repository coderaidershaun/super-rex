//! A run belongs to the job its session had open when the run started, and keeps
//! that job whatever arrives later. Capture never changes a job's status.

use rusqlite::ErrorCode;
use super_rex::capture::{CaptureError, RunState, take_in};
use super_rex::store::StoreError;

use super::fixture::*;

#[test]
fn builder_in_a_sub_agent_keeps_the_open_job() {
    let mut fixture = Fixture::new();
    fixture.open_job(JOB_A, BEFORE_SESSION).unwrap();

    // LEAD's own `rex job create` finds the session's job open: no second job opens.
    match fixture.open_job(JOB_B, "2026-11-10T13:10:20.000Z") {
        Err(StoreError::Sqlite {
            source: rusqlite::Error::SqliteFailure(failure, _),
            ..
        }) if failure.code == ErrorCode::ConstraintViolation => {}
        other => panic!("a second open job for the session must be refused, got {other:?}"),
    }
    fixture.append(&fixture.lines.clone());

    let runs = fixture.read(JOB_A, AFTER_SESSION);
    assert_eq!(run(&runs, WORKER).parent_agent_id.as_deref(), Some(LEAD));
    assert_eq!(run(&runs, BG_CHILD).parent_agent_id.as_deref(), Some(BG));
    for child in [WORKER, BG_CHILD] {
        assert_eq!(
            run(&runs, child).depth,
            Some(2),
            "{child} is tied to the open job"
        );
    }
    assert!(matches!(
        fixture.try_read(JOB_B, AFTER_SESSION),
        Err(CaptureError::Store(StoreError::JobNotFound { .. }))
    ));
}

#[test]
fn late_lines_stay_with_the_first_job() {
    // PLAIN starts in job A; A closes; B opens; then PLAIN's stop line and call come.
    let mut fixture = Fixture::new();
    let [_, stop, call] = plain_lines(&fixture);
    close_a_and_open_b(&mut fixture, &[&stop, &call]);
    fixture.append(&[&stop, &call]);

    let in_a = fixture.read(JOB_A, AFTER_SESSION);
    assert_eq!(run(&in_a, PLAIN).state, RunState::Finished);
    assert_eq!(
        run(&in_a, PLAIN).started_at.as_deref(),
        Some("2026-11-10T13:10:35.345Z")
    );
    assert_eq!(fixture.read(JOB_B, AFTER_SESSION).runs, []);

    // The same with PLAIN's start line lost: its call comes first, so its tie time is
    // the call's time less the run's duration, which is before the close.
    let mut fixture = Fixture::new();
    let [start, stop, call] = plain_lines(&fixture);
    close_a_and_open_b(&mut fixture, &[&start, &stop, &call]);
    fixture.append(&[&call, &stop]);

    let in_a = fixture.read(JOB_A, AFTER_SESSION);
    assert_eq!(
        run(&in_a, PLAIN).started_at.as_deref(),
        Some("2026-11-10T13:10:35.343Z")
    );
    assert_eq!(fixture.read(JOB_B, AFTER_SESSION).runs, []);
}

/// PLAIN's start line, stop line and completed call, as `fixture` wrote them.
fn plain_lines(fixture: &Fixture) -> [String; 3] {
    fixture.lines_of(PLAIN).try_into().unwrap()
}

/// Takes in every recorded line but `held_back` while job A is open, then closes A
/// after PLAIN started and before it stopped, and opens B.
fn close_a_and_open_b(fixture: &mut Fixture, held_back: &[&String]) {
    let close = "2026-11-10T13:10:40.000Z";
    fixture.open_job(JOB_A, BEFORE_SESSION).unwrap();
    let early: Vec<&String> = fixture
        .lines
        .iter()
        .filter(|line| !held_back.contains(line))
        .collect();
    fixture.append(&early);
    take_in(&mut fixture.store).unwrap();
    fixture.close_job(JOB_A, close);
    fixture.open_job(JOB_B, close).unwrap();
}

#[test]
fn run_before_the_plan_leaves_the_status() {
    let mut fixture = Fixture::new();
    fixture.open_job(JOB_A, BEFORE_SESSION).unwrap();
    let before = fixture.status(JOB_A);
    assert_eq!(before, ("created".to_owned(), 1));
    fixture.append(&fixture.lines.clone());
    // The builder ran `rex job create` in the main session: one more record of a
    // request already in the transcript, and the call's result.
    let create = [
        r#"{"type":"assistant","timestamp":"2026-11-10T13:10:14.878Z","effort":"low","message":{"id":"msg_fixture0017","model":"claude-sonnet-5-5","content":[{"type":"tool_use","id":"toolu_create","name":"Bash","input":{"command":"rex job create"}}]}}"#,
        r#"{"type":"user","timestamp":"2026-11-10T13:10:14.900Z","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"toolu_create","content":"job=k3x9p2m7qd\nexisting=no"}]}}"#,
    ];
    append_text(
        &fixture.main_transcript(),
        &format!("{}\n", create.join("\n")),
    );

    let runs = fixture.read(JOB_A, AFTER_SESSION);
    assert_eq!(runs.runs.len(), 6);
    assert_eq!(fixture.status(JOB_A), before);
    let builder = (Some("claude-sonnet-5-5".to_owned()), Some("low".to_owned()));
    assert_eq!(fixture.builder(JOB_A), builder);
}
