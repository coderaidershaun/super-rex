//! A reader that closes stdout early, as `head` does, must not stop
//! `rex init` half way.

use std::io::Write;
use std::process::{Command, Output, Stdio};

use super::support::{Repo, SKILL_MARKER, repo};

/// Runs `rex` with the read end of its stdout already closed, so its first
/// write fails with a broken pipe. A shell holds the binary back until it
/// reads a line, which makes the order of events exact. The shell is named by
/// its full path because PATH holds only the stand-in tools.
fn run_with_closed_stdout(args: &[&str], repo: &Repo) -> Output {
    let mut child = Command::new("/bin/sh")
        .args([
            "-c",
            "read _; exec \"$0\" \"$@\"",
            env!("CARGO_BIN_EXE_rex"),
        ])
        .args(args)
        .current_dir(repo.path())
        .env("PATH", repo.bin())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn rex through sh");

    drop(child.stdout.take());
    child
        .stdin
        .take()
        .expect("child stdin")
        .write_all(b"\n")
        .expect("release the shell");

    child.wait_with_output().expect("wait for rex")
}

fn assert_clean_exit(output: &Output) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "rex failed: {stderr}");
    assert!(!stderr.contains("panicked"), "rex panicked: {stderr}");
}

// This test must never be deleted. A closed stdout must not stop the install
// once the skill files are written, because that would leave no AGENTS.md,
// .gitignore block or pre-commit config.
#[test]
fn skills_finishes_the_whole_install_when_stdout_is_closed() {
    let repo = repo();

    let output = run_with_closed_stdout(&["init"], &repo);

    assert_clean_exit(&output);
    let marker = format!(".claude/{SKILL_MARKER}");
    for installed in [
        marker.as_str(),
        "AGENTS.md",
        ".gitignore",
        ".pre-commit-config.yaml",
    ] {
        assert!(
            repo.path().join(installed).is_file(),
            "{installed} must exist after the install"
        );
    }
}
