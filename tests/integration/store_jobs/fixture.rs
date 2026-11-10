//! The world each job test runs in: real Git repositories with a store, and the
//! `rex` binary run in them with none of the session's variables inherited.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as Process;

use assert_cmd::Command;
use super_rex::store::{Store, StorePaths};

pub fn git(dir: &Path, args: &[&str]) {
    let status = Process::new("git")
        .args(["-c", "user.name=rex", "-c", "user.email=rex@example.com"])
        .args([
            "-c",
            "commit.gpgsign=false",
            "-c",
            "init.defaultBranch=main",
        ])
        .args(args)
        .current_dir(dir)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_INDEX_FILE")
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?} failed in {}", dir.display());
}

/// A new repository with one commit, as its canonical path.
pub fn new_repo(parent: &Path, name: &str) -> PathBuf {
    let repo = parent.join(name);
    fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q"]);
    git(
        &repo,
        &[
            "commit",
            "-q",
            "--allow-empty",
            "--no-verify",
            "-m",
            "start",
        ],
    );
    repo.canonicalize().unwrap()
}

/// A worktree of `repo` on a new branch, as its canonical path.
pub fn new_worktree(repo: &Path, parent: &Path, name: &str) -> PathBuf {
    let worktree = parent.join(name);
    let path = worktree.to_str().unwrap();
    git(repo, &["worktree", "add", "-q", "-b", name, path]);
    worktree.canonicalize().unwrap()
}

/// A store in `repo` with one orchestrator in its library.
pub fn new_store(repo: &Path, orchestrator: &str) -> Store {
    let paths = StorePaths::locate(repo).unwrap();
    let store = Store::create(&paths).unwrap();
    let folder = paths.orchestrators.join(orchestrator);
    fs::create_dir_all(&folder).unwrap();
    fs::write(
        folder.join("SKILL.md"),
        "---\nname: pipeline\ndescription: Plan, build and review.\n---\n\n# Pipeline\n",
    )
    .unwrap();
    store
}

/// `rex <args>` run in `cwd`, with none of the variables that pick a store, a
/// session or an effort, so each case sets only what it needs.
pub fn rex(cwd: &Path, args: &[&str]) -> Command {
    let mut command = Command::cargo_bin("rex").unwrap();
    command
        .current_dir(cwd)
        .env_remove("REX_STORE")
        .env_remove("CLAUDE_PROJECT_DIR")
        .env_remove("CLAUDE_CODE_SESSION_ID")
        .env_remove("CLAUDE_EFFORT")
        .args(args);
    command
}

/// Runs `command`, checks it succeeded, and gives its standard output.
pub fn stdout_of(command: &mut Command) -> String {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{command:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

/// Runs `command`, checks that it exits with `code` and prints nothing on
/// standard output, and gives its standard error.
pub fn stderr_of(command: &mut Command, code: i32) -> String {
    let output = command.output().unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(output.status.code(), Some(code), "{command:?}: {stderr}");
    assert!(output.stdout.is_empty(), "{command:?} printed on stdout");
    stderr
}

/// The value of the first `key=value` line for `key`.
pub fn value<'a>(stdout: &'a str, key: &str) -> &'a str {
    stdout
        .lines()
        .find_map(|line| line.strip_prefix(key)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("no {key}= line in:\n{stdout}"))
}
