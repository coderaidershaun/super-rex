//! The reader of the main session's transcript, on the recorded session's real file:
//! the tokens inside a job's time, and the model and effort of the builder.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use super_rex::capture::{MainFacts, TokenCounts, read_main_transcript};
use super_rex::store::JobId;

use super::{fixture, read_json, sum_over_models};

/// Before every record of the recording.
const EARLIEST: &str = "2026-01-01T00:00:00.000Z";

fn main_transcript() -> PathBuf {
    let session_id = read_json(&fixture("tree/expected.json"))["session_id"]
        .as_str()
        .unwrap()
        .to_owned();
    fixture(&format!("tree/transcripts/{session_id}.jsonl"))
}

fn job(id: &str) -> JobId {
    id.parse().unwrap()
}

fn read(path: &Path, job_id: &str, from: &str, to: Option<&str>) -> MainFacts {
    read_main_transcript(path, &job(job_id), from, to)
        .unwrap()
        .unwrap_or_else(|| panic!("no transcript at {}", path.display()))
}

/// One request of the main session: its message ID, the time of its first record,
/// and its tokens.
struct Request {
    first_at: String,
    tokens: TokenCounts,
}

/// The requests of the transcript at `path`, in the order of their first record.
fn requests(path: &Path) -> Vec<Request> {
    let mut seen = Vec::new();
    let mut requests = Vec::new();
    for line in fs::read_to_string(path).unwrap().lines() {
        let record: Value = serde_json::from_str(line).unwrap();
        let message = &record["message"];
        let Some(id) = message["id"].as_str() else {
            continue;
        };
        if record["type"] != "assistant" || seen.contains(&id.to_owned()) {
            continue;
        }
        seen.push(id.to_owned());
        let usage = &message["usage"];
        requests.push(Request {
            first_at: record["timestamp"].as_str().unwrap().to_owned(),
            tokens: TokenCounts {
                input: usage["input_tokens"].as_u64().unwrap(),
                output: usage["output_tokens"].as_u64().unwrap(),
                cache_read: usage["cache_read_input_tokens"].as_u64().unwrap(),
                cache_creation: usage["cache_creation_input_tokens"].as_u64().unwrap(),
            },
        });
    }
    requests
}

fn sum(requests: &[Request]) -> TokenCounts {
    let mut total = TokenCounts::default();
    for request in requests {
        total += request.tokens;
    }
    total
}

/// A copy of the transcript at `path` with `records` appended, one on each line.
fn copy_with(path: &Path, folder: &Path, name: &str, records: &[&str]) -> PathBuf {
    let mut text = fs::read_to_string(path).unwrap();
    for record in records {
        text.push_str(record);
        text.push('\n');
    }
    let copy = folder.join(name);
    fs::write(&copy, text).unwrap();
    copy
}

const BAD_TIME: &str = r#"{"type":"assistant","timestamp":"2026-11-10T13:10:44Z","message":{"id":"msg_bad_time","model":"claude-sonnet-5-5","content":[],"usage":{"input_tokens":1000,"output_tokens":1000,"cache_read_input_tokens":1000,"cache_creation_input_tokens":1000}}}"#;
const BUILDER_CALL: &str = r#"{"type":"assistant","timestamp":"2026-11-10T13:10:45.000Z","effort":"high","message":{"id":"msg_builder","model":"claude-opus-5-5","content":[{"type":"tool_use","id":"toolu_builder","name":"Bash","input":{"command":"rex job create"}}]}}"#;
const BUILDER_RESULT: &str = r#"{"type":"user","timestamp":"2026-11-10T13:10:46.000Z","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"toolu_builder","content":"job=k3x9p2m7qd\n"}]}}"#;
const EXISTING_RESULT: &str = r#"{"type":"user","timestamp":"2026-11-10T13:10:46.000Z","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"toolu_builder","content":"job=k3x9p2m7qd\nexisting=yes\n"}]}}"#;

#[test]
fn main_transcript_gives_window_tokens_and_builder() {
    let path = main_transcript();
    let requests = requests(&path);
    assert!(requests.len() >= 4, "too few requests to test a window");

    let whole = read(&path, "k3x9p2m7qd", EARLIEST, None);
    let expected = read_json(&fixture("tree/expected.json"));
    assert_eq!(
        whole.tokens,
        sum_over_models(&expected["main_session"]["tokens"])
    );
    assert_eq!(whole.tokens, sum(&requests));
    assert_eq!(whole.damaged_lines, 0);
    assert_eq!(whole.bytes, fs::metadata(&path).unwrap().len());
    assert_eq!((whole.builder_model, whole.builder_effort), (None, None));

    let from = requests[1].first_at.as_str();
    let to = requests[3].first_at.as_str();
    let window = read(&path, "k3x9p2m7qd", from, Some(to));
    assert_eq!(window.tokens, sum(&requests[1..3]), "window [{from}, {to})");

    let folder = tempfile::tempdir().unwrap();
    let builder = copy_with(
        &path,
        folder.path(),
        "builder.jsonl",
        &[BAD_TIME, BUILDER_CALL, BUILDER_RESULT],
    );
    let facts = read(&builder, "k3x9p2m7qd", EARLIEST, None);
    assert_eq!(facts.damaged_lines, 1, "the record with a bad time");
    assert_eq!(facts.tokens, whole.tokens);
    assert_eq!(facts.builder_model.as_deref(), Some("claude-opus-5-5"));
    assert_eq!(facts.builder_effort.as_deref(), Some("high"));

    let other_job = read(&builder, "0g4cbf7k2q", EARLIEST, None);
    assert_eq!(
        (other_job.builder_model, other_job.builder_effort),
        (None, None)
    );

    let existing = copy_with(
        &path,
        folder.path(),
        "existing.jsonl",
        &[BUILDER_CALL, EXISTING_RESULT],
    );
    let facts = read(&existing, "k3x9p2m7qd", EARLIEST, None);
    assert_eq!((facts.builder_model, facts.builder_effort), (None, None));

    let missing = read_main_transcript(
        &folder.path().join("none.jsonl"),
        &job("k3x9p2m7qd"),
        EARLIEST,
        None,
    );
    assert!(
        matches!(missing, Ok(None)),
        "a missing transcript gave {missing:?}"
    );
}
