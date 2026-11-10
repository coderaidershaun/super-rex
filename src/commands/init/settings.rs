//! Merges the safety settings and the hook registration into the repo's
//! `.claude/settings.local.json`, a file the person owns, so nothing of theirs
//! is lost and a weaker setting of theirs is never hidden.

use std::io::{self, ErrorKind, Write};
use std::path::Path;
use std::{env, fs};

use serde_json::{Map, Value, json};

use super::error::InitError;
use super::file_change::FileChange;
use crate::commands::links::is_symlink;
use crate::error::RexError;

/// The settings file, as every message names it.
pub(super) const SETTINGS_FILENAME: &str = ".claude/settings.local.json";

/// The command every rex hook runs. A hook with exactly this command is rex's
/// own, even when the person changed the rest of its entry.
const HOOK_COMMAND: &str = "rex hook >/dev/null 2>&1 || true";

/// The value of the `hooks` key: one entry for each event that tells rex a
/// sub-agent started, stopped, or was called.
fn hook_registration() -> Value {
    let hook = json!([{ "type": "command", "command": HOOK_COMMAND, "timeout": 10 }]);
    json!({
        "SubagentStart": [{ "hooks": hook }],
        "SubagentStop": [{ "hooks": hook }],
        "PostToolUse": [{ "matcher": "Agent|Task", "hooks": hook }],
    })
}

/// Where trivy keeps its vulnerability database. Without write access to it,
/// each push fails once the database is older than about a day.
#[cfg(target_os = "macos")]
const TRIVY_CACHE: &str = "~/Library/Caches/trivy";
#[cfg(not(target_os = "macos"))]
const TRIVY_CACHE: &str = "~/.cache/trivy";

/// The safety settings, with `parent` opened to agents when there is one.
/// The `~/` paths stay as written, because Claude Code reads them the same
/// way from any folder a session starts in.
fn safety_settings(parent: Option<&str>) -> Map<String, Value> {
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

    let sandbox = json!({
        "enabled": true,
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
                TRIVY_CACHE
            ],
            "denyRead": ["~/.aws"]
        }
    });

    let mut settings = Map::new();
    settings.insert("sandbox".into(), sandbox);
    settings.insert("permissions".into(), Value::Object(permissions));
    settings
}

/// What rex adds to the settings file of the repo at `cwd`, in the order it
/// is added.
fn settings_to_add(cwd: &Path, is_without_hooks: bool) -> Result<Map<String, Value>, RexError> {
    let repo = cwd.canonicalize().map_err(|source| RexError::Io {
        path: cwd.to_path_buf(),
        source,
    })?;
    let parent = repo.parent().and_then(parent_to_open);
    let mut settings = safety_settings(parent.as_deref());
    if !is_without_hooks {
        settings.insert("hooks".into(), hook_registration());
    }
    Ok(settings)
}

/// `parent`, the folder that holds the repo, as agents are let to write in
/// it so they can edit sibling repos. `None` when it holds the home folder,
/// or the home folder is unknown, because then agents could write anywhere
/// in it: keys, shell profiles, installed programs.
fn parent_to_open(parent: &Path) -> Option<String> {
    let home = env::home_dir().map(|home| home.canonicalize().unwrap_or(home));
    if home.is_some_and(|home| !home.starts_with(parent)) {
        // Hazard: a folder name that is not valid UTF-8 is written with
        // replacement characters, so the entry names no real folder. Agents
        // then cannot write there, which fails safe.
        return Some(parent.to_string_lossy().into_owned());
    }
    // A warning that cannot be shown must not stop the install.
    let _ = writeln!(
        io::stderr().lock(),
        "rex init: warning: the parent folder {} holds your home folder, so it was not added as a working directory. Agents can write only inside this repo. Move the repo into a folder of its own to let agents edit sibling repos.",
        parent.display()
    );
    None
}

/// Make sure the settings file of the repo at `cwd` holds the safety settings
/// and, unless `is_without_hooks`, the hook registration. Every key and value
/// of the person's is kept, in its order, and only what is missing is added.
/// When nothing is missing, the file is not written at all.
///
/// The file is left as it is, and the reason added to `failures`, when it is
/// not a JSON object, cannot be read, or is to be written through a symbolic
/// link. A kept value of the person's that weakens a safety setting is added
/// to `failures` too; any other kept value is a warning on stderr.
///
/// # Errors
/// [`RexError::Io`] if the repo folder cannot be found, or the file or its
/// folder cannot be written.
pub(super) fn ensure_settings(
    cwd: &Path,
    is_without_hooks: bool,
    failures: &mut Vec<InitError>,
) -> Result<FileChange, RexError> {
    let path = cwd.join(SETTINGS_FILENAME);
    let to_add = settings_to_add(cwd, is_without_hooks)?;

    let (settings, change) = match read_settings(&path) {
        Err(failure) => {
            failures.push(failure);
            return Ok(FileChange::Unchanged);
        }
        Ok(None) => (to_add, FileChange::Created),
        Ok(Some(mut settings)) => {
            if is_without_hooks && holds_rex_hooks(&settings) {
                warn_hooks_left_in();
            }
            let mut merge = Merge::default();
            merge.object(&mut settings, &to_add, "");
            failures.extend(merge.kept.into_iter().filter_map(report_kept_value));
            if !merge.is_changed {
                return Ok(FileChange::Unchanged);
            }
            (settings, FileChange::Updated)
        }
    };

    if let Some(link) = linked_part(cwd) {
        failures.push(InitError::SettingsLinked { link });
        return Ok(FileChange::Unchanged);
    }
    write_settings(&path, &Value::Object(settings))?;
    Ok(change)
}

