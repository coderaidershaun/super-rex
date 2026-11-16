//! The cost of record of a job, read by `rex job show` through each way the store
//! finds the session's transcripts: the path the plugin sent, and the file where
//! Claude Code keeps them when no path reached the store.

use std::path::Path;

use assert_cmd::Command;
use super_rex::capture::TokenCounts;

use super::fixture::*;

/// `rex job show` of JOB_A, run in the fixture's repository with Claude Code's
/// config folder at `config_dir`.
fn job_show(fixture: &Fixture, config_dir: &Path) -> String {
    let output = Command::cargo_bin("rex")
        .unwrap()
        .args(["job", "show", JOB_A])
        .current_dir(fixture.repo())
        .env("CLAUDE_CONFIG_DIR", config_dir)
        .env_remove("REX_STORE")
        .env_remove("CLAUDE_PROJECT_DIR")
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    String::from_utf8(output.stdout).unwrap()
}

fn scalar<'a>(shown: &'a str, key: &str) -> &'a str {
    shown
        .lines()
        .find_map(|line| line.strip_prefix(key)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("no {key} in {shown}"))
}

/// The job's figures as `rex job show` prints them.
fn figures(shown: &str) -> [&str; 6] {
    [
        "tokens_input",
        "tokens_cache_write",
        "tokens_cache_read",
        "tokens_output",
        "cost_usd",
        "incomplete",
    ]
    .map(|key| scalar(shown, key))
}

/// The session with its job, every event sent but the transcript's path.
fn recorded(place: TranscriptPlace) -> Fixture {
    let mut fixture = Fixture::with_transcripts(place);
    fixture.open_job(JOB_A, BEFORE_SESSION).unwrap();
    fixture.append(&fixture.lines.clone());
    fixture
}

// The plugin may never learn the path, or its event may be lost, so the
// binary looks where Claude Code keeps a session's transcripts. Found there,
// the job's figures are the ones the sent path gives.
#[test]
fn the_cost_of_record_is_the_same_whether_the_path_was_sent_or_found() {
    let sent = recorded(TranscriptPlace::Sent);
    let found = recorded(TranscriptPlace::Found);

    let by_sent_path = job_show(&sent, sent.config_dir());
    let by_found_file = job_show(&found, found.config_dir());

    let expected = expected();
    let mut tokens: TokenCounts = expected_tokens(&expected["main_session"]["tokens"]);
    for agent in expected["agents"].as_array().unwrap() {
        tokens += expected_tokens(&agent["tokens"]);
    }
    let counts = [
        tokens.input,
        tokens.cache_creation,
        tokens.cache_read,
        tokens.output,
    ]
    .map(|count| count.to_string());
    assert_eq!(figures(&by_sent_path)[..4], counts, "{by_sent_path}");
    assert_eq!(figures(&by_sent_path)[5], "no", "{by_sent_path}");
    assert_ne!(figures(&by_sent_path)[4], "none", "{by_sent_path}");
    assert_eq!(figures(&by_found_file), figures(&by_sent_path));

    // With the config folder elsewhere the session's main transcript is not
    // found, so its own tokens are missing from the job.
    let elsewhere = tempfile::tempdir().unwrap();
    let without = job_show(&recorded(TranscriptPlace::Found), elsewhere.path());
    assert_ne!(figures(&without)[..4], counts, "{without}");
}
