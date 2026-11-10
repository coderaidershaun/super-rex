//! The reader of one sub-agent transcript, on the recorded session's real files.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use super_rex::capture::{TokenCounts, TranscriptFacts, read_transcript};

use super::{count, fixture, read_json, sum_over_models};

fn facts_of(path: &Path) -> TranscriptFacts {
    read_transcript(path)
        .unwrap()
        .unwrap_or_else(|| panic!("no transcript at {}", path.display()))
}

fn tree_subagent(session_id: &str, agent_id: &str) -> PathBuf {
    fixture(&format!(
        "tree/transcripts/{session_id}/subagents/agent-{agent_id}.jsonl"
    ))
}

fn assistant_records(path: &Path) -> usize {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|line| serde_json::from_str::<Value>(line).unwrap()["type"] == "assistant")
        .count()
}

#[test]
fn transcript_totals_match_recorded_session() {
    let expected = read_json(&fixture("tree/expected.json"));
    let session_id = expected["session_id"].as_str().unwrap();
    let mut by_model: BTreeMap<String, TokenCounts> = BTreeMap::new();
    let mut runs_with_repeated_ids = 0;
    let mut background_runs = 0;

    for agent in expected["agents"].as_array().unwrap() {
        let agent_id = agent["agent_id"].as_str().unwrap();
        let path = tree_subagent(session_id, agent_id);
        let facts = facts_of(&path);

        assert_eq!(
            facts.tokens,
            sum_over_models(&agent["tokens"]),
            "tokens of {agent_id}"
        );
        assert_eq!(
            facts.requests,
            count(agent, "requests"),
            "requests of {agent_id}"
        );
        let transcript_counts = &agent["transcript_counts"];
        assert_eq!(
            facts.tool_uses,
            count(transcript_counts, "tool_uses"),
            "tool uses of {agent_id}"
        );
        assert_eq!(
            facts.lines_added,
            count(transcript_counts, "lines_added"),
            "lines added of {agent_id}"
        );
        assert_eq!(
            facts.lines_removed,
            count(transcript_counts, "lines_removed"),
            "lines removed of {agent_id}"
        );

        if assistant_records(&path) as u64 > facts.requests {
            runs_with_repeated_ids += 1;
        }
        if agent["background"] == true {
            background_runs += 1;
        }
        *by_model.entry(facts.model.clone().unwrap()).or_default() += facts.tokens;
    }
    assert!(
        runs_with_repeated_ids > 0,
        "no run repeats a message ID, so the case is not tested"
    );
    assert_eq!(
        background_runs, 1,
        "the background run must be among the runs checked"
    );

    let main = facts_of(&fixture(&format!("tree/transcripts/{session_id}.jsonl")));
    *by_model.entry(main.model.clone().unwrap()).or_default() += main.tokens;

    let results = read_json(&fixture("tree/result.json"));
    let model_usage = results.as_array().unwrap().last().unwrap()["modelUsage"]
        .as_object()
        .unwrap();
    let reported: BTreeMap<String, TokenCounts> = model_usage
        .iter()
        .map(|(model, usage)| {
            let counts = TokenCounts {
                input: count(usage, "inputTokens"),
                output: count(usage, "outputTokens"),
                cache_read: count(usage, "cacheReadInputTokens"),
                cache_creation: count(usage, "cacheCreationInputTokens"),
            };
            (model.clone(), counts)
        })
        .collect();
    assert_eq!(by_model, reported);

    let skills = read_json(&fixture("skills/expected.json"));
    let skills_session = skills["session_id"].as_str().unwrap();
    for agent in skills["agents"].as_array().unwrap() {
        let path = match agent["agent_id"].as_str() {
            None => fixture(&format!("skills/transcripts/{skills_session}.jsonl")),
            Some(agent_id) => fixture(&format!(
                "skills/transcripts/{skills_session}/subagents/agent-{agent_id}.jsonl"
            )),
        };
        let mut expected_skills: Vec<String> = agent["skills_opened"]
            .as_array()
            .unwrap()
            .iter()
            .map(|name| name.as_str().unwrap().to_owned())
            .collect();
        expected_skills.sort();
        assert_eq!(
            facts_of(&path).skills_opened,
            expected_skills,
            "skills of {}",
            agent["description"]
        );
    }
}

#[test]
fn damaged_cut_or_missing_transcript_is_marked() {
    let expected = read_json(&fixture("tree/expected.json"));
    let session_id = expected["session_id"].as_str().unwrap();
    let original_path = tree_subagent(session_id, "acefc7f2b8d9b97bf");
    let original = fs::read_to_string(&original_path).unwrap();
    let folder = tempfile::tempdir().unwrap();

    let whole_lines = original.trim_end_matches('\n');
    let last_line_start = whole_lines.rfind('\n').unwrap() + 1;
    let whole_lines_path = folder.path().join("whole-lines.jsonl");
    fs::write(&whole_lines_path, &original[..last_line_start]).unwrap();
    let cut = &original[..last_line_start + (whole_lines.len() - last_line_start) / 2];
    let cut_path = folder.path().join("cut.jsonl");
    fs::write(&cut_path, cut).unwrap();
    let expected_cut = TranscriptFacts {
        damaged_lines: 1,
        bytes: cut.len() as u64,
        ..facts_of(&whole_lines_path)
    };
    assert_eq!(facts_of(&cut_path), expected_cut);

    let first_line_end = original.find('\n').unwrap() + 1;
    let garbled = format!(
        "{}{{\"type\":\"assistant\",\"message\":{{\"id\n{}",
        &original[..first_line_end],
        &original[first_line_end..]
    );
    let garbled_path = folder.path().join("garbled.jsonl");
    fs::write(&garbled_path, &garbled).unwrap();
    let expected_garbled = TranscriptFacts {
        damaged_lines: 1,
        bytes: garbled.len() as u64,
        ..facts_of(&original_path)
    };
    assert_eq!(facts_of(&garbled_path), expected_garbled);

    let missing = read_transcript(&folder.path().join("no-such-transcript.jsonl"));
    assert!(
        matches!(missing, Ok(None)),
        "a missing transcript gave {missing:?}"
    );
}
