//! The checks `rex init` makes before it writes anything: the Claude Code
//! version it needs.

use std::fs;

use super::support::{Repo, repo, rex_init, rex_init_failed};

/// The names in the top folder of `repo`, sorted.
fn names_in(repo: &Repo) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(repo.path())
        .expect("read repo folder")
        .map(|entry| {
            entry
                .expect("read repo entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

/// Run `rex init` in `repo`, and check that it stopped with `error` as its one
/// line on stderr and wrote nothing.
fn assert_stopped_with(repo: &Repo, error: &str) {
    let printed = rex_init_failed(repo, &[]);

    assert_eq!(printed.stderr, format!("error: {error}\n"));
    assert_eq!(printed.stdout, "");
    assert_eq!(names_in(repo), [".git"], "nothing is written");
}

#[test]
fn claude_missing_or_too_old_stops_before_any_write() {
    let missing = repo();
    missing.remove_tool("claude");
    assert_stopped_with(
        &missing,
        "Claude Code was not found: claude is not on PATH; install Claude Code, then run rex init again",
    );

    let too_old = repo();
    too_old.write_tool("claude", "echo '2.1.300 (Claude Code)'");
    assert_stopped_with(
        &too_old,
        "Claude Code 2.1.300 is older than 2.1.323, the oldest version SuperRex works with; run claude update, then run rex init again",
    );

    // As text, 2.1.1000 sorts before 2.1.323. As numbers it is newer.
    let newer = repo();
    newer.write_claude("2.1.1000");
    let output = rex_init(&newer, &[]);
    assert!(output.status.success(), "a newer Claude Code must pass");
    assert!(newer.path().join(".rex/config.json").is_file());
}
