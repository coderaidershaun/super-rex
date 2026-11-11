//! A teammate of an agent team stops each time it goes idle, and a message starts it
//! again under the same agent ID: its run is open until it stops again, keeps its
//! first start, and is the same whatever the order of its lines.

use serde_json::Value;
use super_rex::capture::RunState;

use super::fixture::*;

/// How long after its recorded stop BG is woken, and when it stops again.
const WOKEN_AFTER_MS: u64 = 5_000;
const STOPS_AGAIN_AFTER_MS: u64 = 20_000;

#[test]
fn a_woken_teammate_is_open_until_it_stops_again() {
    let mut fixture = Fixture::new();
    fixture.open_job(JOB_A, BEFORE_SESSION).unwrap();
    let recorded_lines = fixture.lines.clone();
    fixture.append(&recorded_lines);
    let recorded = run(&fixture.read(JOB_A, AFTER_SESSION), BG).clone();
    assert_eq!(recorded.state, RunState::Finished);

    let [woken, stops_again] = wake_lines(&fixture);
    fixture.append(&[&woken]);
    let runs = fixture.read(JOB_A, AFTER_SESSION);
    let open = run(&runs, BG);
    assert_eq!(open.state, RunState::Open, "a woken teammate works again");
    assert_eq!(open.stopped_at, None, "an open run has no stop yet");
    assert_eq!(open.duration_ms, None);
    assert_eq!(
        open.started_at, recorded.started_at,
        "the first start stays"
    );

    fixture.append(&[&stops_again]);
    let runs = fixture.read(JOB_A, AFTER_SESSION);
    let finished = run(&runs, BG);
    assert_eq!(finished.state, RunState::Finished);
    assert_eq!(finished.started_at, recorded.started_at);
    let again_at = time_text(&fixture, at_ms(&stops_again));
    assert_eq!(finished.stopped_at.as_deref(), Some(again_at.as_str()));

    // The same lines in the reverse order give the same runs.
    let mut reversed = Fixture::new();
    reversed.open_job(JOB_A, BEFORE_SESSION).unwrap();
    let mut lines = recorded_lines;
    lines.extend([woken, stops_again]);
    lines.reverse();
    reversed.append(&lines);
    assert_eq!(reversed.read(JOB_A, AFTER_SESSION).runs, runs.runs);
}

/// BG's recorded start and stop lines, moved to after its recorded stop: the start
/// of a message that wakes it, and its next stop.
fn wake_lines(fixture: &Fixture) -> [String; 2] {
    let [_launch, start, stop] = <[String; 3]>::try_from(fixture.lines_of(BG)).unwrap();
    let stopped_ms = at_ms(&stop);
    [
        with_at_ms(&start, stopped_ms + WOKEN_AFTER_MS),
        with_at_ms(&stop, stopped_ms + STOPS_AGAIN_AFTER_MS),
    ]
}

fn at_ms(line: &str) -> u64 {
    let line: Value = serde_json::from_str(line).unwrap();
    line["at_ms"].as_u64().unwrap()
}

fn with_at_ms(line: &str, at_ms: u64) -> String {
    let mut line: Value = serde_json::from_str(line).unwrap();
    line["at_ms"] = at_ms.into();
    line.to_string()
}

/// `at_ms` in the store's text form.
fn time_text(fixture: &Fixture, at_ms: u64) -> String {
    fixture
        .store
        .conn()
        .query_row(
            "SELECT strftime('%Y-%m-%dT%H:%M:%fZ', ?1 / 1000.0, 'unixepoch')",
            [i64::try_from(at_ms).unwrap()],
            |row| row.get(0),
        )
        .unwrap()
}
