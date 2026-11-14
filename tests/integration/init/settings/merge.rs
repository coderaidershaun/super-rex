//! The merge of rex's settings into `.claude/settings.local.json`, a file the
//! person owns: what a fresh file holds, and what the merge adds and keeps.

use std::path::Path;

use serde_json::{Value, json};

use super::expected::{
    OLD_HOOKS, PLUGIN_DENY_RULES, as_written, parent_of, read_settings, safety_settings,
    trivy_cache, with_depth, write_settings,
};
use crate::init::support::{ONE_STEP_FAILED, repo, rex_init_failed, rex_init_ok};

/// `text` as a JSON string, quotes included.
fn quoted(text: &str) -> String {
    serde_json::to_string(text).expect("quote text")
}

/// A worked example: a person's settings file before
/// `rex init` runs in the repo `/Users/me/code/app` on macOS...
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
      "Bash(npm test)",
      "Bash(rex worktree add *)",
      "Bash(rex worktree remove *)"
    ],
    "deny": [
      "Bash(rm -rf *)",
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
    "additionalDirectories": [
      "/Users/me/code"
    ],
    "blockWritesOutsideWorkingDirectories": true
  },
  "sandbox": {
    "enabled": false,
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
      "git worktree *",
      "git merge *",
      "git pull *",
      "git switch *",
      "rex init*",
      "rex worktree *"
    ]
  },
  "model": "opus",
  "env": {
    "CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH": "4"
  }
}
"#;

#[test]
fn fresh_settings_hold_the_safety_settings_and_the_depth() {
    let repo = repo();

    let printed = rex_init_ok(&repo, &[]);

    assert!(
        printed
            .stdout
            .starts_with("rex init: .claude/settings.local.json created\n"),
        "got: {}",
        printed.stdout
    );
    let expected = with_depth(safety_settings(&repo));
    let text = read_settings(&repo);
    assert_eq!(text, as_written(&expected));
    assert!(!text.contains("\"hooks\""), "rex init registers no hook");

    let written: Value = serde_json::from_str(&text).expect("settings JSON");
    let parent = written["permissions"]["additionalDirectories"][0]
        .as_str()
        .expect("a parent folder");
    assert!(Path::new(parent).is_absolute(), "got: {parent}");
}

// A deny rule that is there twice is harmless, but a run that adds it again
// each time would grow the person's file on every run.
#[test]
fn init_writes_deny_rules_once() {
    let repo = repo();
    write_settings(
        &repo,
        &json!({ "permissions": { "deny": [PLUGIN_DENY_RULES[3], "Bash(rm -rf *)"] } }).to_string(),
    );

    rex_init_ok(&repo, &[]);
    rex_init_ok(&repo, &[]);

    let written: Value = serde_json::from_str(&read_settings(&repo)).expect("settings JSON");
    let deny: Vec<&str> = written["permissions"]["deny"]
        .as_array()
        .expect("a deny list")
        .iter()
        .map(|rule| rule.as_str().expect("a rule"))
        .collect();
    for rule in PLUGIN_DENY_RULES {
        assert_eq!(
            deny.iter().filter(|kept| **kept == rule).count(),
            1,
            "{rule}"
        );
    }
    assert_eq!(
        deny[..2],
        [PLUGIN_DENY_RULES[3], "Bash(rm -rf *)"],
        "the person's order stays"
    );
}

// The depth is the person's to choose: a higher one is kept as it is, a lower
// one is kept with a warning, and only `--depth` writes another value.
#[test]
fn init_keeps_a_higher_depth_and_warns_on_a_lower_one() {
    let depth_of = |repo: &crate::init::support::Repo| -> Value {
        let written: Value = serde_json::from_str(&read_settings(repo)).expect("settings JSON");
        written["env"]["CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH"].clone()
    };
    let config_depth = |repo: &crate::init::support::Repo| -> Value {
        let text = std::fs::read_to_string(repo.path().join(".rex/config.json")).expect("config");
        serde_json::from_str::<Value>(&text).expect("config JSON")["depth"].clone()
    };

    let higher = repo();
    write_settings(
        &higher,
        r#"{"env": {"CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH": "6", "OTHER": "x"}}"#,
    );

    let printed = rex_init_ok(&higher, &[]);

    assert_eq!(printed.stderr, "");
    assert_eq!(depth_of(&higher), json!("6"));
    assert_eq!(config_depth(&higher), json!(6));

    let lower = repo();
    write_settings(
        &lower,
        r#"{"env": {"CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH": "2"}}"#,
    );

    let printed = rex_init_ok(&lower, &[]);

    assert_eq!(
        printed.stderr,
        "rex init: warning: kept env.CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH = \"2\" in .claude/settings.local.json, below 4, so /rex master refuses to start: its workers start their own sub-agents at depth 4; run rex init --depth 4 to allow it.\n"
    );
    assert_eq!(depth_of(&lower), json!("2"));
    assert_eq!(config_depth(&lower), json!(2));

    let printed = rex_init_ok(&lower, &["--depth", "5"]);

    assert_eq!(printed.stderr, "");
    assert_eq!(depth_of(&lower), json!("5"));
    assert_eq!(config_depth(&lower), json!(5));
}

