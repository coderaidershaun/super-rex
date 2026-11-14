//! `rex doctor` on a store the plugin wrote: it names a plugin of another
//! version than the binary, and an open job that a newer session left behind.

use serde_json::json;

use super::harness::World;

// A plugin of another version reads keys the binary does not print, and a job
// left open by a session that ended keeps its owner from opening another:
// neither shows anywhere else, so doctor must name both.
#[test]
fn doctor_reports_version_skew_and_an_open_orphan() {
    let world = World::new();
    for path in [
        "fixtures/shared/ingest/batch-open-stream.json",
        "fixtures/shared/ingest/batch-example.json",
    ] {
        assert!(world.ingest_file(path).status.success());
    }
    let binary = env!("CARGO_PKG_VERSION");
    let later_session = json!({
        "version": 1, "session": "00000000-0000-4000-8000-000000000002",
        "stream": "dddddddddddddddd", "secret": "4".repeat(64),
        "events": [{
            "seq": 1, "at": "2026-11-14T09:00:00.000Z", "kind": "session.start", "agent": "main",
            "job": null, "data": { "claude_code": "2.1.400", "plugin": binary, "storage": "on" }
        }]
    });
    assert!(
        world
            .ingest(later_session.to_string().as_bytes())
            .status
            .success()
    );

    let output = world.rex(&["doctor"]).output().unwrap();

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        stdout,
        format!(
            "doctor=problems\nbinary={binary}\nstore=v2\nstorage=on\nclaude=2.1.400\nplugin={binary}\n\
             problem=open orphan: job 0gaq820001 is created, but its session 00000000-0000-4000-8000-000000000001 is not the newest; close it, or resume it with /rex master resume\n"
        )
    );

    // The first session alone: no orphan, but its plugin is not this binary.
    let first = World::new();
    for path in [
        "fixtures/shared/ingest/batch-open-stream.json",
        "fixtures/shared/ingest/batch-example.json",
    ] {
        assert!(first.ingest_file(path).status.success());
    }
    let stdout = String::from_utf8(first.rex(&["doctor"]).output().unwrap().stdout).unwrap();
    assert!(
        stdout.ends_with(&format!(
            "plugin=0.8.0\nproblem=version skew: plugin 0.8.0, binary {binary}; install the same version of both\n"
        )) || binary == "0.8.0",
        "{stdout}"
    );
}
