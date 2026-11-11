//! The safety settings `rex init` sets, which keep an agent's commands in the
//! sandbox and its writes in the repo, and which values of a person's weaken them.

use std::env;
use std::path::Path;

use serde_json::{Map, Value, json};

use super::print_line_to_stderr;

/// Where trivy keeps its vulnerability database. Without write access to it,
/// each push fails once the database is older than about a day.
#[cfg(target_os = "macos")]
const TRIVY_CACHE: &str = "~/Library/Caches/trivy";
#[cfg(not(target_os = "macos"))]
const TRIVY_CACHE: &str = "~/.cache/trivy";

/// Commands that run outside the sandbox, unless `is_strict`. The sandbox never
/// lets a command write `.claude/skills`, `.claude/agents` or the settings
/// files, and no setting narrows that. These commands must write them: a Git
/// checkout of a repo that tracks those folders, and `rex init` itself. Each
/// still goes through the permission flow. `git commit` and `git push` are not
/// here, because their hooks run code that an agent can change. Claude Code
/// takes a git command out of the sandbox only when its paths are relative,
/// with no `..`, so `git -C /abs/path ...` stays sandboxed whatever the entry.
const COMMANDS_OUTSIDE_SANDBOX: [&str; 5] = [
    "git worktree *",
    "git merge *",
    "git pull *",
    "git switch *",
    "rex init*",
];

/// The safety settings, with `parent` opened to agents when there is one.
/// The `~/` paths stay as written, because Claude Code reads them the same
/// way from any folder a session starts in.
///
/// A sandboxed command reaches the network only through the sandbox's proxy,
/// so a program that opens its own connection, such as a WebSocket client or
/// a database driver, cannot connect. Unless `is_strict`, an agent may
/// therefore run a command again outside the sandbox, from
/// [`COMMANDS_OUTSIDE_SANDBOX`].
pub(super) fn safety_settings(parent: Option<&str>, is_strict: bool) -> Map<String, Value> {
    let mut permissions = Map::new();
    if let Some(parent) = parent {
        permissions.insert("additionalDirectories".into(), json!([parent]));
    }
    permissions.insert("blockWritesOutsideWorkingDirectories".into(), json!(true));
    permissions.insert(
        "deny".into(),
        json!([
            "Bash(sudo *)",
            "Bash(git push --force*)",
            "Bash(cargo publish*)",
            "Bash(gh pr merge*--admin*)"
        ]),
    );

    let mut sandbox = json!({
        "enabled": true,
        "failIfUnavailable": true,
        "allowUnsandboxedCommands": !is_strict,
        "enableWeakerNetworkIsolation": true,
        // Lets a test server or a nested `claude -p` listen on localhost.
        "network": { "allowLocalBinding": true },
        "filesystem": {
            "allowWrite": [
                "~/.cargo/registry",
                "~/.cargo/git",
                "~/.cargo/.package-cache",
                "~/.cargo/.package-cache-mutate",
                "~/.cargo/.global-cache",
                "~/.cache/pre-commit/patch*",
                TRIVY_CACHE
            ],
            "denyRead": ["~/.aws"]
        }
    });

    if !is_strict {
        sandbox["excludedCommands"] = json!(COMMANDS_OUTSIDE_SANDBOX);
    }

    let mut settings = Map::new();
    settings.insert("sandbox".into(), sandbox);
    settings.insert("permissions".into(), Value::Object(permissions));
    settings
}

/// `parent`, the folder that holds the repo, as text, so agents may write in
/// it and edit sibling repos. `None`, with a warning, when it holds the home
/// folder or the home folder is unknown, because agents could then write to
/// keys, shell profiles and installed programs.
pub(super) fn parent_to_open(parent: &Path) -> Option<String> {
    let home = env::home_dir().map(|home| home.canonicalize().unwrap_or(home));
    if home.is_some_and(|home| !home.starts_with(parent)) {
        // Hazard: a folder name that is not valid UTF-8 is written with
        // replacement characters, so the entry names no real folder. Agents
        // then cannot write there, which fails safe.
        return Some(parent.to_string_lossy().into_owned());
    }
    print_line_to_stderr(&format!(
        "rex init: warning: the parent folder {} holds your home folder, so it was not added as a working directory. Agents can write only inside this repo. Move the repo into a folder of its own to let agents edit sibling repos.",
        parent.display()
    ));
    None
}

/// True when `settings` set `sandbox.allowUnsandboxedCommands` to false, as
/// `rex init --strict` or the person did.
pub(super) fn forbids_unsandboxed(settings: &Map<String, Value>) -> bool {
    settings
        .get("sandbox")
        .and_then(|sandbox| sandbox.get("allowUnsandboxedCommands"))
        == Some(&Value::Bool(false))
}

/// The one safety setting outside `sandbox`.
const BLOCK_WRITES_KEY: &str = "permissions.blockWritesOutsideWorkingDirectories";

/// A sandbox setting, the block on writes outside the repo, or a key above
/// that block. A weaker value for one of these turns a protection off. A
/// value of another type above the block, such as `permissions` as a text,
/// keeps rex from setting the block at all, which is as weak.
pub(super) fn is_safety_key(key: &str) -> bool {
    let is_in_sandbox = key == "sandbox" || key.starts_with("sandbox.");
    let is_at_or_above_block = BLOCK_WRITES_KEY
        .strip_prefix(key)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('.'));
    is_in_sandbox || is_at_or_above_block
}

/// The setting that lets an agent run a command again outside the sandbox.
pub(super) const UNSANDBOXED_KEY: &str = "sandbox.allowUnsandboxedCommands";