/// The part of the settings path that is a symbolic link, as messages name
/// it, or `None` when neither is one. A write through either would land on
/// whatever the link points at.
fn linked_part(cwd: &Path) -> Option<&'static str> {
    [SETTINGS_FILENAME, ".claude"]
        .into_iter()
        .find(|part| is_symlink(&cwd.join(part)))
}

/// The person's settings, or `None` when there is no file. A link is read
/// through, which changes nothing.
///
/// # Errors
/// The failure that names why the file cannot be merged: it cannot be read,
/// it is not JSON, or its top level is not an object.
fn read_settings(path: &Path) -> Result<Option<Map<String, Value>>, InitError> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(source) if source.kind() == ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(InitError::SettingsUnreadable { source }),
    };
    match serde_json::from_slice(&bytes) {
        Ok(Value::Object(settings)) => Ok(Some(settings)),
        Ok(_) => Err(InitError::SettingsNotObject),
        Err(source) => Err(InitError::SettingsNotJson { source }),
    }
}

/// A value of the person's that rex left in place although it differs from
/// the one rex sets.
struct KeptValue {
    /// The keys from the top of the file down to the value, joined by dots.
    key: String,
    found: Value,
    needed: Value,
}

/// The work of one merge of rex's settings into the person's.
#[derive(Default)]
struct Merge {
    is_changed: bool,
    kept: Vec<KeptValue>,
}

impl Merge {
    /// Add each key of `to_add` that `settings` lacks, after the keys it has,
    /// and merge the values of the keys both have.
    fn object(
        &mut self,
        settings: &mut Map<String, Value>,
        to_add: &Map<String, Value>,
        key: &str,
    ) {
        for (name, needed) in to_add {
            let key = if key.is_empty() {
                name.clone()
            } else {
                format!("{key}.{name}")
            };
            match settings.get_mut(name) {
                Some(found) => self.value(found, needed, key),
                None => {
                    settings.insert(name.clone(), needed.clone());
                    self.is_changed = true;
                }
            }
        }
    }

    fn value(&mut self, found: &mut Value, needed: &Value, key: String) {
        match (found, needed) {
            (Value::Object(found), Value::Object(needed)) => self.object(found, needed, &key),
            (Value::Array(found), Value::Array(needed)) if is_hook_event(&key) => {
                if !found.iter().any(holds_rex_hook) {
                    found.extend(needed.iter().cloned());
                    self.is_changed = true;
                }
            }
            (Value::Array(found), Value::Array(needed)) => {
                for element in needed {
                    if !found.contains(element) {
                        found.push(element.clone());
                        self.is_changed = true;
                    }
                }
            }
            (found, needed) if found == needed => {}
            (found, needed) => self.kept.push(KeptValue {
                key,
                found: found.clone(),
                needed: needed.clone(),
            }),
        }
    }
}

/// True for the list of hook groups of one event, `hooks.<Event>`.
fn is_hook_event(key: &str) -> bool {
    key.strip_prefix("hooks.")
        .is_some_and(|event| !event.contains('.'))
}

/// True when a hook in `group` runs rex's hook command, whatever else the
/// person changed in it.
fn holds_rex_hook(group: &Value) -> bool {
    group
        .get("hooks")
        .and_then(Value::as_array)
        .is_some_and(|hooks| {
            hooks
                .iter()
                .any(|hook| hook.get("command").and_then(Value::as_str) == Some(HOOK_COMMAND))
        })
}

/// True when a hook of any event in `settings` runs rex's hook command.
fn holds_rex_hooks(settings: &Map<String, Value>) -> bool {
    settings
        .get("hooks")
        .and_then(Value::as_object)
        .is_some_and(|events| {
            events
                .values()
                .filter_map(Value::as_array)
                .flatten()
                .any(holds_rex_hook)
        })
}

/// Leaving the hooks out removes none that an earlier run wrote, because the
/// file is the person's, so they are told how to turn telemetry off.
fn warn_hooks_left_in() {
    // A warning that cannot be shown must not stop the install.
    let _ = writeln!(
        io::stderr().lock(),
        "rex init: warning: hooks left out, but hook entries from an earlier run are still in {SETTINGS_FILENAME}. Remove the entries that run rex hook to turn telemetry off."
    );
}

/// A sandbox setting, or the block on writes outside the repo. A weaker
/// value for one of these turns a protection off.
fn is_safety_key(key: &str) -> bool {
    key == "sandbox"
        || key.starts_with("sandbox.")
        || key == "permissions.blockWritesOutsideWorkingDirectories"
}

/// Tell the person about a value of theirs that rex kept. A weaker safety
/// setting fails the run, so it cannot go unseen; any other value is a
/// warning. Making the safety case a warning too is a change here only.
fn report_kept_value(kept: KeptValue) -> Option<InitError> {
    let KeptValue { key, found, needed } = kept;
    if is_safety_key(&key) {
        return Some(InitError::SafetySettingKept {
            key,
            found: found.to_string(),
            needed: needed.to_string(),
        });
    }
    // A warning that cannot be shown must not stop the install.
    let _ = writeln!(
        io::stderr().lock(),
        "rex init: warning: kept {key} = {found} in {SETTINGS_FILENAME}; SuperRex sets {needed}."
    );
    None
}

/// Write `settings` as pretty JSON, in place, creating `.claude` if needed.
fn write_settings(path: &Path, settings: &Value) -> Result<(), RexError> {
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder).map_err(|source| RexError::Io {
            path: folder.to_path_buf(),
            source,
        })?;
    }
    let mut text =
        serde_json::to_string_pretty(settings).expect("a JSON value with text keys always prints");
    text.push('\n');
    fs::write(path, text).map_err(|source| RexError::Io {
        path: path.to_path_buf(),
        source,
    })
}
