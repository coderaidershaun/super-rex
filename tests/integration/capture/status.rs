//! `rex status` run on a store that holds the recorded session: the job with its
//! runs, no job open, a job that is not there, and an ID that is not an ID.

use std::fs::{self, File};
use std::path::Path;
use std::time::SystemTime;

use assert_cmd::Command;
use serde_json::Value;
use super_rex::store::StoreError;

#[allow(dead_code)]
#[path = "../../contract/capture/fixture.rs"]
mod fixture;

use fixture::{
    AFTER_SESSION, BEFORE_SESSION, BG, BG_CHILD, Fixture, JOB_A, JOB_B, LEAD, PLAIN, REVIEWER,
    WORKER, event_of,
};

/// `rex status` in `cwd`, with the two variables that steer the store search removed,
/// so the test machine's own setup cannot change the result.
fn rex_status(cwd: &Path, args: &[&str]) -> Command {
    let mut command = Command::cargo_bin("rex").unwrap();
    command
        .arg("status")
        .args(args)
        .current_dir(cwd)
        .env_remove("REX_STORE")
        .env_remove("CLAUDE_PROJECT_DIR");
    command
}

fn stdout_of(cwd: &Path, args: &[&str]) -> String {
    let output = rex_status(cwd, args)
        .assert()
        .success()
        .get_output()
        .clone();
    assert!(output.stderr.is_empty(), "something on standard error");
    String::from_utf8(output.stdout).unwrap()
}

/// The seconds that a duration of the form `41s`, `2m05s` or `1h02m` states.
fn seconds_of(text: &str) -> u64 {
    let number = |part: &str| part.parse::<u64>().unwrap();
    if let Some((hours, minutes)) = text.trim_end_matches('m').split_once('h') {
        return number(hours) * 3600 + number(minutes) * 60;
    }
    let seconds = text.trim_end_matches('s');
    seconds.split_once('m').map_or_else(
        || number(seconds),
        |(minutes, seconds)| number(minutes) * 60 + number(seconds),
    )
}

/// `41s`, `2m05s` or `1h02m`.
fn is_duration(text: &str) -> bool {
    let digits = |part: &str| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());
    let two_digits = |part: &str| part.len() == 2 && digits(part);
    if let Some(seconds) = text.strip_suffix('s') {
        return match seconds.split_once('m') {
            Some((minutes, seconds)) => digits(minutes) && two_digits(seconds),
            None => digits(seconds),
        };
    }
    text.strip_suffix('m')
        .and_then(|rest| rest.split_once('h'))
        .is_some_and(|(hours, minutes)| digits(hours) && two_digits(minutes))
}

#[test]
fn status_prints_runs() {
    let mut fixture = Fixture::new();
    fixture.open_job(JOB_A, BEFORE_SESSION).unwrap();
    fixture.append(&fixture.lines.clone());
    let root = fixture.store.paths().root.clone();

    let text = stdout_of(&root, &[JOB_A]);

    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 8, "header, section name, six runs:\n{text}");
    assert_eq!(
        lines[0],
        "job k3x9p2m7qd  created  0 open  6 finished  0 unconfirmed"
    );
    assert_eq!(lines[1], "finished");
    let expected_runs: [(&str, &[&str]); 6] = [
        (
            BG,
            &[
                "background-worker",
                "model-low-effort-medium",
                "bg",
                "60.4k",
                "tokens~",
            ],
        ),
        (
            LEAD,
            &["lead", "model-medium-effort-low", "52.9k", "tokens~"],
        ),
        (
            BG_CHILD,
            &[
                "worker",
                "model-low-effort-low",
                "<a70d8a0f630d8701c",
                "24.1k",
                "tokens~",
            ],
        ),
        (
            WORKER,
            &[
                "worker",
                "model-low-effort-low",
                "<acefc7f2b8d9b97bf",
                "24.4k",
                "tokens~",
            ],
        ),
        (
            REVIEWER,
            &["reviewer", "model-high-effort-low", "24.1k", "tokens~"],
        ),
        (PLAIN, &["-", "general-purpose", "11.5k", "tokens~"]),
    ];
    for (line, (agent, rest)) in lines[2..].iter().zip(expected_runs) {
        assert!(line.starts_with(&format!("  {agent}  ")), "{line}");
        let mut words: Vec<&str> = line.split_whitespace().skip(1).collect();
        let duration = words.remove(2);
        assert!(is_duration(duration), "{duration} in {line}");
        assert_eq!(words, rest, "{line}");
    }
    let type_offsets: Vec<usize> = lines[2..]
        .iter()
        .map(|line| {
            line.find("model-")
                .or_else(|| line.find("general-"))
                .unwrap()
        })
        .collect();
    assert!(
        type_offsets.windows(2).all(|pair| pair[0] == pair[1]),
        "agent types do not line up:\n{text}"
    );

    assert_eq!(
        stdout_of(&root, &[]),
        text,
        "no argument gives the open job"
    );

    fixture.close_job(JOB_A, AFTER_SESSION);
    assert_eq!(stdout_of(&root, &[]), "no open job\n");

    let unknown = rex_status(&root, &[JOB_B])
        .assert()
        .code(1)
        .stdout("")
        .get_output()
        .clone();
    let error = String::from_utf8(unknown.stderr).unwrap();
    assert!(
        error.starts_with("error: ") && error.trim_end().lines().count() == 1,
        "one error line expected, got {error:?}"
    );
    rex_status(&root, &["K3X9"]).assert().code(2).stdout("");

    open_and_unconfirmed_runs_are_shown();
}

