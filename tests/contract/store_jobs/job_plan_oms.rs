//! What `rex ingest` promises about the selection function when it takes in a
//! plan: each role's stored record is exactly what the function returns for the
//! job's and the role's inputs, so the record and the function never disagree.

use rusqlite::Row;
use serde_json::json;
use super_rex::oms::{self, Inputs, Level};

use crate::calls::harness::World;

fn level(row: &Row<'_>, column: &str) -> Option<Level> {
    let text: Option<String> = row.get(column).unwrap();
    match text.as_deref() {
        None => None,
        Some("low") => Some(Level::Low),
        Some("medium") => Some(Level::Medium),
        Some("high") => Some(Level::High),
        Some(other) => panic!("{column} holds {other}"),
    }
}

/// What a role row holds after the plan.
#[derive(Debug, PartialEq, Eq)]
struct Record {
    agent: Option<String>,
    demand: Option<String>,
    rules: Option<String>,
    conflict: Option<String>,
    oms_version: Option<u32>,
}

#[test]
fn ingest_of_a_plan_records_what_oms_returns() {
    let world = World::new();
    let open_stream = world.ingest_file("fixtures/shared/ingest/batch-open-stream.json");
    assert!(open_stream.status.success());
    let job = "0gaq820002";
    let role = |name: &str, ratings: [&str; 4]| {
        json!({ "role": name, "intelligence": ratings[0], "reliability": ratings[1],
                "consequence": ratings[2], "verification": ratings[3],
                "grid_agent": "model-medium-effort-medium", "conflict": null, "oms_version": 2 })
    };
    let batch = json!({
        "version": 1, "session": "00000000-0000-4000-8000-000000000001",
        "stream": "aaaaaaaaaaaaaaaa", "secret": "1".repeat(64),
        "events": [
            { "seq": 1, "at": "2026-11-13T20:00:02.000Z", "kind": "job.open", "agent": "main", "job": job,
              "data": { "owner": { "session": "00000000-0000-4000-8000-000000000001", "builder": "main" },
                        "origin": "composer", "request": "Add a search page.", "restatement": null,
                        "repo": "/r", "branch": "rex/0gaq820002", "base": "main", "worktree": null,
                        "type": "feature", "size": "large", "consequence": "high", "langs": ["rust"],
                        "prefs": { "duration": "high", "budget": "low", "autonomy": "high" },
                        "limit_nano": 1, "limit_source": "default", "tree": null,
                        "depends_on": [], "merge_after": [] } },
            { "seq": 2, "at": "2026-11-13T20:01:00.000Z", "kind": "job.plan", "agent": "main", "job": job,
              "data": { "orchestrator": "pipeline", "breakdown": "Build it.", "tasks": [], "manifest": [],
                        "roles": [ role("architect", ["high", "high", "high", "high"]),
                                   role("engineer", ["medium", "none", "low", "low"]),
                                   role("scout", ["none", "none", "none", "none"]) ] } }
        ]
    });
    let output = world.ingest(batch.to_string().as_bytes());
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let store = world.store();
    let mut statement = store
        .conn()
        .prepare("SELECT * FROM role WHERE job_id = ?1 ORDER BY rowid")
        .unwrap();
    let rows: Vec<(String, Inputs, Record)> = statement
        .query_map([job], |row| {
            let inputs = Inputs {
                duration: level(row, "in_duration"),
                budget: level(row, "in_budget"),
                intelligence: level(row, "in_intelligence"),
                autonomy: level(row, "in_autonomy"),
                reliability: level(row, "in_reliability"),
                consequence: level(row, "in_consequence"),
                verification: level(row, "in_verification"),
            };
            let record = Record {
                agent: row.get("agent")?,
                demand: row.get("demand")?,
                rules: row.get("rules")?,
                conflict: row.get("conflict")?,
                oms_version: row.get("oms_version")?,
            };
            Ok((row.get("role")?, inputs, record))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();

    let names: Vec<&str> = rows.iter().map(|(name, ..)| name.as_str()).collect();
    assert_eq!(names, ["architect", "engineer", "scout"]);
    let job_inputs = Inputs {
        duration: Some(Level::High),
        budget: Some(Level::Low),
        autonomy: Some(Level::High),
        ..Inputs::default()
    };
    let role_inputs = [
        (
            Some(Level::High),
            Some(Level::High),
            Some(Level::High),
            Some(Level::High),
        ),
        (
            Some(Level::Medium),
            None,
            Some(Level::Low),
            Some(Level::Low),
        ),
        (None, None, None, None),
    ];
    for ((name, inputs, record), role) in rows.iter().zip(role_inputs) {
        let (intelligence, reliability, consequence, verification) = role;
        let expected_inputs = Inputs {
            intelligence,
            reliability,
            consequence,
            verification,
            ..job_inputs
        };
        assert_eq!(*inputs, expected_inputs, "{name}");

        let selection = oms::select(expected_inputs);
        let rules: Vec<&str> = selection.rules.iter().map(|rule| rule.id()).collect();
        let conflict: Vec<&str> = selection.conflict.iter().map(|cap| cap.name()).collect();
        let expected = Record {
            agent: Some("model-medium-effort-medium".to_owned()),
            demand: Some(selection.demand.to_string()),
            rules: Some(rules.join(",")),
            conflict: Some(if conflict.is_empty() {
                "none".to_owned()
            } else {
                conflict.join(",")
            }),
            oms_version: Some(oms::VERSION),
        };
        assert_eq!(*record, expected, "{name}");
    }
}
