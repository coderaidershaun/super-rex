//! Integration tests of what `rex commands` and `rex codebase` do when stdout is closed.

#![cfg(unix)]

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use tempfile::tempdir;

/// Runs `rex` with the read end of its stdout already closed, so its first
/// write fails with a broken pipe. A shell holds the binary back until it
/// reads a line, which makes the order of events exact.
fn run_with_closed_stdout(args: &[&str], cwd: &Path) -> Output {
    let mut child = Command::new("sh")
        .args([
            "-c",
            "read _; exec \"$0\" \"$@\"",
            env!("CARGO_BIN_EXE_rex"),
        ])
        .args(args)
        .current_dir(cwd)
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

#[test]
fn commands_does_not_panic_when_stdout_is_closed() {
    let dir = tempdir().expect("create tempdir");

    let output = run_with_closed_stdout(&["commands"], dir.path());

    assert_clean_exit(&output);
}

#[test]
fn codebase_for_human_still_writes_its_file_when_stdout_is_closed() {
    let dir = tempdir().expect("create tempdir");
    fs::write(dir.path().join("readme.txt"), b"hello\n").expect("write readme.txt");

    let output = run_with_closed_stdout(&["codebase", "--for-human"], dir.path());

    assert_clean_exit(&output);
    assert!(dir.path().join("CODEBASE.md").is_file());
}
