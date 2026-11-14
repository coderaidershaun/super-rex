//! `rex jobs --json` keeps its fixed shape, which scripts and agents read.

use super::support::{Fixture, JOB_A, JOB_B};

#[test]
fn jobs_json_keeps_its_fixed_shape() {
    let mut fixture = Fixture::new();
    let long = format!("Make the validator reject zero. {}", "x".repeat(200));
    fixture
        .add_job(JOB_A)
        .class("fix", "medium", "high", "go,rust")
        .request(&long)
        .created_at("2026-11-09T16:05:41.330Z")
        .insert();
    fixture
        .add_job(JOB_B)
        .class("feature", "small", "low", "rust")
        .request("Add a --json flag\nto rex status.")
        .created_at("2026-11-10T09:14:03.512Z")
        .status("delivered")
        .insert();
    fixture.add_open_pr(
        JOB_B,
        "/work/widgets",
        "feature/status-json",
        "https://example.test/pr/12",
    );

    let run = fixture.rex(&["jobs", "--json"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(run.stdout_lines().len(), 1);
    let document: serde_json::Value = serde_json::from_str(&run.stdout).unwrap();
    assert_eq!(document["schema"], 2);

    // Newest first; the keys of one job object in their fixed order.
    let jobs = document["jobs"].as_array().unwrap();
    let ids: Vec<_> = jobs.iter().map(|job| job["id"].as_str().unwrap()).collect();
    assert_eq!(ids, [JOB_B, JOB_A]);
    let keys: Vec<_> = jobs[0]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        keys,
        [
            "id",
            "created_at",
            "updated_at",
            "status",
            "stored_status",
            "request",
            "request_truncated",
            "origin",
            "builder_agent_id",
            "tree",
            "work_type",
            "size",
            "consequence",
            "languages",
            "orchestrator",
            "merge_agreed",
            "builder_model",
            "builder_effort",
            "deliveries",
            "runs",
            "tokens",
            "cost",
            "incomplete",
            "has_feedback",
        ]
    );
    let delivered = &jobs[0];
    assert_eq!(delivered["status"], "delivered");
    assert_eq!(delivered["stored_status"], "delivered");
    assert_eq!(delivered["request"], "Add a --json flag\nto rex status.");
    assert_eq!(delivered["languages"], serde_json::json!(["rust"]));
    assert_eq!(delivered["orchestrator"], serde_json::Value::Null);
    assert_eq!(
        delivered["deliveries"],
        serde_json::json!([{
            "repo": "/work/widgets",
            "branch": "feature/status-json",
            "pr": "https://example.test/pr/12",
            "outcome": "pr-open",
        }])
    );
    assert_eq!(
        delivered["runs"],
        serde_json::json!({"open": 0, "finished": 0, "unconfirmed": 0})
    );
    assert_eq!(delivered["tokens"], serde_json::Value::Null);
    assert_eq!(delivered["cost"], serde_json::Value::Null);
    assert_eq!(delivered["has_feedback"], false);
    assert_eq!(delivered["origin"], "composer");
    assert_eq!(delivered["builder_agent_id"], "main");
    assert_eq!(delivered["tree"], serde_json::Value::Null);

    // A long request is cut at 200 characters and says so.
    assert_eq!(jobs[1]["request"].as_str().unwrap().chars().count(), 200);
    assert_eq!(jobs[1]["request_truncated"], true);
    assert_eq!(jobs[1]["languages"], serde_json::json!(["go", "rust"]));

    // No match: an empty list, exit 0. Search ignores ASCII case and needs every term.
    let none = fixture.rex(&["jobs", "--json", "--search", "nothing-like-this"]);
    assert_eq!(
        (none.code, none.stdout.as_str()),
        (0, "{\"schema\":2,\"jobs\":[]}\n")
    );
    let found = fixture.rex(&["jobs", "--json", "--search", "JSON", "--search", "STATUS"]);
    assert!(found.stdout.contains(JOB_B) && !found.stdout.contains(JOB_A));

    // Dates are inclusive, on the date part of created_at.
    let day = fixture.rex(&[
        "jobs",
        "--json",
        "--since",
        "2026-11-10",
        "--until",
        "2026-11-10",
    ]);
    assert!(day.stdout.contains(JOB_B) && !day.stdout.contains(JOB_A));
    let reversed = fixture.rex(&[
        "jobs",
        "--json",
        "--since",
        "2026-11-11",
        "--until",
        "2026-11-10",
    ]);
    assert_eq!(
        (reversed.code, reversed.stdout.as_str()),
        (0, "{\"schema\":2,\"jobs\":[]}\n")
    );
    let bad = fixture.rex(&["jobs", "--since", "10/11/2026"]);
    assert_eq!(bad.code, 2);
    assert_eq!(
        bad.stderr.trim_end(),
        "error: --since needs a date as YYYY-MM-DD, not 10/11/2026"
    );

    // Plain form: one line per job, line breaks flattened, nothing when no match.
    let plain = fixture.rex(&["jobs", "--search", "status"]);
    assert_eq!(
        plain.stdout_lines(),
        [format!(
            "{JOB_B} delivered feature small low rust 2026-11-10T09:14:03.512Z Add a --json flag to rex status."
        )]
    );
    assert_eq!(
        fixture
            .rex(&["jobs", "--search", "nothing-like-this"])
            .stdout,
        ""
    );
}
