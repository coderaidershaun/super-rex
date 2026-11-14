//! A job's life as the plugin records it through `rex ingest`, read back with
//! `rex job show` from a worktree: opened, planned, its run started and ended,
//! and closed with one delivery for each of two repositories.

use serde_json::json;

use super::fixture::{
    COMMIT, SESSION, event, ingest, job_open, new_repo, new_store, new_worktree, rex, show_json,
    stderr_of, stdout_of, storage_on, value,
};

const JOB: &str = "0gaq820001";

#[test]
fn job_is_opened_planned_run_closed_and_shown_from_a_worktree() {
    let temp = tempfile::tempdir().unwrap();
    let shop = new_repo(temp.path(), "shop");
    let web = new_repo(temp.path(), "web-client");
    new_store(&shop, "pipeline");
    storage_on(&shop);
    let worktree = new_worktree(&shop, temp.path(), "rex-0gaq820001");

    let plan = json!({
        "orchestrator": "pipeline", "breakdown": "1. Reject zero.",
        "tasks": [{ "id": "T1", "role": "engineer", "skills": ["rex-code-tdd"], "open_when": [] }],
        "roles": [{ "role": "engineer", "intelligence": "high", "reliability": "high",
                    "consequence": "high", "verification": "low",
                    "grid_agent": "model-high-effort-high", "conflict": null, "oms_version": 2 }],
        "manifest": [{ "skill": "rex-code-tdd", "file": "SKILL.md",
                       "hash": format!("sha256:{}", "a".repeat(64)), "source": "plugin" }]
    });
    let last = ingest(
        &worktree,
        SESSION,
        1,
        &[
            event(
                "job.open",
                "2026-11-13T20:00:02.000Z",
                Some(JOB),
                job_open(SESSION, "main", &shop, JOB),
            ),
            event(
                "job.merge_answer",
                "2026-11-13T20:00:03.000Z",
                Some(JOB),
                json!({ "answer": "yes", "source": "dialog", "question": null, "policy_hash": null }),
            ),
            event("job.plan", "2026-11-13T20:01:00.000Z", Some(JOB), plan),
            event(
                "run.start",
                "2026-11-13T20:02:00.000Z",
                Some(JOB),
                json!({
                "agent_id": "a1", "parent": "main", "parent_source": "engine", "depth": 1,
                "agent_type": "super-rex:model-high-effort-high", "planned": "model-high-effort-high",
                "role": "engineer", "task": "T1", "tool_use_id": "toolu_1", "brief_hash": null,
                "assembled": "yes", "lint": "pass", "skills_attached": ["rex-code-tdd"], "tree": null }),
            ),
            event(
                "run.end",
                "2026-11-13T20:03:00.000Z",
                None,
                json!({
                "agent_id": "a1", "reason": "answer",
                "tokens": { "input": 1, "output": 2, "cache_read": 3, "cache_creation": 4 },
                "spent_nano": 5, "final_message": "Done. Report: /r/out.md", "output_file": "/r/out.md" }),
            ),
        ],
    );
    assert_eq!(last, "last_sequence=5\n");

    // A second open job for one owner is refused; the batch changes nothing.
    let other = ingest_refused(&worktree, 6, "0gaq820002");
    assert!(
        other.contains("seq 6 job.open: job 0gaq820002 is in the store already, or its owner"),
        "{other}"
    );

    // A delivery that is blocked is recorded, then recorded again once merged.
    let note = "pull request 17 waits for a human's approval";
    let close = |status: &str, web_outcome: serde_json::Value| {
        json!({ "status": status, "outcome": "Shop merged", "deliveries": [
            web_outcome,
            { "repo": shop, "branch": format!("rex/{JOB}"), "pr": null, "outcome": "merged",
              "merge_commit": COMMIT, "note": null } ] })
    };
    ingest(
        &shop,
        SESSION,
        6,
        &[event(
            "job.close",
            "2026-11-13T20:04:00.000Z",
            Some(JOB),
            close(
                "blocked",
                json!({
            "repo": web, "branch": null, "pr": "https://example.com/pull/17", "outcome": "blocked",
            "merge_commit": null, "note": note }),
            ),
        )],
    );
    let shown = stdout_of(&mut rex(&worktree, &["job", "show", JOB]));
    assert_eq!(value(&shown, "status"), "blocked");
    assert!(
        value(&shown, "delivery.1").starts_with("blocked "),
        "{shown}"
    );
    ingest(
        &shop,
        SESSION,
        7,
        &[event(
            "job.close",
            "2026-11-13T20:05:00.000Z",
            Some(JOB),
            close(
                "delivered",
                json!({
            "repo": web, "branch": null, "pr": "https://example.com/pull/17", "outcome": "merged",
            "merge_commit": COMMIT, "note": null }),
            ),
        )],
    );

    let record = show_json(&worktree, JOB);
    let job = &record["job"];
    assert_eq!(job["status"], "delivered");
    assert_eq!(
        job["merge"],
        json!({ "answer": "yes", "source": "dialog", "question": null })
    );
    assert_eq!(job["skills"], json!(["rex-code-tdd"]));
    assert_eq!(job["orchestrator"]["name"], "pipeline");
    assert_eq!(record["roles"][0]["agent"], "model-high-effort-high");
    assert_eq!(record["manifest"][0]["skill"], "rex-code-tdd");
    let run = &record["runs"][0];
    assert_eq!(
        (run["agent_id"].as_str(), run["role"].as_str()),
        (Some("a1"), Some("engineer"))
    );
    assert_eq!(run["final_message"], "Done. Report: /r/out.md");
    let outcomes: Vec<&str> = record["deliveries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|delivery| delivery["outcome"].as_str().unwrap())
        .collect();
    assert_eq!(outcomes, ["merged", "merged"]);
    let statuses: Vec<&str> = record["transitions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["to"].as_str().unwrap())
        .collect();
    assert_eq!(statuses, ["created", "planned", "blocked", "delivered"]);
}

/// Sends the opening of `job` for the owner that holds the open job, and gives
/// the refusal.
fn ingest_refused(cwd: &std::path::Path, seq: usize, job: &str) -> String {
    let batch = json!({ "version": 1, "session": SESSION, "stream": "aaaaaaaaaaaaaaaa",
        "secret": "1".repeat(64), "events": [{ "seq": seq, "at": "2026-11-13T20:03:30.000Z",
        "kind": "job.open", "agent": "main", "job": job,
        "data": job_open(SESSION, "main", cwd, job) }] });
    stderr_of(rex(cwd, &["ingest"]).write_stdin(batch.to_string()), 2)
}
