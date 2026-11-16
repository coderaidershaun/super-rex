//! Proves the store reader of `rex watch` over a store the plugin's events filled:
//! a run shows its task, planned agent, brief check, skills and report, also when a
//! read of the transcripts made its row before its start event came.

use std::path::Path;
use std::time::SystemTime;

use serde_json::{Value, json};

use super::*;
use crate::commands::watch::data::WatchData;
use crate::commands::watch::nav::Node;
use crate::ingest::ingest;
use crate::store::StorePaths;

const SESSION: &str = "53f1c255-4567-4e00-ab35-553d90ce69d1";
const JOB: &str = "0gaq820001";

/// A new store in `dir`, which needs no Git repository.
fn new_store(dir: &Path) -> Store {
    let rex = dir.join(".rex");
    std::fs::create_dir_all(&rex).unwrap();
    Store::create(&StorePaths {
        root: dir.to_owned(),
        db: rex.join("rex.db"),
        orchestrators: rex.join("orchestrators"),
        dir: rex,
    })
    .unwrap()
}

/// Sends `events` as the next batch of the session, numbered from `first`.
fn send(store: &mut Store, first: usize, events: &[Value]) {
    let numbered: Vec<Value> = events
        .iter()
        .enumerate()
        .map(|(index, event)| {
            let mut event = event.clone();
            event["seq"] = json!(first + index);
            event
        })
        .collect();
    let batch = json!({ "version": 1, "session": SESSION, "stream": "aaaaaaaaaaaaaaaa",
                        "secret": "1".repeat(64), "events": numbered });
    ingest(store, batch.to_string().as_bytes(), &|_| None).unwrap();
}

fn event(kind: &str, at: &str, job: Option<&str>, data: Value) -> Value {
    json!({ "at": at, "kind": kind, "agent": "main", "job": job, "data": data })
}

fn job_open() -> Value {
    event(
        "job.open",
        "2026-11-13T20:00:02.000Z",
        Some(JOB),
        json!({
            "owner": { "session": SESSION, "builder": "main" }, "origin": "composer",
            "request": "A request.", "restatement": null, "repo": "/r", "branch": "rex/0gaq820001",
            "base": "main", "worktree": null, "type": "fix", "size": "medium", "consequence": "high",
            "langs": ["rust"], "prefs": { "duration": null, "budget": null, "autonomy": null },
            "limit_nano": 32_000_000_000_u64, "limit_source": "default", "tree": null,
            "depends_on": [], "merge_after": []
        }),
    )
}

fn run_start() -> Value {
    event(
        "run.start",
        "2026-11-13T20:02:00.000Z",
        Some(JOB),
        json!({
            "agent_id": "a1", "parent": "main", "parent_source": "engine", "depth": 1,
            "agent_type": "super-rex:model-medium-effort-medium",
            "planned": "model-medium-effort-medium", "role": "engineer", "task": "T1",
            "tool_use_id": "toolu_1", "brief_hash": null, "assembled": "yes", "lint": "pass",
            "skills_attached": ["rex-code-tdd", "rex-code-philosophy"], "tree": null
        }),
    )
}

fn skill_open(at: &str, skill: &str, file: &str, how: &str) -> Value {
    event(
        "skill.open",
        at,
        Some(JOB),
        json!({ "agent_id": "a1", "skill": skill, "file": file, "hash": "sha256:0", "how": how }),
    )
}

fn run_end() -> Value {
    event(
        "run.end",
        "2026-11-13T20:03:00.000Z",
        None,
        json!({
            "agent_id": "a1", "reason": "answer",
            "tokens": { "input": 1, "output": 2, "cache_read": 3, "cache_creation": 4 },
            "spent_nano": 5, "final_message": "Done. Report: /r/out.md", "output_file": "/r/out.md"
        }),
    )
}

/// The facts of run `a1` as `rex watch` shows them from the store alone.
fn facts_of_the_run(store: &mut Store) -> Vec<(&'static str, String)> {
    let part = load(store, SystemTime::now()).unwrap();
    WatchData::merge(Vec::new(), Some(part)).facts(&Node::Run("a1".into()))
}

fn fact<'a>(facts: &'a [(&'static str, String)], label: &str) -> &'a str {
    facts
        .iter()
        .find(|(shown, _)| *shown == label)
        .map_or("", |(_, value)| value.as_str())
}

#[test]
fn a_run_of_the_store_shows_what_the_plugin_recorded_of_it() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = new_store(temp.path());
    send(
        &mut store,
        1,
        &[
            job_open(),
            run_start(),
            skill_open(
                "2026-11-13T20:02:01.000Z",
                "rex-code-tdd",
                "SKILL.md",
                "attached",
            ),
            skill_open(
                "2026-11-13T20:02:30.000Z",
                "rex-deliver",
                "start.md",
                "rex_skill",
            ),
            skill_open(
                "2026-11-13T20:02:40.000Z",
                "rex-deliver",
                "start.md",
                "rex_skill",
            ),
            run_end(),
        ],
    );

    let facts = facts_of_the_run(&mut store);

    assert_eq!(fact(&facts, "role"), "engineer");
    assert_eq!(fact(&facts, "task"), "T1");
    assert_eq!(
        fact(&facts, "agent"),
        "super-rex:model-medium-effort-medium  planned model-medium-effort-medium"
    );
    assert_eq!(fact(&facts, "brief check"), "pass");
    assert_eq!(
        fact(&facts, "skills given"),
        "rex-code-philosophy, rex-code-tdd"
    );
    assert_eq!(
        fact(&facts, "skills opened"),
        "rex-code-tdd/SKILL.md, rex-deliver/start.md",
        "in the order opened, each once"
    );
    assert_eq!(fact(&facts, "report"), "/r/out.md");
}

#[test]
fn a_run_whose_row_a_transcript_read_made_first_keeps_what_its_start_says() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = new_store(temp.path());
    send(&mut store, 1, &[job_open()]);
    // The row as recovery writes it: the job, the ids, the times and the transcript.
    store
        .conn()
        .execute(
            "INSERT INTO sub_agent_run (job_id, session_id, agent_id, first_seen_at, started_at,
                                        transcript_path)
             VALUES (?1, ?2, 'a1', '2026-11-13T20:02:01.000Z', '2026-11-13T20:01:59.990Z',
                     '/t/agent-a1.jsonl')",
            [JOB, SESSION],
        )
        .unwrap();

    send(&mut store, 2, &[run_start(), run_end()]);

    let facts = facts_of_the_run(&mut store);
    assert_eq!(fact(&facts, "role"), "engineer");
    assert_eq!(fact(&facts, "task"), "T1");
    assert_eq!(
        fact(&facts, "agent"),
        "super-rex:model-medium-effort-medium  planned model-medium-effort-medium"
    );
    assert_eq!(fact(&facts, "brief check"), "pass");
    assert_eq!(
        fact(&facts, "skills given"),
        "rex-code-philosophy, rex-code-tdd"
    );
    let started: String = store
        .conn()
        .query_row(
            "SELECT started_at FROM sub_agent_run WHERE agent_id = 'a1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        started, "2026-11-13T20:01:59.990Z",
        "the row keeps its first start"
    );
}