// The settings file is the person's own: every key, value and order of
// theirs must survive, and only what is missing is added.
#[test]
fn a_settings_file_the_user_has_keeps_its_keys() {
    let example = repo();
    write_settings(&example, EXAMPLE_BEFORE);

    let printed = rex_init_failed(&example, &[]);

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

    // A value that is not a safety setting is kept with a warning only.
    let kept_deny = repo();
    write_settings(&kept_deny, r#"{"permissions": {"deny": "Bash(sudo *)"}}"#);

    let printed = rex_init_ok(&kept_deny, &[]);

    let deny = safety_settings(&kept_deny)["permissions"]["deny"].to_string();
    assert_eq!(
        printed.stderr,
        format!(
            "rex init: warning: kept permissions.deny = \"Bash(sudo *)\" in .claude/settings.local.json; SuperRex sets {deny}.\n"
        )
    );

    // A value that is not an object where the block on outside writes goes
    // keeps that block from being set, which is as weak as turning it off.
    let blocked = repo();
    write_settings(&blocked, "{\"permissions\": \"none\"}\n");

    let printed = rex_init_failed(&blocked, &[]);

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
    let everything = with_depth(safety_settings(&complete));
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

// The commands of the 0.7 hooks are gone, so each of their entries would fail
// on every event. They are taken out, and only they: a hook of the person's own
// stays, also in an event that held one of rex's.
#[test]
fn init_takes_out_the_hook_entries_of_rex_0_7_and_only_those() {
    let repo = repo();
    let own_group = json!({ "hooks": [{ "type": "command", "command": "my-logger" }] });
    let changed_rex_group = json!({ "hooks": [{ "type": "command", "command": "rex hook >/dev/null 2>&1 || true", "timeout": 30 }] });
    let mixed_group = json!({ "matcher": "Bash", "hooks": [
        { "type": "command", "command": "rex test-hook 2>/dev/null || true", "timeout": 10 },
        { "type": "command", "command": "my-linter" }
    ] });
    let mut old: Value = serde_json::from_str(OLD_HOOKS).expect("hooks JSON");
    old["SubagentStart"]
        .as_array_mut()
        .expect("a list")
        .push(own_group.clone());
    old["SubagentStop"] = json!([changed_rex_group]);
    old["PreToolUse"]
        .as_array_mut()
        .expect("a list")
        .push(mixed_group);
    write_settings(&repo, &json!({ "hooks": old, "model": "opus" }).to_string());

    let printed = rex_init_ok(&repo, &[]);

    assert_eq!(printed.stderr, "");
    let written: Value = serde_json::from_str(&read_settings(&repo)).expect("settings JSON");
    assert_eq!(
        written["hooks"],
        json!({
            "SubagentStart": [own_group],
            "PreToolUse": [{ "matcher": "Bash", "hooks": [{ "type": "command", "command": "my-linter" }] }]
        })
    );
    assert_eq!(written["model"], json!("opus"));

    // With only rex's entries in the file, no `hooks` key is left.
    let only_rex = super::super::support::repo();
    write_settings(
        &only_rex,
        &json!({ "hooks": serde_json::from_str::<Value>(OLD_HOOKS).expect("hooks JSON") })
            .to_string(),
    );

    rex_init_ok(&only_rex, &[]);

    let written: Value = serde_json::from_str(&read_settings(&only_rex)).expect("settings JSON");
    assert_eq!(written.get("hooks"), None);
}
