//! The checks `rex init` makes before it writes anything: where it runs, the
//! Claude Code version, and the tools it looks for on PATH.

use std::fs;
use std::path::PathBuf;

use super::support::{Printed, Repo, SKILL_MARKER, installed, repo, rex_init};

/// Run `rex init` in `repo` and check that it exited with 1.
fn rex_init_failed(repo: &Repo) -> Printed {
    let output = rex_init(repo, &[]);
    let printed = Printed {
        stdout: String::from_utf8(output.stdout).expect("utf8 stdout"),
        stderr: String::from_utf8(output.stderr).expect("utf8 stderr"),
    };
    assert_eq!(
        output.status.code(),
        Some(1),
        "rex init must exit with 1; stderr: {}",
        printed.stderr
    );
    printed
}

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

/// The repo folder as `rex init` names it in its messages.
fn canonical(repo: &Repo) -> PathBuf {
    repo.path().canonicalize().expect("canonical repo path")
}

#[test]
fn outside_a_repo_root_nothing_is_written() {
    let repo = repo();
    fs::remove_dir_all(repo.path().join(".git")).expect("remove .git directory");

    let printed = rex_init_failed(&repo);

    assert_eq!(
        printed.stderr,
        format!(
            "error: {} is not the root of a Git repository; run rex init in the folder that holds .git, or run git init first\n",
            canonical(&repo).display()
        )
    );
    assert_eq!(printed.stdout, "");
    assert_eq!(names_in(&repo), Vec::<String>::new(), "nothing is written");
}

/// Run `rex init` in `repo`, and check that it stopped with `error` as its one
/// line on stderr and wrote nothing.
fn assert_stopped_with(repo: &Repo, error: &str) {
    let printed = rex_init_failed(repo);

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

    let not_runnable = repo();
    fs::write(not_runnable.bin().join("claude"), "not a program\n").expect("write claude");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(
            not_runnable.bin().join("claude"),
            fs::Permissions::from_mode(0o644),
        )
        .expect("chmod");
    }
    let printed = rex_init_failed(&not_runnable);
    let could_not_run = "error: could not run \"claude --version\"; check the Claude Code install, then run rex init again: ";
    assert!(
        printed.stderr.starts_with(could_not_run) && printed.stderr.lines().count() == 1,
        "got: {}",
        printed.stderr
    );
    assert_eq!(names_in(&not_runnable), [".git"], "nothing is written");

    let too_old = repo();
    too_old.write_tool("claude", "echo '2.1.300 (Claude Code)'");
    assert_stopped_with(
        &too_old,
        "Claude Code 2.1.300 is older than 2.1.322, the oldest version SuperRex works with; run claude update, then run rex init again",
    );

    let garbage = repo();
    garbage.write_tool("claude", "echo garbage");
    assert_stopped_with(
        &garbage,
        "could not read the Claude Code version from \"claude --version\" (it printed: garbage); run claude update, then run rex init again",
    );

    let silent = repo();
    silent.write_tool("claude", "exit 0");
    assert_stopped_with(
        &silent,
        "could not read the Claude Code version from \"claude --version\" (it printed: nothing); run claude update, then run rex init again",
    );

    // As text, 2.1.1000 sorts before 2.1.322. As numbers it is newer.
    let newer = repo();
    newer.write_tool("claude", "echo '2.1.1000 (Claude Code)'");
    let output = rex_init(&newer, &[]);
    assert!(output.status.success(), "a newer Claude Code must pass");
    assert!(installed(&newer, SKILL_MARKER).is_file());
}

#[test]
fn gh_missing_is_a_warning() {
    let repo = repo();
    repo.remove_tool("gh");

    let output = rex_init(&repo, &[]);

    assert!(
        output.status.success(),
        "a missing gh must not stop rex init"
    );
    assert_eq!(
        String::from_utf8(output.stderr).expect("utf8 stderr"),
        "rex init: warning: gh not found on PATH. Delivery works without pull requests until it is installed.\n"
    );
    assert!(installed(&repo, SKILL_MARKER).is_file());
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
