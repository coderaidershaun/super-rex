//! The hook that `rex init` installs before each shell command: a test command
//! runs with its output cut to the summary or the failures, and every other
//! command runs as it is.

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use super::support::{Repo, repo};

/// The event Claude Code sends the hook before it runs `command`.
fn bash_event(command: &str) -> Value {
    json!({
        "session_id": "s1",
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": { "command": command, "description": "Run tests", "timeout": 120000 },
        "tool_use_id": "toolu_1"
    })
}

/// What `rex test-hook` printed for `input`. It must always exit with 0.
fn test_hook(repo: &Repo, input: &str) -> String {
    let output = assert_cmd::Command::cargo_bin("rex")
        .expect("find rex binary")
        .arg("test-hook")
        .current_dir(repo.path())
        .write_stdin(input)
        .output()
        .expect("run rex test-hook");
    assert_eq!(output.status.code(), Some(0), "the hook must exit with 0");
    assert!(output.stderr.is_empty(), "the hook must print no error");
    String::from_utf8(output.stdout).expect("utf8 stdout")
}

/// Runs `command` as the Bash tool would, with the stand-in tools and `rex` on
/// PATH, and gives its output and exit status.
fn run_shell(repo: &Repo, command: &str) -> (String, i32) {
    let output = Command::new("/bin/sh")
        .arg("-c")
        .arg(command)
        .current_dir(repo.path())
        .env("PATH", repo.bin())
        .env("TMPDIR", repo.path())
        .output()
        .expect("run the command");
    assert!(output.stderr.is_empty(), "the stream is merged into stdout");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    (stdout, output.status.code().expect("an exit status"))
}

/// Lines of a failing test's message that start with words a runner also
/// prints for a passing test or a build step. None of them may be cut.
const MESSAGE_LINES: [&str; 6] = [
    "Updating the index left 3 stale entries",
    "Compiling the report took 9 s",
    "ok was expected, got err",
    "PASS was not printed",
    "Passed 2 of 3 checks",
    "Finished with 1 stale entry",
];

/// A stand-in `pytest` that passes 2000 tests, and fails one more when
/// `status` is not 0.
fn write_pytest(repo: &Repo, status: u8) {
    let message: String = MESSAGE_LINES
        .iter()
        .map(|line| format!("echo '{line}'\n"))
        .collect();
    let failure = if status == 0 {
        "echo '===== 2000 passed in 1.00s ====='".to_string()
    } else {
        format!(
            "echo 'tests/test_y.py::test_bad FAILED'\necho 'E   assert 1 == 2' >&2\n{message}echo '===== 1 failed, 2000 passed in 1.00s ====='"
        )
    };
    repo.write_tool(
        "pytest",
        &format!(
            "i=0\nwhile [ $i -lt 2000 ]; do echo \"tests/test_x.py::test_$i PASSED\"; i=$((i+1)); done\n{failure}\nexit {status}"
        ),
    );
}

const NOTE_END: &str = "To see every line, read the file named on the first line.";

/// The file that the first line of the filter's output names, and the rest of
/// the output.
fn split_file_line(output: &str) -> (PathBuf, &str) {
    let (first, rest) = output.split_once('\n').expect("a first line");
    let path = first
        .strip_prefix("rex test-filter: every line of this run goes to ")
        .and_then(|rest| rest.strip_suffix(" as it comes."))
        .unwrap_or_else(|| panic!("first line names the file, got: {first}"));
    (PathBuf::from(path), rest)
}

