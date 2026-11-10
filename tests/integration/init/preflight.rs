//! The checks `rex init` makes around its writes: the Claude Code version it
//! needs before it writes anything, and the sandbox tools it needs on Linux.

use std::fs;

use super::support::{Repo, SKILL_MARKER, installed, repo, rex_init, rex_init_failed};

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
        "Claude Code 2.1.300 is older than 2.1.322, the oldest version SuperRex works with; run claude update, then run rex init again",
    );

    // As text, 2.1.1000 sorts before 2.1.322. As numbers it is newer.
    let newer = repo();
    newer.write_tool("claude", "echo '2.1.1000 (Claude Code)'");
    let output = rex_init(&newer, &[]);
    assert!(output.status.success(), "a newer Claude Code must pass");
    assert!(installed(&newer, SKILL_MARKER).is_file());
}

#[test]
fn linux_sandbox_tools_missing_fails_after_the_install() {
    let cases = [
        (&["bwrap"][..], "bwrap"),
        (&["bwrap", "socat"][..], "bwrap and socat"),
    ];
    for (removed, named) in cases {
        let repo = repo();
        for tool in removed {
            repo.remove_tool(tool);
        }

        let output = rex_init(&repo, &[]);

        assert!(
            installed(&repo, SKILL_MARKER).is_file(),
            "the install runs in full"
        );
        let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
        if cfg!(target_os = "linux") {
            assert_eq!(output.status.code(), Some(1), "got: {stderr}");
            assert_eq!(
                stderr,
                format!(
                    "rex init: failed: the Claude Code sandbox cannot start on this machine: {named} not found on PATH, so Claude Code will refuse to start in this repo until they are installed; install them with sudo apt-get install bubblewrap socat (Ubuntu or Debian), or sudo dnf install bubblewrap socat (Fedora)\n\
                     error: rex init did not finish 1 step(s); each is named above\n"
                )
            );
        } else {
            assert!(output.status.success(), "no sandbox check here: {stderr}");
            assert_eq!(stderr, "");
        }
    }
}
