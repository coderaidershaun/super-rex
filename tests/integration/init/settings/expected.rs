//! What `rex init` writes into a fresh `.claude/settings.local.json`, worked out
//! here without the crate's code, and the reading and writing of that file.

use std::fs;

use serde_json::{Value, json};

use crate::init::support::{Repo, settings_path};

/// The hook entries that rex 0.7 wrote, word for word, which `rex init` takes out.
pub(super) const OLD_HOOKS: &str = r#"{
    "SubagentStart": [{ "hooks": [{ "type": "command", "command": "rex hook >/dev/null 2>&1 || true", "timeout": 10 }] }],
    "SubagentStop":  [{ "hooks": [{ "type": "command", "command": "rex hook >/dev/null 2>&1 || true", "timeout": 10 }] }],
    "PostToolUse":   [{ "matcher": "Agent|Task", "hooks": [{ "type": "command", "command": "rex hook >/dev/null 2>&1 || true", "timeout": 10 }] }],
    "PreToolUse":    [
        { "matcher": "Agent|Task", "hooks": [{ "type": "command", "command": "rex gate 2>/dev/null || true", "timeout": 10 }] },
        { "matcher": "Bash", "hooks": [{ "type": "command", "command": "rex test-hook 2>/dev/null || true", "timeout": 10 }] }
    ]
}"#;

/// The deny rules that keep merges and the record out of an agent's hands.
pub(super) const PLUGIN_DENY_RULES: [&str; 6] = [
    "Bash(gh pr merge:*)",
    "Bash(git push origin main:*)",
    "Bash(git push origin HEAD:main:*)",
    "Bash(rex ingest:*)",
    "Bash(rex migrate:*)",
    "Bash(rex limit:*)",
];

/// `settings` with the sub-agent depth that `rex init` writes when the person set none.
pub(super) fn with_depth(mut settings: Value) -> Value {
    settings["env"] = json!({ "CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH": "4" });
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
const OUTSIDE_SANDBOX: [&str; 6] = [
    "git worktree *",
    "git merge *",
    "git pull *",
    "git switch *",
    "rex init*",
    "rex worktree *",
];

/// The allow rules that let `rex worktree` run outside the sandbox with no
/// prompt, except with `--strict`.
pub(super) const WORKTREE_ALLOW_RULES: [&str; 2] =
    ["Bash(rex worktree add *)", "Bash(rex worktree remove *)"];

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
            "deny": ["Bash(sudo *)", "Bash(git push --force*)", "Bash(cargo publish*)", "Bash(gh pr merge*--admin*)",
                     PLUGIN_DENY_RULES[0], PLUGIN_DENY_RULES[1], PLUGIN_DENY_RULES[2],
                     PLUGIN_DENY_RULES[3], PLUGIN_DENY_RULES[4], PLUGIN_DENY_RULES[5]],
            "allow": WORKTREE_ALLOW_RULES
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
