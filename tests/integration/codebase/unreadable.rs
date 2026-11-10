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

// One tree with a locked directory, a locked `.rs` file and a locked `.go`
// file beside readable ones. Both commands must go on, list what they can
// read, and name each path they skipped on stderr.
#[test]
fn codebase_skips_an_unreadable_directory_and_warns() {
    let dir = tempdir().expect("create tempdir");
    fs::create_dir_all(dir.path().join("ok")).expect("create ok dir");
    fs::write(
        dir.path().join("ok/a.rs"),
        "//! Fine module. More.\n// SMELL: bad name\nfn a() {}\n",
    )
    .expect("write ok/a.rs");
    fs::write(dir.path().join("ok.go"), "// Fine feed.\n\npackage feed\n").expect("write ok.go");
    let Some(_dir) = Locked::dir(dir.path().join("locked")) else {
        return;
    };
    let Some(_rs) = Locked::file(
        dir.path().join("locked.rs"),
        "//! Hidden module. More.\n// SMELL: hidden\n",
    ) else {
        return;
    };
    let Some(_go) = Locked::file(
        dir.path().join("locked.go"),
        "// Hidden feed.\n\npackage feed\n",
    ) else {
        return;
    };

    let run = |args: &[&str], skipped: &[&str], expected: &str| {
        let assert = Command::cargo_bin("rex")
            .expect("find rex binary")
            .args(args)
            .current_dir(dir.path())
            .assert()
            .success();
        let output = assert.get_output();
        let stdout = String::from_utf8(output.stdout.clone()).expect("utf8 stdout");
        let stderr = String::from_utf8(output.stderr.clone()).expect("utf8 stderr");
        assert_eq!(stdout, expected, "rex {args:?}");
        for path in skipped {
            assert!(
                stderr.contains(&format!("rex: skipped ./{path}: permission denied")),
                "rex {args:?} must name the skipped {path}, got: {stderr}"
            );
        }
    };

    run(
        &["codebase", "--with-context"],
        &["locked", "locked.rs", "locked.go"],
        "locked/\nlocked.go\nlocked.rs\nok/\n  a.rs - Fine module.\nok.go - Fine feed.\n",
    );
    run(
        &["smells"],
        &["locked", "locked.rs"],
        "ok/a.rs:2\nCode smells: 1\n",
    );
}