/// A planned job whose REVIEWER and PLAIN runs lost their stop and completed-call
/// lines and their `finished` mark: REVIEWER's transcript is fresh, PLAIN's is three
/// hours old. The log also holds one damaged line.
fn open_and_unconfirmed_runs_are_shown() {
    let mut fixture = Fixture::new();
    fixture.open_job(JOB_A, BEFORE_SESSION).unwrap();
    fixture
        .store
        .write(|tx| {
            tx.execute("UPDATE job SET status = 'planned' WHERE id = ?1", [JOB_A])?;
            Ok::<_, StoreError>(())
        })
        .unwrap();
    let lost = [REVIEWER, PLAIN];
    let mut log: Vec<String> = fixture
        .lines
        .iter()
        .filter(|line| {
            let event = event_of(line);
            let is_stop = event["hook_event_name"] == "SubagentStop"
                && lost.iter().any(|agent| event["agent_id"] == *agent);
            let is_completed_call = lost
                .iter()
                .any(|agent| event["tool_response"]["agentId"] == *agent)
                && event["hook_event_name"] == "PostToolUse";
            !is_stop && !is_completed_call
        })
        .cloned()
        .collect();
    log.push("this line is not JSON".to_owned());
    fixture.append(&log);
    for agent in lost {
        let path = fixture.metadata(agent);
        let mut metadata: Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        metadata.as_object_mut().unwrap().remove("finished");
        fs::write(path, metadata.to_string()).unwrap();
    }
    // An hour after the recorded session, so it is newer than the run's start and
    // the quiet time must be counted from here, not from the start.
    let last_write = fixture.time("2026-11-10T14:00:00.000Z");
    File::options()
        .write(true)
        .open(fixture.transcript(PLAIN))
        .unwrap()
        .set_modified(last_write)
        .unwrap();
    let root = fixture.store.paths().root.clone();

    let before = SystemTime::now();
    let text = stdout_of(&root, &[]);
    let after = SystemTime::now();

    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 11, "{text}");
    assert_eq!(
        lines[0],
        "job k3x9p2m7qd  planned (running)  1 open  4 finished  1 unconfirmed  incomplete"
    );
    assert_eq!(lines[1], "open");
    let open: Vec<&str> = lines[2].split_whitespace().collect();
    assert_eq!(
        &open[..3],
        [REVIEWER, "reviewer", "model-high-effort-low"],
        "{text}"
    );
    assert!(is_duration(open[3]), "{text}");
    assert_eq!(open[4..], ["24.1k", "tokens~"], "{text}");
    assert_eq!(lines[3], "finished");
    assert_eq!(lines[8], "unconfirmed");
    let plain: Vec<&str> = lines[9].split_whitespace().collect();
    assert_eq!(
        &plain[..7],
        [
            PLAIN,
            "-",
            "general-purpose",
            "no",
            "stop",
            "event,",
            "quiet"
        ],
        "{text}"
    );
    // The hour form drops its seconds, so allow a minute below the true value.
    let since = |time: SystemTime| time.duration_since(last_write).unwrap().as_secs();
    let quiet = seconds_of(plain[7]);
    assert!(
        quiet + 60 > since(before) && quiet <= since(after),
        "quiet {} is not the time since the transcript was written: {text}",
        plain[7]
    );
    assert_eq!(plain[8..], ["11.5k", "tokens~"], "{text}");
    assert_eq!(lines[10], "event log: 1 damaged lines");
}
