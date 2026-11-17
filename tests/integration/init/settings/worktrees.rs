//! The job worktrees in `.claude/settings.local.json`: `rex init` lists each
//! one that exists under `<repo>.worktrees/`, and no other, and takes out the
//! entry of one that is gone, while a folder of the person's keeps its own.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Value, json};

use super::expected::{parent_of, read_settings, write_settings};
use crate::init::support::{Repo, repo, rex_init_ok};

const GIT_LOCATION_VARIABLES: [&str; 4] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_COMMON_DIR",
    "GIT_INDEX_FILE",
];

/// Run git in `dir`, as the test process does: `rex init` itself never needs it.
fn git(dir: &Path, args: &[&str]) {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.email=t@example.invalid", "-c", "user.name=t"])
        .args(args);
    for variable in GIT_LOCATION_VARIABLES {
        command.env_remove(variable);
    }
    assert!(command.status().expect("run git").success(), "git {args:?}");
}

/// Give `repo` a first commit, and a linked worktree at each of `dirs`.
fn add_worktrees(repo: &Repo, dirs: &[&Path]) {
    fs::write(repo.path().join("a"), "a\n").expect("write a file");
    git(repo.path(), &["add", "a"]);
    git(repo.path(), &["commit", "-q", "-m", "first"]);
    for (index, dir) in dirs.iter().enumerate() {
        let dir = dir.to_str().expect("utf8 path");
        git(
            repo.path(),
            &["worktree", "add", "-q", "-b", &format!("w{index}"), dir],
        );
    }
}

/// `<parent>/<name>.worktrees`, the folder of the job worktrees of `repo`,
/// removed when the test ends: it sits beside the repo's temp folder, not in it.
struct WorktreesRoot(PathBuf);

impl Drop for WorktreesRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

impl std::ops::Deref for WorktreesRoot {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.0
    }
}

fn worktrees_root(repo: &Repo) -> WorktreesRoot {
    let canonical = repo.path().canonicalize().expect("canonical repo");
    let name = canonical
        .file_name()
        .expect("a name")
        .to_str()
        .expect("utf8");
    WorktreesRoot(canonical.with_file_name(format!("{name}.worktrees")))
}

fn canonical(path: &Path) -> String {
    path.canonicalize()
        .expect("canonical path")
        .to_str()
        .expect("utf8")
        .to_string()
}

fn written(repo: &Repo) -> Value {
    serde_json::from_str(&read_settings(repo)).expect("settings JSON")
}

#[test]
fn the_worktrees_of_the_repo_are_listed_and_no_other() {
    let repo = repo();
    let root = worktrees_root(&repo);
    let job = root.join("0gjob1");
    let unit = root.join("0gjob1--lexer");
    let other = tempfile::tempdir().expect("other folder");
    let elsewhere = other.path().join("wt");
    add_worktrees(&repo, &[&job, &unit, &elsewhere]);

    let printed = rex_init_ok(&repo, &[]);

    assert_eq!(printed.stderr, "");
    assert_eq!(
        written(&repo)["permissions"]["additionalDirectories"],
        json!([parent_of(&repo), canonical(&job), canonical(&unit)])
    );

    // A second run finds nothing to change.
    let before = read_settings(&repo);

    let printed = rex_init_ok(&repo, &[]);

    assert_eq!(printed.stderr, "");
    assert!(
        !printed.stdout.contains("settings.local.json"),
        "got: {}",
        printed.stdout
    );
    assert_eq!(read_settings(&repo), before);
}

#[test]
fn a_gone_worktree_loses_its_entry_and_a_folder_of_the_person_keeps_its_own() {
    let repo = repo();
    let root = worktrees_root(&repo);
    let live = root.join("0gjob1");
    add_worktrees(&repo, &[&live]);
    let gone = format!("{}/0gjob0", root.display());
    let theirs = "/nowhere/of/theirs";
    write_settings(
        &repo,
        &format!(
            "{:#}\n",
            json!({ "permissions": { "additionalDirectories": [theirs, gone, canonical(&live)] } })
        ),
    );

    let printed = rex_init_ok(&repo, &[]);

    assert_eq!(
        printed.stderr,
        format!(
            "rex init: removed \"{gone}\" from permissions.additionalDirectories in .claude/settings.local.json: it is no longer a worktree of this repo.\n"
        )
    );
    assert_eq!(
        written(&repo)["permissions"]["additionalDirectories"],
        json!([theirs, canonical(&live), parent_of(&repo)])
    );
}