fn wait_for(path: &Path, text: &str) -> bool {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(10) {
        if fs::read_to_string(path).is_ok_and(|held| held.contains(text)) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

#[test]
fn test_commands_show_only_their_failures() {
    let repo = repo();

    let printed = test_hook(&repo, &bash_event("pytest -q").to_string());

    // No braces, quotes or subshell, which Claude Code's check of a Bash
    // command refuses in the default permission mode.
    let expected_command = "set -o pipefail; pytest -q 2>&1 | rex test-filter";
    let output: Value = serde_json::from_str(&printed).expect("one JSON object");
    assert_eq!(
        output,
        json!({
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "updatedInput": { "command": expected_command, "description": "Run tests", "timeout": 120000 }
            }
        })
    );
    assert!(!printed.contains("allow"), "got: {printed}");

    // A file of an earlier run older than a day is removed by the next run.
    let old_file = repo.path().join("rex-test-output-1-1.log");
    fs::write(&old_file, "old run\n").expect("write an old file");
    let touched = Command::new("/usr/bin/touch")
        .args(["-t", "200001010000"])
        .arg(&old_file)
        .status()
        .expect("run touch");
    assert!(touched.success());

    write_pytest(&repo, 0);
    let (passed, status) = run_shell(&repo, expected_command);
    let (file, rest) = split_file_line(&passed);
    assert_eq!(
        (rest, status),
        (
            format!(
                "===== 2000 passed in 1.00s =====\nrex test-filter: left out 2000 passing lines of 2001. {NOTE_END}\n"
            )
            .as_str(),
            0
        )
    );
    let every_line = fs::read_to_string(&file).expect("read the file of every line");
    assert!(!old_file.exists(), "the old file must be removed");
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&file)
            .expect("file metadata")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600, "only the owner may read the file");
    }
    assert_eq!(every_line.matches(" PASSED\n").count(), 2000);

    // A message line of the failing test is kept, whatever word it starts with.
    write_pytest(&repo, 3);
    let (failed, status) = run_shell(&repo, expected_command);
    let (_, rest) = split_file_line(&failed);
    let message = MESSAGE_LINES.join("\n");
    assert_eq!(
        (rest, status),
        (
            format!(
                "tests/test_y.py::test_bad FAILED\nE   assert 1 == 2\n{message}\n===== 1 failed, 2000 passed in 1.00s =====\nrex test-filter: left out 2000 passing lines of 2009. {NOTE_END}\n"
            )
            .as_str(),
            3
        )
    );

    // A run that hangs shows the file at once, and the file gets each line as
    // it comes, so a run stopped at the time limit still shows what it printed.
    repo.write_tool(
        "pytest",
        "echo 'test hangs_forever has been running for over 60 seconds'\nwhile [ ! -f release ]; do /bin/sleep 0.1; done\nexit 1",
    );
    let mut hanging = Command::new("/bin/sh")
        .arg("-c")
        .arg(expected_command)
        .current_dir(repo.path())
        .env("PATH", repo.bin())
        .env("TMPDIR", repo.path())
        .stdout(Stdio::piped())
        .spawn()
        .expect("start the command");
    let mut first_line = String::new();
    BufReader::new(hanging.stdout.as_mut().expect("piped stdout"))
        .read_line(&mut first_line)
        .expect("read the first line");
    let (file, _) = split_file_line(&first_line);
    let is_streamed = wait_for(&file, "hangs_forever");
    fs::write(repo.path().join("release"), "").expect("release the run");
    let finished = hanging.wait().expect("wait for the run");
    assert!(
        is_streamed,
        "the file must get each line before the run ends"
    );
    assert_eq!(finished.code(), Some(1));

    // A test command run with a pipe, after `;`, after `cd`, or with `exit` is
    // left as it is, and so is every other command and every input that is not a Bash call.
    for command in [
        "ls -la",
        "pytest -q | tail -5 -f",
        "pytest -q | grep FAILED",
        "pytest -q | tail -5 | head -1",
        "cd src && pytest -q",
        "cd src && pytest -q; rm x",
        "pytest -q && exit 0",
        "pytest -q && pytest -x",
        "pytest -q # all of them",
        "cargo testing",
        "cargo test \\",
        "cargo test -- --nocapture",
        "cargo test --help",
        "pytest -s",
        "go test -v ./...",
    ] {
        assert_eq!(
            test_hook(&repo, &bash_event(command).to_string()),
            "",
            "{command}"
        );
    }
    // A test command piped into `tail` with only a line count gets the filter in
    // place of `tail`, which could cut a failure off.
    for command in [
        "pytest -q | tail -5",
        "pytest -q 2>&1 | tail -n 50",
        "pytest -q | tail",
    ] {
        let printed = test_hook(&repo, &bash_event(command).to_string());
        let output: Value = serde_json::from_str(&printed).expect("one JSON object");
        assert_eq!(
            output["hookSpecificOutput"]["updatedInput"]["command"],
            json!(expected_command),
            "{command}"
        );
    }

    let mut outside_sandbox = bash_event("pytest -q");
    outside_sandbox["tool_input"]["dangerouslyDisableSandbox"] = json!(true);
    assert_eq!(test_hook(&repo, &outside_sandbox.to_string()), "");
    let mut not_bash = bash_event("pytest -q");
    not_bash["tool_name"] = json!("Monitor");
    for input in ["", "not json", "{}", &not_bash.to_string()] {
        assert_eq!(test_hook(&repo, input), "", "{input}");
    }

    // The captured output of a failed cargo test is kept whole, also its lines in
    // a passing form. Only the build step and the passing test before it go.
    let report = [
        "running 2 tests",
        "test tests::connection ... FAILED",
        "",
        "failures:",
        "",
        "---- tests::connection stdout ----",
        "running migration 3",
        "test connection ... ok",
        "tests/db.py::test_x PASSED",
        "PASS",
        "",
        "thread 'tests::connection' panicked at src/lib.rs:12:9:",
        "boom",
        "",
        "failures:",
        "    tests::connection",
        "",
        "test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s",
    ];
    let printed: String = ["   Compiling probe v0.1.0 (/x)"]
        .iter()
        .chain(&report[..1])
        .chain(&["test tests::other ... ok"])
        .chain(&report[1..])
        .map(|line| format!("echo \"{line}\"\n"))
        .collect();
    repo.write_tool("cargo", &format!("{printed}exit 101"));
    let cargo_command = "set -o pipefail; cargo test 2>&1 | rex test-filter";
    let (failed, status) = run_shell(&repo, cargo_command);
    let (_, rest) = split_file_line(&failed);
    let kept = report.join("\n");
    assert_eq!(
        (rest, status),
        (
            format!("{kept}\nrex test-filter: left out 2 passing lines of 20. {NOTE_END}\n")
                .as_str(),
            101
        )
    );

    // Output with no passing line is shown whole, and the filter itself always
    // ends with 0, so the pipe ends with the status of the command before it.
    let (shown, status) = run_shell(&repo, "printf 'a\\nb\\n' | rex test-filter");
    let (_, shown) = split_file_line(&shown);
    assert_eq!(
        (shown, status),
        (
            format!("a\nb\nrex test-filter: left out 0 passing lines of 2. {NOTE_END}\n").as_str(),
            0
        )
    );
}
