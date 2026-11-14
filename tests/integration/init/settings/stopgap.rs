//! One `rex init` moves a repo off the stopgap that ran every git command
//! outside the sandbox: it takes those entries out, keeps the person's others,
//! and registers the job worktrees that already exist, with no failure.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Value, json};

use super::expected::{OLD_HOOKS, PLUGIN_DENY_RULES, parent_of, read_settings, write_settings};
use crate::init::support::{Repo, repo, rex_init_ok};

/// The settings file a person copied in while git could not commit in a
/// worktree, word for word but for `PARENT`, the folder that holds the repo.
const STOPGAP: &str = r#"{
  "sandbox": {
    "enabled": true,
    "failIfUnavailable": false,
    "allowUnsandboxedCommands": true,
    "enableWeakerNetworkIsolation": true,
    "network": {
      "allowLocalBinding": true
    },
    "filesystem": {
      "allowWrite": [
        "~/.cargo/registry",
        "~/.cargo/git",
        "~/.cargo/.package-cache",
        "~/.cargo/.package-cache-mutate",
        "~/.cargo/.global-cache",
        "~/.cache/pre-commit/patch*",
        "~/Library/Caches/trivy"
      ],
      "denyRead": [
        "~/.aws"
      ]
    },
    "excludedCommands": [
      "git *",
      "rex init*"
    ]
  },
  "permissions": {
    "additionalDirectories": [
      "PARENT"
    ],
    "blockWritesOutsideWorkingDirectories": true,
    "deny": [
      "Bash(sudo *)",
      "Bash(git push --force*)",
      "Bash(cargo publish*)",
      "Bash(gh pr merge*--admin*)",
      "Bash(git push -f*)",
      "Bash(git push * --force*)",
      "Bash(git push * -f*)",
      "Bash(git push *+*)"
    ],
    "allow": [
      "Bash(git *)",
      "Bash(cd PARENT/*)",
      "Bash(rex test-filter)",
      "Bash(set -o pipefail)"
    ]
  },
  "hooks": HOOKS
}
"#;

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

fn stopgap(repo: &Repo) -> String {
    STOPGAP
        .replace("PARENT", &parent_of(repo))
        .replace("HOOKS", OLD_HOOKS)
}

fn written(repo: &Repo) -> Value {
    serde_json::from_str(&read_settings(repo)).expect("settings JSON")
}

#[test]
fn one_run_takes_the_stopgap_out_and_keeps_the_rest() {
    let repo = repo();
    let root = worktrees_root(&repo);
    let job = root.join("0gjob1");
    let unit = root.join("0gjob1--lexer");
    let other = tempfile::tempdir().expect("other folder");
    let elsewhere = other.path().join("wt");
    add_worktrees(&repo, &[&job, &unit, &elsewhere]);
    write_settings(&repo, &stopgap(&repo));

    let printed = rex_init_ok(&repo, &[]);

    let parent = parent_of(&repo);
    assert_eq!(
        printed.stderr,
        format!(
            "rex init: removed \"git *\" from sandbox.excludedCommands in .claude/settings.local.json: it ran git and its hooks outside the sandbox, and rex now registers its worktrees, so git works inside it.\n\
             rex init: removed \"Bash(git *)\" from permissions.allow in .claude/settings.local.json: it ran git and its hooks outside the sandbox, and rex now registers its worktrees, so git works inside it.\n\
             rex init: removed \"Bash(cd {parent}/*)\" from permissions.allow in .claude/settings.local.json: it ran git and its hooks outside the sandbox, and rex now registers its worktrees, so git works inside it.\n"
        )
    );
    assert!(
        printed
            .stdout
            .starts_with("rex init: .claude/settings.local.json updated\n"),
        "got: {}",
        printed.stdout
    );
    let settings = written(&repo);
    assert_eq!(
        settings["sandbox"]["excludedCommands"],
        json!([
            "rex init*",
            "git worktree *",
            "git merge *",
            "git pull *",
            "git switch *",
            "rex worktree *"
        ])
    );
    assert_eq!(
        settings["permissions"]["allow"],
        json!([
            "Bash(rex test-filter)",
            "Bash(set -o pipefail)",
            "Bash(rex worktree add *)",
            "Bash(rex worktree remove *)"
        ])
    );
    let mut deny = vec![
        "Bash(sudo *)",
        "Bash(git push --force*)",
        "Bash(cargo publish*)",
        "Bash(gh pr merge*--admin*)",
        "Bash(git push -f*)",
        "Bash(git push * --force*)",
        "Bash(git push * -f*)",
        "Bash(git push *+*)",
    ];
    deny.extend(PLUGIN_DENY_RULES);
    assert_eq!(settings["permissions"]["deny"], json!(deny));
    // The worktrees under <repo>.worktrees/ are registered, and no other.
    assert_eq!(
        settings["permissions"]["additionalDirectories"],
        json!([parent, canonical(&job), canonical(&unit)])
    );
    assert_eq!(settings.get("hooks"), None, "rex 0.7's hooks are taken out");

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

#[test]
fn strict_takes_the_stopgap_out_and_adds_no_command_outside_the_sandbox() {
    let repo = repo();
    write_settings(
        &repo,
        r#"{
  "sandbox": { "failIfUnavailable": true, "allowUnsandboxedCommands": false, "excludedCommands": ["git:*"] },
  "permissions": { "allow": ["Bash(git:*)", "Bash(git log *)", "Bash(git status)"] }
}
"#,
    );

    let printed = rex_init_ok(&repo, &["--strict"]);

    assert!(
        printed
            .stderr
            .contains("removed \"git:*\" from sandbox.excludedCommands"),
        "got: {}",
        printed.stderr
    );
    assert!(
        printed
            .stderr
            .contains("removed \"Bash(git:*)\" from permissions.allow"),
        "got: {}",
        printed.stderr
    );
    let settings = written(&repo);
    assert_eq!(settings["sandbox"]["excludedCommands"], json!([]));
    // A narrower git rule of the person's is theirs to keep.
    assert_eq!(
        settings["permissions"]["allow"],
        json!(["Bash(git log *)", "Bash(git status)"])
    );

    // The same file run without --strict keeps every command in the sandbox,
    // so the rules for `rex worktree` are not added either.
    let printed = rex_init_ok(&repo, &[]);

    let settings = written(&repo);
    assert_eq!(settings["sandbox"]["excludedCommands"], json!([]));
    assert_eq!(
        settings["permissions"]["allow"],
        json!(["Bash(git log *)", "Bash(git status)"])
    );
    assert!(
        !printed.stderr.contains("removed"),
        "got: {}",
        printed.stderr
    );
}
