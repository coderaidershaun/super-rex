//! `rex hook` run the way Claude Code runs it: one event on standard input, from
//! a Git worktree, a sub-folder, or a folder outside the repo.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use assert_cmd::Command;
use serde_json::{Value, json};
use super_rex::store::{Store, StorePaths};
use tempfile::TempDir;

use super::fixture;

/// A main checkout with a store, a sub-folder, and a linked worktree laid out by
/// hand the way `git worktree add` lays it out.
struct Repo {
    _folder: TempDir,
    root: PathBuf,
    worktree: PathBuf,
}

impl Repo {
    fn new() -> Repo {
        let folder = tempfile::tempdir().unwrap();
        let root = folder.path().canonicalize().unwrap().join("repo");
        let worktree_git_dir = root.join(".git/worktrees/agent-a1");
        fs::create_dir_all(&worktree_git_dir).unwrap();
        fs::write(worktree_git_dir.join("commondir"), "../..\n").unwrap();
        let worktree = root.join(".claude/worktrees/agent-a1");
        fs::create_dir_all(worktree.join("src")).unwrap();
        fs::write(
            worktree.join(".git"),
            format!("gitdir: {}\n", worktree_git_dir.display()),
        )
        .unwrap();
        fs::create_dir_all(root.join("sub")).unwrap();
        Store::create(&StorePaths::locate(&root).unwrap()).unwrap();
        Repo {
            _folder: folder,
            root,
            worktree,
        }
    }

    fn log(&self) -> PathBuf {
        self.root.join(".rex/events.jsonl")
    }

    fn log_text(&self) -> String {
        fs::read_to_string(self.log()).unwrap_or_default()
    }
}

/// `rex hook` in `cwd`, with the two variables that steer the store search removed,
/// so the test machine's own setup cannot change the result.
fn rex_hook(cwd: &Path, input: impl Into<Vec<u8>>) -> Command {
    let mut command = Command::cargo_bin("rex").unwrap();
    command
        .arg("hook")
        .current_dir(cwd)
        .env_remove("REX_STORE")
        .env_remove("CLAUDE_PROJECT_DIR")
        .write_stdin(input);
    command
}

fn recorded_event() -> Value {
    let log = fs::read_to_string(fixture("tree/events.jsonl")).unwrap();
    let first_line: Value = serde_json::from_str(log.lines().next().unwrap()).unwrap();
    first_line["event"].clone()
}

fn now_ms() -> u64 {
    let since_epoch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    u64::try_from(since_epoch.as_millis()).unwrap()
}

/// The one line of `log_text` after `before`, checked to be the C3 form of `event`
/// with a time between `earliest` and `latest`.
fn assert_one_line_of(log_text: &str, before: &str, event: &Value, earliest: u64, latest: u64) {
    let added = log_text
        .strip_prefix(before)
        .unwrap_or_else(|| panic!("the log lost what it held: {log_text}"));
    let line: Value = serde_json::from_str(added).unwrap();
    let at_ms = line["at_ms"].as_u64().unwrap();
    assert!(
        (earliest..=latest).contains(&at_ms),
        "at_ms {at_ms} is not between {earliest} and {latest}"
    );
    let expected = format!(
        "{{\"v\":1,\"at_ms\":{at_ms},\"event\":{}}}\n",
        serde_json::to_string(event).unwrap()
    );
    assert_eq!(added, expected);
}

#[test]
fn hook_appends_one_line_from_a_worktree() {
    let repo = Repo::new();
    let event = recorded_event();
    let input = format!("{}\n", serde_json::to_string(&event).unwrap());

    let earliest = now_ms();
    rex_hook(&repo.worktree.join("src"), input.clone())
        .assert()
        .success();
    let after_first = repo.log_text();
    assert_one_line_of(&after_first, "", &event, earliest, now_ms());

    let outside = tempfile::tempdir().unwrap();
    let earliest = now_ms();
    rex_hook(outside.path(), input)
        .env("CLAUDE_PROJECT_DIR", repo.root.join("sub"))
        .assert()
        .success();
    assert_one_line_of(&repo.log_text(), &after_first, &event, earliest, now_ms());
}

/// Run `command` and check what Claude Code needs of every hook: exit code 0 and
/// nothing on standard output or standard error.
fn assert_silent_success(command: &mut Command) {
    let output = command.output().unwrap();
    assert_eq!(output.status.code(), Some(0), "exit code");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "",
        "standard output"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "",
        "standard error"
    );
}

#[test]
fn hook_never_fails_or_prints() {
    let repo = Repo::new();
    assert_silent_success(&mut rex_hook(&repo.root, ""));
    assert!(!repo.log().exists(), "empty input made the log");

    let earliest = now_ms();
    assert_silent_success(&mut rex_hook(&repo.root, "not json\n"));
    let line: Value = serde_json::from_str(&repo.log_text()).unwrap();
    let at_ms = line["at_ms"].as_u64().unwrap();
    assert!((earliest..=now_ms()).contains(&at_ms), "at_ms {at_ms}");
    assert_eq!(
        repo.log_text(),
        format!("{{\"v\":1,\"at_ms\":{at_ms},\"raw\":\"not json\"}}\n")
    );

    let no_store = tempfile::tempdir().unwrap();
    fs::create_dir(no_store.path().join(".git")).unwrap();
    assert_silent_success(&mut rex_hook(no_store.path(), "{\"a\":1}\n"));
    assert!(
        !no_store.path().join(".rex").exists(),
        "the hook made a folder"
    );

    let unwritable = Repo::new();
    fs::create_dir(unwritable.log()).unwrap();
    assert_silent_success(&mut rex_hook(&unwritable.root, "{\"a\":1}\n"));
}

#[test]
fn hook_lines_never_mix() {
    const HOOKS: usize = 32;
    let repo = Arc::new(Repo::new());
    let events: Vec<Value> = (0..HOOKS)
        .map(|n| json!({ "hook": n, "filler": "x".repeat(200_000 + n) }))
        .collect();

    let start = Arc::new(Barrier::new(HOOKS));
    let hooks: Vec<_> = events
        .iter()
        .map(|event| {
            let repo = Arc::clone(&repo);
            let start = Arc::clone(&start);
            let input = format!("{event}\n");
            thread::spawn(move || {
                start.wait();
                rex_hook(&repo.root, input).output().unwrap().status
            })
        })
        .collect();
    for hook in hooks {
        assert!(hook.join().unwrap().success());
    }

    let log_text = repo.log_text();
    let mut logged: Vec<Value> = log_text
        .lines()
        .map(|line| {
            let line: Value = serde_json::from_str(line)
                .unwrap_or_else(|error| panic!("a line does not parse: {error}"));
            line["event"].clone()
        })
        .collect();
    assert_eq!(logged.len(), HOOKS, "lines in the log");
    logged.sort_by_key(|event| event["hook"].as_u64());
    assert_eq!(logged, events);
}
