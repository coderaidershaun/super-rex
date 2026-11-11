//! The merge of rex's settings into `.claude/settings.local.json`, a file the
//! person owns: what a fresh file holds, and what the merge adds and keeps.

use std::path::Path;

use serde_json::{Value, json};

use super::expected::{
    FILTER_RULES, HOOKS, as_written, hooked, parent_of, read_settings, safety_settings,
    trivy_cache, write_settings,
};
use crate::init::support::{ONE_STEP_FAILED, repo, rex_init_failed, rex_init_ok};

/// `text` as a JSON string, quotes included.
fn quoted(text: &str) -> String {
    serde_json::to_string(text).expect("quote text")
}

/// A worked example: a person's settings file before
/// `rex init --no-hooks` runs in the repo `/Users/me/code/app` on macOS...
const EXAMPLE_BEFORE: &str = r#"{
  "permissions": {
    "allow": ["Bash(npm test)"],
    "deny": ["Bash(rm -rf *)", "Bash(sudo *)"]
  },
  "sandbox": {
    "enabled": false
  },
  "model": "opus"
}
"#;

/// ...and after it, byte for byte.
const EXAMPLE_AFTER: &str = r#"{
  "permissions": {
    "allow": [
      "Bash(npm test)"
    ],
    "deny": [
      "Bash(rm -rf *)",
      "Bash(sudo *)",
      "Bash(git push --force*)",
      "Bash(cargo publish*)",
      "Bash(gh pr merge*--admin*)"
    ],
    "additionalDirectories": [
      "/Users/me/code"
    ],
    "blockWritesOutsideWorkingDirectories": true
  },
  "sandbox": {
    "enabled": false,
    "failIfUnavailable": true,
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
      "git worktree *",
      "git merge *",
      "git pull *",
      "git switch *",
      "rex init*"
    ]
  },
  "model": "opus"
}
"#;

#[test]
fn fresh_settings_hold_the_safety_settings_and_the_hooks() {
    let repo = repo();

    let printed = rex_init_ok(&repo, &[]);

    assert!(
        printed
            .stdout
            .starts_with("rex init: .claude/settings.local.json created\n"),
        "got: {}",
        printed.stdout
    );
    let expected = hooked(safety_settings(&repo));
    let text = read_settings(&repo);
    assert_eq!(text, as_written(&expected));

    // A second run finds the rule and the hooks in place and writes nothing,
    // so the rule is there once.
    rex_init_ok(&repo, &[]);
    assert_eq!(read_settings(&repo), text);
    for rule in FILTER_RULES {
        assert_eq!(text.matches(rule).count(), 1, "{rule}");
    }

    let written: Value = serde_json::from_str(&text).expect("settings JSON");
    let parent = written["permissions"]["additionalDirectories"][0]
        .as_str()
        .expect("a parent folder");
    assert!(Path::new(parent).is_absolute(), "got: {parent}");
}

// The settings file is the person's own: every key, value and order of
// theirs must survive, and only what is missing is added.
#[test]
fn a_settings_file_the_user_has_keeps_its_keys() {
    let example = repo();
    write_settings(&example, EXAMPLE_BEFORE);

    let printed = rex_init_failed(&example, &["--no-hooks"]);

    assert!(
        printed
            .stdout
            .starts_with("rex init: .claude/settings.local.json updated\n"),
        "got: {}",
        printed.stdout
    );
    // A safety setting the person turned off is theirs to keep, but the run
    // fails, so that the weaker setting cannot go unseen.
    assert_eq!(
        printed.stderr,
        format!(
            "rex init: failed: kept sandbox.enabled = false in .claude/settings.local.json, so the protection it gives is off; set it to true to turn it on\n{ONE_STEP_FAILED}"
        )
    );
    let expected = EXAMPLE_AFTER
        .replace(&quoted("/Users/me/code"), &quoted(&parent_of(&example)))
        .replace(&quoted("~/Library/Caches/trivy"), &quoted(trivy_cache()));
    assert_eq!(read_settings(&example), expected);

    // A hook group of the person's own stays, a rex group they changed is not
    // added again, a rex group they removed beside one they kept is added
    // again, and a value that is not a safety setting is kept with a warning only.
    let with_hooks = repo();
    let own_group = json!({ "hooks": [{ "type": "command", "command": "my-logger" }] });
    let changed_rex_group = json!({ "hooks": [{ "type": "command", "command": "rex hook >/dev/null 2>&1 || true", "timeout": 30 }] });
    let rex_hooks: Value = serde_json::from_str(HOOKS).expect("hooks JSON");
    let before = json!({
        "hooks": {
            "SubagentStop": [changed_rex_group],
            "SubagentStart": [own_group],
            "PreToolUse": [rex_hooks["PreToolUse"][0]]
        },
        "permissions": { "deny": "Bash(sudo *)" }
    });
    write_settings(&with_hooks, &before.to_string());

    let printed = rex_init_ok(&with_hooks, &[]);

    assert_eq!(
        printed.stderr,
        "rex init: warning: kept permissions.deny = \"Bash(sudo *)\" in .claude/settings.local.json; SuperRex sets [\"Bash(sudo *)\",\"Bash(git push --force*)\",\"Bash(cargo publish*)\",\"Bash(gh pr merge*--admin*)\"].\n"
    );
    let safety = safety_settings(&with_hooks);
    let expected = json!({
        "hooks": {
            "SubagentStop": [changed_rex_group],
            "SubagentStart": [own_group, rex_hooks["SubagentStart"][0]],
            "PreToolUse": rex_hooks["PreToolUse"],
            "PostToolUse": rex_hooks["PostToolUse"]
        },
        "permissions": {
            "deny": "Bash(sudo *)",
            "additionalDirectories": [parent_of(&with_hooks)],
            "blockWritesOutsideWorkingDirectories": true,
            "allow": FILTER_RULES
        },
        "sandbox": safety["sandbox"]
    });
    assert_eq!(read_settings(&with_hooks), as_written(&expected));

    // A value that is not an object where the block on outside writes goes
    // keeps that block from being set, which is as weak as turning it off.
    let blocked = repo();
    write_settings(&blocked, "{\"permissions\": \"none\"}\n");

    let printed = rex_init_failed(&blocked, &["--no-hooks"]);

    let permissions = safety_settings(&blocked)["permissions"].to_string();
    assert_eq!(
        printed.stderr,
        format!(
            "rex init: failed: kept permissions = \"none\" in .claude/settings.local.json, so the protection it gives is off; set it to {permissions} to turn it on\n{ONE_STEP_FAILED}"
        )
    );
    let written: Value = serde_json::from_str(&read_settings(&blocked)).expect("settings JSON");
    assert_eq!(written["permissions"], json!("none"));

    // A file that already holds everything is not written at all, so it
    // keeps the person's own format.
    let complete = repo();
    let everything = hooked(safety_settings(&complete));
    let one_line = everything.to_string();
    write_settings(&complete, &one_line);

    let printed = rex_init_ok(&complete, &[]);

    assert!(
        !printed.stdout.contains("settings.local.json"),
        "got: {}",
        printed.stdout
    );
    assert_eq!(read_settings(&complete), one_line);
}
