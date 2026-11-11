//! The safety settings and the hook registration that `rex init` merges into
//! `.claude/settings.local.json`, a file the person owns: what it adds, what
//! it keeps of theirs, and what it refuses to change.

use std::fs;
use std::path::Path;

use serde_json::{Value, json};

use super::support::{
    ONE_STEP_FAILED, Repo, repo, rex_init_command, rex_init_failed, rex_init_ok, settings_path,
};

/// The `hooks` value that `rex init` writes, word for word.
const HOOKS: &str = r#"{
    "SubagentStart": [{ "hooks": [{ "type": "command", "command": "rex hook >/dev/null 2>&1 || true", "timeout": 10 }] }],
    "SubagentStop":  [{ "hooks": [{ "type": "command", "command": "rex hook >/dev/null 2>&1 || true", "timeout": 10 }] }],
    "PostToolUse":   [{ "matcher": "Agent|Task", "hooks": [{ "type": "command", "command": "rex hook >/dev/null 2>&1 || true", "timeout": 10 }] }],
    "PreToolUse":    [
        { "matcher": "Agent|Task", "hooks": [{ "type": "command", "command": "rex gate 2>/dev/null || true", "timeout": 10 }] },
        { "matcher": "Bash", "hooks": [{ "type": "command", "command": "rex test-hook 2>/dev/null || true", "timeout": 10 }] }
    ]
}"#;

/// The allow rules that `rex init` writes beside the hooks, so a test command
/// piped into the filter needs no rule beyond the test command's own.
const FILTER_RULES: [&str; 2] = ["Bash(rex test-filter)", "Bash(set -o pipefail)"];

/// `settings` with the hooks and the filter's allow rule that `rex init`
/// writes unless the hooks are left out.
fn hooked(mut settings: Value) -> Value {
    settings["hooks"] = serde_json::from_str(HOOKS).expect("hooks JSON");
    settings["permissions"]["allow"] = json!(FILTER_RULES);
    settings
}

/// Where the trivy scanner keeps its database on the machine that runs the
/// tests.
fn trivy_cache() -> &'static str {
    if cfg!(target_os = "macos") {
        "~/Library/Caches/trivy"
    } else {
        "~/.cache/trivy"
    }
}

/// The folder that holds `repo`, as `rex init` names it.
fn parent_of(repo: &Repo) -> String {
    let canonical = repo.path().canonicalize().expect("canonical repo path");
    let parent = canonical.parent().expect("the repo has a parent");
    parent.to_str().expect("utf8 parent").to_string()
}

/// The safety settings `rex init` writes for `repo`, in their order.
/// The commands that `rex init` lets run outside the sandbox, except with
/// `--strict`.
const OUTSIDE_SANDBOX: [&str; 5] = [
    "git worktree *",
    "git merge *",
    "git pull *",
    "git switch *",
    "rex init*",
];

fn safety_settings(repo: &Repo) -> Value {
    json!({
        "sandbox": {
            "enabled": true,
            "failIfUnavailable": true,
            "allowUnsandboxedCommands": true,
            "enableWeakerNetworkIsolation": true,
            "network": { "allowLocalBinding": true },
            "filesystem": {
                "allowWrite": ["~/.cargo/registry", "~/.cargo/git", "~/.cargo/.package-cache", "~/.cargo/.package-cache-mutate", "~/.cargo/.global-cache", "~/.cache/pre-commit/patch*", trivy_cache()],
                "denyRead": ["~/.aws"]
            },
            "excludedCommands": OUTSIDE_SANDBOX
        },
        "permissions": {
            "additionalDirectories": [parent_of(repo)],
            "blockWritesOutsideWorkingDirectories": true,
            "deny": ["Bash(sudo *)", "Bash(git push --force*)", "Bash(cargo publish*)", "Bash(gh pr merge*--admin*)"]
        }
    })
}

/// `settings` as `rex init` writes it: pretty JSON with a 2-space indent, in
/// the order of its keys, and a newline at the end.
fn as_written(settings: &Value) -> String {
    let mut text = serde_json::to_string_pretty(settings).expect("print settings");
    text.push('\n');
    text
}

pub(super) fn read_settings(repo: &Repo) -> String {
    fs::read_to_string(settings_path(repo)).expect("read settings")
}

pub(super) fn write_settings(repo: &Repo, text: &str) {
    fs::create_dir_all(repo.path().join(".claude")).expect("create .claude");
    fs::write(settings_path(repo), text).expect("write settings");
}

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

