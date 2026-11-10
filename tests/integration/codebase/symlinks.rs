//! Integration test of `rex codebase --for-human` when `CODEBASE.md` is a symbolic link.

#![cfg(unix)]

use std::fs;
use std::os::unix::fs::symlink;

use assert_cmd::Command;
use tempfile::tempdir;

// Writing CODEBASE.md is all `rex codebase --for-human` does, so a link it
// must not write through has to fail. Exiting 0 would tell a script the file
// was written.
#[test]
fn codebase_for_human_fails_on_a_symlinked_output_and_writes_nothing_outside() {
    let dir = tempdir().expect("create tempdir");
    let project = dir.path().join("project");
    fs::create_dir_all(&project).expect("create project");
    let outside = dir.path().join("outside-codebase.md");
    symlink(&outside, project.join("CODEBASE.md")).expect("create dangling symlink");

    let assert = Command::cargo_bin("rex")
        .expect("find rex binary")
        .args(["codebase", "--for-human"])
        .current_dir(&project)
        .assert()
        .failure();

    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("utf8 stderr");
    assert!(
        stderr.contains("./CODEBASE.md is a symbolic link; rex does not write through one"),
        "got: {stderr}"
    );
    assert!(
        !outside.exists(),
        "rex created a file where the link points"
    );
}
