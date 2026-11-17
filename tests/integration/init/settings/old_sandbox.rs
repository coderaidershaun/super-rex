//! A settings file that an older `rex init` wrote, with the Claude Code sandbox
//! on: one run turns the sandbox off and takes out the block on outside
//! writes, each with a notice, keeps the rest, and a second run changes nothing.

use serde_json::{Value, json};

use super::expected::{PLUGIN_DENY_RULES, parent_of, read_settings, write_settings};
use crate::init::support::{repo, rex_init_ok};

/// A settings file as rex 0.10 wrote it, shortened, with `PARENT` for the folder
/// that holds the repo and a stopgap git rule of the person's in it.
const OLD: &str = r#"{
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
        "~/.cargo/registry"
      ],
      "denyRead": [
        "~/.aws"
      ]
    },
    "excludedCommands": [
      "git worktree *",
      "rex worktree *"
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
      "Bash(gh pr merge:*)",
      "Bash(git push origin main:*)",
      "Bash(git push origin HEAD:main:*)",
      "Bash(rex ingest:*)",
      "Bash(rex migrate:*)",
      "Bash(rex limit:*)"
    ],
    "allow": [
      "Bash(git *)",
      "Bash(rex worktree add *)",
      "Bash(rex worktree remove *)"
    ]
  },
  "env": {
    "CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH": "4"
  }
}
"#;

const SANDBOX_OFF: &str = "rex init: turned the Claude Code sandbox off in .claude/settings.local.json (sandbox.enabled was true): the plugin's guard on recursive rm replaces it, and each run of rex init turns it off again.\n";

const BLOCK_REMOVED: &str = "rex init: removed permissions.blockWritesOutsideWorkingDirectories from .claude/settings.local.json (it was true): the plugin's guard on recursive rm replaces it, and each run of rex init removes it again.\n";

fn written(repo: &crate::init::support::Repo) -> Value {
    serde_json::from_str(&read_settings(repo)).expect("settings JSON")
}

#[test]
fn one_run_turns_the_old_sandbox_off_and_a_second_is_silent() {
    let repo = repo();
    write_settings(&repo, &OLD.replace("PARENT", &parent_of(&repo)));

    let printed = rex_init_ok(&repo, &[]);

    assert_eq!(printed.stderr, format!("{SANDBOX_OFF}{BLOCK_REMOVED}"));
    assert!(
        printed
            .stdout
            .starts_with("rex init: .claude/settings.local.json updated\n"),
        "got: {}",
        printed.stdout
    );
    let settings = written(&repo);
    let mut sandbox: Value =
        serde_json::from_str::<Value>(OLD).expect("old JSON")["sandbox"].clone();
    sandbox["enabled"] = json!(false);
    assert_eq!(settings["sandbox"], sandbox, "only enabled changes");
    let permissions = settings["permissions"].as_object().expect("permissions");
    assert_eq!(
        permissions.get("blockWritesOutsideWorkingDirectories"),
        None
    );
    // The person's rules stay, the old rules of rex too, and "Bash" is added.
    assert_eq!(
        settings["permissions"]["allow"],
        json!([
            "Bash(git *)",
            "Bash(rex worktree add *)",
            "Bash(rex worktree remove *)",
            "Bash"
        ])
    );
    let deny = settings["permissions"]["deny"].as_array().expect("deny");
    assert_eq!(deny.len(), 10);
    assert_eq!(deny[4..], PLUGIN_DENY_RULES.map(|rule| json!(rule)));

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

// Only a value of true is rex's to turn off: a sandbox the person turned off,
// or a block set to false, is theirs and stays, and no sandbox key is added.
#[test]
fn a_sandbox_that_is_off_is_left_as_it_is() {
    let repo = repo();
    write_settings(
        &repo,
        r#"{"sandbox": {"enabled": false, "failIfUnavailable": true}, "permissions": {"blockWritesOutsideWorkingDirectories": false}}"#,
    );

    let printed = rex_init_ok(&repo, &[]);

    assert_eq!(printed.stderr, "");
    let settings = written(&repo);
    assert_eq!(
        settings["sandbox"],
        json!({"enabled": false, "failIfUnavailable": true})
    );
    assert_eq!(
        settings["permissions"]["blockWritesOutsideWorkingDirectories"],
        json!(false)
    );

    // Only the block, with no sandbox key: the block goes, and no sandbox comes.
    let block_only = crate::init::support::repo();
    write_settings(
        &block_only,
        r#"{"permissions": {"blockWritesOutsideWorkingDirectories": true}}"#,
    );

    let printed = rex_init_ok(&block_only, &[]);

    assert_eq!(printed.stderr, BLOCK_REMOVED);
    let settings = written(&block_only);
    assert_eq!(settings.get("sandbox"), None);
    assert_eq!(
        settings["permissions"].get("blockWritesOutsideWorkingDirectories"),
        None
    );
}
