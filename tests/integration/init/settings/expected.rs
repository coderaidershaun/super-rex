//! What `rex init` writes into a fresh `.claude/settings.local.json`, worked out
//! here without the crate's code, and the reading and writing of that file.

use std::fs;

use serde_json::{Value, json};

use crate::init::support::{Repo, settings_path};

/// The `hooks` value that `rex init` writes, word for word.
pub(super) const HOOKS: &str = r#"{
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
pub(super) const FILTER_RULES: [&str; 2] = ["Bash(rex test-filter)", "Bash(set -o pipefail)"];

/// `settings` with the hooks and the filter's allow rule that `rex init`
/// writes unless the hooks are left out.
pub(super) fn hooked(mut settings: Value) -> Value {
    settings["hooks"] = serde_json::from_str(HOOKS).expect("hooks JSON");
    settings["permissions"]["allow"] = json!(FILTER_RULES);
    settings
}

/// Where the trivy scanner keeps its database on the machine that runs the
/// tests.
pub(super) fn trivy_cache() -> &'static str {
    if cfg!(target_os = "macos") {
        "~/Library/Caches/trivy"
    } else {
        "~/.cache/trivy"
    }
}

/// The folder that holds `repo`, as `rex init` names it.
pub(super) fn parent_of(repo: &Repo) -> String {
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

pub(super) fn safety_settings(repo: &Repo) -> Value {
    json!({
        "sandbox": {
            "enabled": true,
            "failIfUnavailable": false,
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
pub(super) fn as_written(settings: &Value) -> String {
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
