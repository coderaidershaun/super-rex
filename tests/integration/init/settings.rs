//! The safety settings and the hook registration that `rex init` merges into
//! `.claude/settings.local.json`, a file the person owns: what it adds, what
//! it keeps of theirs, and what it refuses to change.

use std::fs;
use std::path::Path;

use serde_json::{Value, json};

use super::support::{
    Repo, SKILL_MARKER, installed, repo, rex_init_command, rex_init_failed, rex_init_ok,
    settings_path,
};

/// The `hooks` value of the hook contract, word for word.
const HOOKS: &str = r#"{
    "SubagentStart": [{ "hooks": [{ "type": "command", "command": "rex hook >/dev/null 2>&1 || true", "timeout": 10 }] }],
    "SubagentStop":  [{ "hooks": [{ "type": "command", "command": "rex hook >/dev/null 2>&1 || true", "timeout": 10 }] }],
    "PostToolUse":   [{ "matcher": "Agent|Task", "hooks": [{ "type": "command", "command": "rex hook >/dev/null 2>&1 || true", "timeout": 10 }] }]
}"#;

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
fn safety_settings(repo: &Repo) -> Value {
    json!({
        "sandbox": {
            "enabled": true,
            "failIfUnavailable": true,
            "allowUnsandboxedCommands": false,
            "enableWeakerNetworkIsolation": true,
            "filesystem": {
                "allowWrite": ["~/.cargo/registry", "~/.cargo/git", "~/.cargo/.package-cache", "~/.cargo/.package-cache-mutate", "~/.cargo/.global-cache", "~/.cache/pre-commit/patch*", trivy_cache()],
                "denyRead": ["~/.aws"]
            }
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

fn read_settings(repo: &Repo) -> String {
    fs::read_to_string(settings_path(repo)).expect("read settings")
}

fn write_settings(repo: &Repo, text: &str) {
    fs::create_dir_all(repo.path().join(".claude")).expect("create .claude");
    fs::write(settings_path(repo), text).expect("write settings");
}

/// `text` as a JSON string, quotes included.
fn quoted(text: &str) -> String {
    serde_json::to_string(text).expect("quote text")
}

/// The worked example of the init contract: a person's settings file before
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
    "allowUnsandboxedCommands": false,
    "enableWeakerNetworkIsolation": true,
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
    }
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
    let mut expected = safety_settings(&repo);
    expected["hooks"] = serde_json::from_str(HOOKS).expect("hooks JSON");
    let text = read_settings(&repo);
    assert_eq!(text, as_written(&expected));

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
    assert!(
        printed.stderr.starts_with(
            "rex init: failed: kept sandbox.enabled = false in .claude/settings.local.json"
        ),
        "got: {}",
        printed.stderr
    );
    let expected = EXAMPLE_AFTER
        .replace(&quoted("/Users/me/code"), &quoted(&parent_of(&example)))
        .replace(&quoted("~/Library/Caches/trivy"), &quoted(trivy_cache()));
    assert_eq!(read_settings(&example), expected);

    // A hook group of the person's own stays, a rex group they changed is not
    // added again, and a value that is not a safety setting is kept with a
    // warning only.
    let with_hooks = repo();
    let own_group = json!({ "hooks": [{ "type": "command", "command": "my-logger" }] });
    let changed_rex_group = json!({ "hooks": [{ "type": "command", "command": "rex hook >/dev/null 2>&1 || true", "timeout": 30 }] });
    let before = json!({
        "hooks": {
            "SubagentStop": [changed_rex_group],
            "SubagentStart": [own_group]
        },
        "permissions": { "deny": "Bash(sudo *)" }
    });
    write_settings(&with_hooks, &before.to_string());

    let printed = rex_init_ok(&with_hooks, &[]);

    assert_eq!(
        printed.stderr,
        "rex init: warning: kept permissions.deny = \"Bash(sudo *)\" in .claude/settings.local.json; SuperRex sets [\"Bash(sudo *)\",\"Bash(git push --force*)\",\"Bash(cargo publish*)\",\"Bash(gh pr merge*--admin*)\"].\n"
    );
    let rex_hooks: Value = serde_json::from_str(HOOKS).expect("hooks JSON");
    let safety = safety_settings(&with_hooks);
    let expected = json!({
        "hooks": {
            "SubagentStop": [changed_rex_group],
            "SubagentStart": [own_group, rex_hooks["SubagentStart"][0]],
            "PostToolUse": rex_hooks["PostToolUse"]
        },
        "permissions": {
            "deny": "Bash(sudo *)",
            "additionalDirectories": [parent_of(&with_hooks)],
            "blockWritesOutsideWorkingDirectories": true
        },
        "sandbox": safety["sandbox"]
    });
    assert_eq!(read_settings(&with_hooks), as_written(&expected));
}

// A safety setting the person turned off is theirs to keep, but the run must
// fail, so that the weaker setting cannot go unseen.
#[test]
fn a_weaker_safety_setting_is_kept_and_the_run_fails() {
    let repo = repo();
    write_settings(&repo, "{\"sandbox\": {\"enabled\": false}}\n");

    let printed = rex_init_failed(&repo, &[]);

    assert_eq!(
        printed.stderr,
        "rex init: failed: kept sandbox.enabled = false in .claude/settings.local.json, so the protection it gives is off; set it to true to turn it on\n\
         error: rex init did not finish 1 step(s); each is named above\n"
    );
    let written: Value = serde_json::from_str(&read_settings(&repo)).expect("settings JSON");
    assert_eq!(written["sandbox"]["enabled"], json!(false));
    assert_eq!(written["sandbox"]["failIfUnavailable"], json!(true));
}

// A file rex cannot read as a JSON object is still the person's: writing it
// would lose whatever they meant it to hold.
#[test]
fn a_settings_file_that_is_not_json_is_left_alone_and_the_run_fails() {
    let not_json = repo();
    let broken = "{ \"sandbox\": { \"enabled\": true, }\n";
    write_settings(&not_json, broken);

    let printed = rex_init_failed(&not_json, &[]);

    assert_eq!(read_settings(&not_json), broken);
    assert!(
        installed(&not_json, SKILL_MARKER).is_file() && not_json.path().join("AGENTS.md").is_file(),
        "the rest of the install must still happen"
    );
    assert!(
        !printed.stdout.contains("settings.local.json"),
        "got: {}",
        printed.stdout
    );
    let failed = "rex init: failed: .claude/settings.local.json is not valid JSON and was left as it is, so the settings SuperRex needs are not set; correct it, then run rex init again: ";
    assert!(
        printed.stderr.starts_with(failed),
        "got: {}",
        printed.stderr
    );
    assert!(
        printed.stderr.contains(" line 1 column "),
        "got: {}",
        printed.stderr
    );
    assert!(
        printed
            .stderr
            .ends_with("\nerror: rex init did not finish 1 step(s); each is named above\n"),
        "got: {}",
        printed.stderr
    );
    assert_eq!(printed.stderr.lines().count(), 2, "got: {}", printed.stderr);

    let not_an_object = repo();
    write_settings(&not_an_object, "[]\n");

    let printed = rex_init_failed(&not_an_object, &[]);

    assert_eq!(read_settings(&not_an_object), "[]\n");
    assert_eq!(
        printed.stderr,
        "rex init: failed: .claude/settings.local.json is not a JSON object and was left as it is, so the settings SuperRex needs are not set; make its top level an object, then run rex init again\n\
         error: rex init did not finish 1 step(s); each is named above\n"
    );
}

// Leaving the hooks out turns telemetry off for a new repo, but it never
// deletes entries an earlier run wrote: the person is told to remove them.
#[test]
fn no_hooks_writes_the_safety_settings_without_hooks() {
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
        "rex init: warning: hooks left out, but hook entries from an earlier run are still in .claude/settings.local.json. Remove the entries that run rex hook to turn telemetry off.\n"
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

/// The end of every run that failed one step.
const ONE_STEP_FAILED: &str = "error: rex init did not finish 1 step(s); each is named above\n";

// A write through a link lands on whatever it points at, a file the person
// never named in this repo, so rex reads through a link but never writes
// through one.
#[cfg(unix)]
#[test]
fn a_linked_settings_file_is_not_written_through() {
    use std::os::unix::fs::symlink;

    let elsewhere = tempfile::tempdir().expect("create tempdir");
    let linked_file = repo();
    let outside = elsewhere.path().join("shared-settings.json");
    fs::write(&outside, "{}\n").expect("write outside file");
    fs::create_dir_all(linked_file.path().join(".claude")).expect("create .claude");
    symlink(&outside, settings_path(&linked_file)).expect("link settings");

    let printed = rex_init_failed(&linked_file, &[]);

    assert_eq!(
        fs::read_to_string(&outside).expect("read outside file"),
        "{}\n"
    );
    assert!(installed(&linked_file, SKILL_MARKER).is_file());
    assert_eq!(
        printed.stderr,
        format!(
            "rex init: failed: .claude/settings.local.json is a symbolic link, and rex does not write through one, so the settings SuperRex needs are not set; add them to .claude/settings.local.json where the link points, then run rex init again\n{ONE_STEP_FAILED}"
        )
    );

    let linked_folder = repo();
    let outside = elsewhere.path().join("shared-claude");
    fs::create_dir_all(&outside).expect("create outside folder");
    symlink(&outside, linked_folder.path().join(".claude")).expect("link .claude");

    let printed = rex_init_failed(&linked_folder, &[]);

    assert!(!outside.join("settings.local.json").exists());
    assert_eq!(
        printed.stderr,
        format!(
            "rex init: failed: .claude is a symbolic link, and rex does not write through one, so the settings SuperRex needs are not set; add them to .claude/settings.local.json where the link points, then run rex init again\n{ONE_STEP_FAILED}"
        )
    );

    // A person who merged the settings by hand where the link points is not
    // stopped.
    let merged_by_hand = repo();
    rex_init_ok(&merged_by_hand, &[]);
    let outside = elsewhere.path().join("merged-by-hand.json");
    fs::copy(settings_path(&merged_by_hand), &outside).expect("copy settings");
    fs::remove_file(settings_path(&merged_by_hand)).expect("remove settings");
    symlink(&outside, settings_path(&merged_by_hand)).expect("link settings");
    let before = fs::read(&outside).expect("read outside file");

    let printed = rex_init_ok(&merged_by_hand, &[]);

    assert_eq!(printed.stderr, "");
    assert_eq!(fs::read(&outside).expect("read outside file"), before);
}
