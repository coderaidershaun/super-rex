//! Integration tests of what `rex codebase` and `rex smells` do with paths they cannot read.

#![cfg(unix)]

use std::fs::{self, Permissions};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use assert_cmd::Command;
use tempfile::tempdir;

/// A path that nobody can read. It is made readable again when dropped, so
/// the temp dir can be removed even if the test fails.
struct Locked(PathBuf);

impl Locked {
    /// `None` when the lock has no effect, as when the tests run as root.
    fn dir(path: PathBuf) -> Option<Self> {
        fs::create_dir_all(path.join("inner")).expect("create locked dir");
        fs::set_permissions(&path, Permissions::from_mode(0o000)).expect("lock dir");
        let locked = Self(path);
        fs::read_dir(&locked.0).is_err().then_some(locked)
    }

    /// `None` when the lock has no effect, as when the tests run as root.
    fn file(path: PathBuf, contents: &str) -> Option<Self> {
        fs::write(&path, contents).expect("write locked file");
        fs::set_permissions(&path, Permissions::from_mode(0o000)).expect("lock file");
        let locked = Self(path);
        fs::read(&locked.0).is_err().then_some(locked)
    }
}

impl Drop for Locked {
    fn drop(&mut self) {
        let _ = fs::set_permissions(&self.0, Permissions::from_mode(0o755));
    }
}

#[test]
fn codebase_skips_an_unreadable_directory_and_warns() {
    let dir = tempdir().expect("create tempdir");
    fs::create_dir_all(dir.path().join("ok")).expect("create ok dir");
    fs::write(dir.path().join("ok/a.rs"), "fn a() {}\n").expect("write ok/a.rs");
    let Some(_locked) = Locked::dir(dir.path().join("locked")) else {
        return;
    };

    let assert = Command::cargo_bin("rex")
        .expect("find rex binary")
        .arg("codebase")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = assert.get_output();
    let stdout = String::from_utf8(output.stdout.clone()).expect("utf8 stdout");
    let stderr = String::from_utf8(output.stderr.clone()).expect("utf8 stderr");
    assert_eq!(stdout, "locked/\nok/\n  a.rs\n");
    assert!(
        stderr.contains("rex: skipped ./locked: permission denied"),
        "warning must name the skipped path, got: {stderr}"
    );
}

#[test]
fn smells_still_counts_flags_in_readable_files_and_warns() {
    let dir = tempdir().expect("create tempdir");
    fs::write(dir.path().join("a.rs"), "// SMELL: bad name\nfn a() {}\n").expect("write a.rs");
    let Some(_locked) = Locked::dir(dir.path().join("locked")) else {
        return;
    };

    let assert = Command::cargo_bin("rex")
        .expect("find rex binary")
        .arg("smells")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = assert.get_output();
    let stdout = String::from_utf8(output.stdout.clone()).expect("utf8 stdout");
    let stderr = String::from_utf8(output.stderr.clone()).expect("utf8 stderr");
    assert_eq!(stdout, "a.rs:1\nCode smells: 1\n");
    assert!(
        stderr.contains("rex: skipped ./locked: permission denied"),
        "warning must name the skipped path, got: {stderr}"
    );
}

#[test]
fn smells_skips_an_unreadable_rust_file_and_warns() {
    let dir = tempdir().expect("create tempdir");
    fs::write(dir.path().join("ok.rs"), "// SMELL: bad name\nfn a() {}\n").expect("write ok.rs");
    let Some(_locked) = Locked::file(dir.path().join("locked.rs"), "// SMELL: hidden\n") else {
        return;
    };

    let assert = Command::cargo_bin("rex")
        .expect("find rex binary")
        .arg("smells")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = assert.get_output();
    let stdout = String::from_utf8(output.stdout.clone()).expect("utf8 stdout");
    let stderr = String::from_utf8(output.stderr.clone()).expect("utf8 stderr");
    assert_eq!(stdout, "ok.rs:1\nCode smells: 1\n");
    assert!(
        stderr.contains("rex: skipped ./locked.rs: permission denied"),
        "warning must name the skipped file, got: {stderr}"
    );
}

#[test]
fn codebase_with_context_lists_an_unreadable_rust_file_without_its_sentence() {
    let dir = tempdir().expect("create tempdir");
    fs::write(
        dir.path().join("ok.rs"),
        "//! Fine module. More.\nfn a() {}\n",
    )
    .expect("write ok.rs");
    let Some(_locked) = Locked::file(dir.path().join("locked.rs"), "//! Hidden module. More.\n")
    else {
        return;
    };

    let assert = Command::cargo_bin("rex")
        .expect("find rex binary")
        .args(["codebase", "--with-context"])
        .current_dir(dir.path())
        .assert()
        .success();

    let output = assert.get_output();
    let stdout = String::from_utf8(output.stdout.clone()).expect("utf8 stdout");
    let stderr = String::from_utf8(output.stderr.clone()).expect("utf8 stderr");
    assert_eq!(stdout, "locked.rs\nok.rs - Fine module.\n");
    assert!(
        stderr.contains("rex: skipped ./locked.rs: permission denied"),
        "warning must name the skipped file, got: {stderr}"
    );
}

#[test]
fn codebase_with_context_lists_an_unreadable_go_file_without_its_sentence() {
    let dir = tempdir().expect("create tempdir");
    fs::write(dir.path().join("ok.go"), "// Fine feed.\n\npackage feed\n").expect("write ok.go");
    let Some(_locked) = Locked::file(
        dir.path().join("locked.go"),
        "// Hidden feed.\n\npackage feed\n",
    ) else {
        return;
    };

    let assert = Command::cargo_bin("rex")
        .expect("find rex binary")
        .args(["codebase", "--with-context"])
        .current_dir(dir.path())
        .assert()
        .success();

    let output = assert.get_output();
    let stdout = String::from_utf8(output.stdout.clone()).expect("utf8 stdout");
    let stderr = String::from_utf8(output.stderr.clone()).expect("utf8 stderr");
    assert_eq!(stdout, "locked.go\nok.go - Fine feed.\n");
    assert!(
        stderr.contains("rex: skipped ./locked.go: permission denied"),
        "warning must name the skipped file, got: {stderr}"
    );
}
