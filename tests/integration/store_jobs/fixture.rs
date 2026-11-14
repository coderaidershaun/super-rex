//! The world each job test runs in: real Git repositories with a store, the
//! `rex` binary run in them with none of the session's variables inherited, and
//! the job commands the tests share.

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

pub const SESSION: &str = "4f9c2e1a-7b3d-4e8f-9a6c-1d2e3f4a5b6c";

pub const COMMIT: &str = "3f2a9c1e8b7d6a5f4e3d2c1b0a9f8e7d6c5b4a39";

/// Turns storage on in `repo`, as `rex init --with-storage` does.
pub fn storage_on(repo: &Path) {
    fs::write(repo.join(".rex/config.json"), r#"{"storage": "on"}"#).unwrap();
}

/// One event of the plugin, for [`ingest`] to number.
pub fn event(
    kind: &str,
    at: &str,
    job: Option<&str>,
    data: serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({ "at": at, "kind": kind, "agent": "main", "job": job, "data": data })
}

/// Sends `events` of `session` through `rex ingest` in `cwd`, numbered from
/// `first`, and gives the last sequence number it printed.
pub fn ingest(cwd: &Path, session: &str, first: usize, events: &[serde_json::Value]) -> String {
    let numbered: Vec<serde_json::Value> = events
        .iter()
        .enumerate()
        .map(|(index, event)| {
            let mut event = event.clone();
            event["seq"] = serde_json::json!(first + index);
            event
        })
        .collect();
    let batch = serde_json::json!({ "version": 1, "session": session, "stream": "aaaaaaaaaaaaaaaa",
                                     "secret": "1".repeat(64), "events": numbered });
    stdout_of(rex(cwd, &["ingest"]).write_stdin(batch.to_string()))
}

/// The `data` of a job's opening, owned by the main loop of `session`.
pub fn job_open(session: &str, builder: &str, repo: &Path, job: &str) -> serde_json::Value {
    serde_json::json!({
        "owner": { "session": session, "builder": builder }, "origin": "composer",
        "request": "A request.", "restatement": null, "repo": repo, "branch": format!("rex/{job}"),
        "base": "main", "worktree": null, "type": "fix", "size": "medium", "consequence": "high",
        "langs": ["rust"], "prefs": { "duration": null, "budget": null, "autonomy": null },
        "limit_nano": 32_000_000_000_u64, "limit_source": "default", "tree": null,
        "depends_on": [], "merge_after": []
    })
}

pub fn show_json(cwd: &Path, job: &str) -> serde_json::Value {
    serde_json::from_str(&stdout_of(&mut rex(cwd, &["job", "show", job, "--json"]))).unwrap()
}