// A program that opens its own network connection cannot run in the sandbox,
// so an agent may run a command again outside it. `--strict` takes that away,
// and a file that is stricter than the run asks for is never made weaker.
// `--no-hooks` leaves out the hook entries.
#[test]
fn strict_and_no_hooks_change_only_the_settings_they_name() {
    let allow_key = |repo: &Repo| {
        let written: Value = serde_json::from_str(&read_settings(repo)).expect("settings JSON");
        written["sandbox"]["allowUnsandboxedCommands"].clone()
    };

    let strict = repo();

    let printed = rex_init_ok(&strict, &["--strict"]);

    assert_eq!(printed.stderr, "");
    assert_eq!(allow_key(&strict), json!(false));
    let mut expected = hooked(safety_settings(&strict));
    expected["sandbox"]["allowUnsandboxedCommands"] = json!(false);
    // With --strict no command may run outside the sandbox, so none is excluded.
    expected["sandbox"]
        .as_object_mut()
        .expect("sandbox object")
        .remove("excludedCommands");
    assert_eq!(read_settings(&strict), as_written(&expected));

    // A later run without the flag keeps the stricter value, and says how to
    // change it. A file that an earlier version of rex wrote is this case.
    let before = read_settings(&strict);

    let printed = rex_init_ok(&strict, &[]);

    assert_eq!(read_settings(&strict), before);
    assert_eq!(
        printed.stderr,
        "rex init: warning: kept sandbox.allowUnsandboxedCommands = false in .claude/settings.local.json, so no agent can run a command outside the sandbox, such as a test that opens its own network connection; set it to true to let agents do that.\n"
    );

    // The other way round, the kept value is the weaker one, so the run fails.
    let open = repo();
    rex_init_ok(&open, &[]);
    assert_eq!(allow_key(&open), json!(true));

    let printed = rex_init_failed(&open, &["--strict"]);

    assert_eq!(allow_key(&open), json!(true));
    assert_eq!(
        printed.stderr,
        format!(
            "rex init: failed: kept sandbox.allowUnsandboxedCommands = true in .claude/settings.local.json, so the protection it gives is off; set it to false to turn it on\n{ONE_STEP_FAILED}"
        )
    );

    // Leaving the hooks out turns telemetry off for a new repo, but it never
    // deletes entries an earlier run wrote: the person is told to remove them.
    let fresh = repo();

    let printed = rex_init_ok(&fresh, &["--no-hooks"]);

    assert_eq!(printed.stderr, "");
    assert_eq!(read_settings(&fresh), as_written(&safety_settings(&fresh)));

    let hooked = repo();
    rex_init_ok(&hooked, &[]);
    let before = read_settings(&hooked);

    let printed = rex_init_ok(&hooked, &["--no-hooks"]);

    assert_eq!(read_settings(&hooked), before, "the hook entries stay");
    assert_eq!(
        printed.stderr,
        "rex init: warning: hooks left out, but hook entries from an earlier run are still in .claude/settings.local.json. Remove the entries that run rex hook, rex gate or rex test-hook to turn telemetry, the spend gate and the test output filter off.\n"
    );
}

// Opening the folder above the repo to agents must never open the home
// folder, which holds keys and shell profiles.
#[test]
fn a_parent_that_holds_home_is_not_added() {
    let homes: [fn(&Path) -> &Path; 2] = [
        |repo| repo.parent().expect("the repo has a parent"),
        |repo| repo,
    ];
    for home_of in homes {
        let repo = repo();
        let canonical = repo.path().canonicalize().expect("canonical repo path");

        let output = rex_init_command(&repo, &[])
            .env("HOME", home_of(&canonical))
            .output()
            .expect("run rex init");

        let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
        assert!(output.status.success(), "got: {stderr}");
        assert_eq!(
            stderr,
            format!(
                "rex init: warning: the parent folder {} holds your home folder, so it was not added as a working directory. Agents can write only inside this repo. Move the repo into a folder of its own to let agents edit sibling repos.\n",
                parent_of(&repo)
            )
        );
        let written: Value = serde_json::from_str(&read_settings(&repo)).expect("settings JSON");
        assert_eq!(written["permissions"].get("additionalDirectories"), None);
        assert_eq!(
            written["permissions"]["blockWritesOutsideWorkingDirectories"],
            json!(true)
        );
    }
}
