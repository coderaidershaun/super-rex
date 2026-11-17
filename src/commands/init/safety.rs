//! The permissions `rex init` sets, which let an agent run any Bash command
//! while the deny rules hold, and the turning off of a sandbox an older run set.

use std::env;
use std::path::Path;

use serde_json::{Map, Value, json};

use super::print_line_to_stderr;
use crate::commands::worktree::SETTINGS_FILE;

/// The allow rule that lets an agent run every Bash command with no prompt.
/// The deny rules still hold, and the plugin's guard refuses a recursive `rm`
/// outside the folder that holds the repo.
const ALLOW_EVERY_BASH_COMMAND: &str = "Bash";

/// The permissions `rex init` merges in, with `parent` opened to agents when
/// there is one, and each of `worktrees`, the job worktrees of the repo,
/// listed after it, so the file tools may work in them.
///
/// The deny rules after the first four keep merges and the record out of an
/// agent's hands: a merge goes through the plugin's own merge tool, which does
/// not run in the shell, and only the plugin and the person write the store
/// and the spend limits. A rule holds even when a hook of the plugin is
/// skipped, and over the rule that allows every Bash command.
pub(super) fn permission_settings(
    parent: Option<&str>,
    worktrees: &[String],
) -> Map<String, Value> {
    let mut permissions = Map::new();
    let directories: Vec<&str> = parent
        .into_iter()
        .chain(worktrees.iter().map(String::as_str))
        .collect();
    if !directories.is_empty() {
        permissions.insert("additionalDirectories".into(), json!(directories));
    }
    permissions.insert(
        "deny".into(),
        json!([
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
        ]),
    );
    permissions.insert("allow".into(), json!([ALLOW_EVERY_BASH_COMMAND]));

    let mut settings = Map::new();
    settings.insert("permissions".into(), Value::Object(permissions));
    settings
}

/// `parent`, the folder that holds the repo, as text, so the file tools of
/// agents may work in it and edit sibling repos. `None`, with a warning, when
/// it holds the home folder or the home folder is unknown, because those tools
/// could then write to keys, shell profiles and installed programs.
pub(super) fn parent_to_open(parent: &Path) -> Option<String> {
    let home = env::home_dir().map(|home| home.canonicalize().unwrap_or(home));
    if home.is_some_and(|home| !home.starts_with(parent)) {
        // Hazard: a folder name that is not valid UTF-8 is written with
        // replacement characters, so the entry names no real folder. Agents
        // then cannot edit there, which fails safe.
        return Some(parent.to_string_lossy().into_owned());
    }
    print_line_to_stderr(&format!(
        "rex init: warning: the parent folder {} holds your home folder, so it was not added as a working directory, and the file tools of agents work only inside this repo. Move the repo into a folder of its own to let them edit sibling repos.",
        parent.display()
    ));
    None
}

/// Turns off what an older `rex init` turned on in this file: sets
/// `sandbox.enabled` to false when it is true, keeping every other `sandbox`
/// key, and takes out `permissions.blockWritesOutsideWorkingDirectories` when
/// it is true. Any other value is the person's and stays, and no key is
/// added. Gives one notice for each change, so a second run gives none.
pub(super) fn turn_old_sandbox_off(settings: &mut Map<String, Value>) -> Vec<String> {
    let mut notices = Vec::new();
    if let Some(Value::Object(sandbox)) = settings.get_mut("sandbox")
        && let Some(enabled @ Value::Bool(true)) = sandbox.get_mut("enabled")
    {
        *enabled = Value::Bool(false);
        notices.push(format!(
            "rex init: turned the Claude Code sandbox off in {SETTINGS_FILE} (sandbox.enabled was true): the plugin's guard on recursive rm replaces it, and each run of rex init turns it off again."
        ));
    }
    if let Some(Value::Object(permissions)) = settings.get_mut("permissions")
        && permissions.get("blockWritesOutsideWorkingDirectories") == Some(&Value::Bool(true))
    {
        permissions.remove("blockWritesOutsideWorkingDirectories");
        notices.push(format!(
            "rex init: removed permissions.blockWritesOutsideWorkingDirectories from {SETTINGS_FILE} (it was true): the plugin's guard on recursive rm replaces it, and each run of rex init removes it again."
        ));
    }
    notices
}

#[cfg(test)]
mod tests {
    use super::*;

    fn object(value: Value) -> Map<String, Value> {
        let Value::Object(map) = value else {
            panic!("not an object")
        };
        map
    }

    #[test]
    fn an_old_sandbox_is_turned_off_once_and_the_rest_stays() {
        let mut settings = object(json!({
            "sandbox": { "enabled": true, "failIfUnavailable": true, "network": { "allowLocalBinding": true } },
            "permissions": { "blockWritesOutsideWorkingDirectories": true, "deny": ["Bash(sudo *)"] }
        }));

        let notices = turn_old_sandbox_off(&mut settings);

        assert_eq!(notices.len(), 2, "got: {notices:?}");
        assert_eq!(
            Value::Object(settings.clone()),
            json!({
                "sandbox": { "enabled": false, "failIfUnavailable": true, "network": { "allowLocalBinding": true } },
                "permissions": { "deny": ["Bash(sudo *)"] }
            })
        );
        assert!(turn_old_sandbox_off(&mut settings).is_empty());
    }

    #[test]
    fn a_value_that_is_not_true_is_left_as_it_is() {
        for value in [
            json!({}),
            json!({ "sandbox": { "enabled": false }, "permissions": { "blockWritesOutsideWorkingDirectories": false } }),
            json!({ "sandbox": { "enabled": "yes" }, "permissions": { "blockWritesOutsideWorkingDirectories": 1 } }),
            json!({ "sandbox": "on", "permissions": ["Bash"] }),
        ] {
            let mut settings = object(value.clone());

            let notices = turn_old_sandbox_off(&mut settings);

            assert!(notices.is_empty(), "got: {notices:?}");
            assert_eq!(Value::Object(settings), value);
        }
    }
}
