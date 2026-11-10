//! Merges the safety settings and the hooks into the person's own settings
//! file, so nothing of theirs is lost and a weaker setting is never hidden.

use std::io::ErrorKind;
use std::path::Path;
use std::{env, fs};

use serde_json::{Map, Value, json};

use super::InitOptions;
use super::error::InitError;
use super::file_change::FileChange;
use super::print_line_to_stderr;
use crate::capture::{HOOK_COMMAND, hook_registration};
use crate::commands::links::is_symlink;
use crate::error::RexError;

/// The settings file, as every message names it.
pub(super) const SETTINGS_FILENAME: &str = ".claude/settings.local.json";

/// Where trivy keeps its vulnerability database. Without write access to it,
/// each push fails once the database is older than about a day.
#[cfg(target_os = "macos")]
const TRIVY_CACHE: &str = "~/Library/Caches/trivy";
#[cfg(not(target_os = "macos"))]
const TRIVY_CACHE: &str = "~/.cache/trivy";

/// The safety settings, with `parent` opened to agents when there is one.
/// The `~/` paths stay as written, because Claude Code reads them the same
/// way from any folder a session starts in.
///
/// A sandboxed command reaches the network only through the sandbox's proxy,
/// so a program that opens its own connection, such as a WebSocket client or
/// a database driver, cannot connect. Unless `is_strict`, an agent may
/// therefore run a command again outside the sandbox.
fn safety_settings(parent: Option<&str>, is_strict: bool) -> Map<String, Value> {
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
        "allowUnsandboxedCommands": !is_strict,
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
fn settings_to_add(cwd: &Path, options: InitOptions) -> Result<Map<String, Value>, RexError> {
    let repo = cwd.canonicalize().map_err(|source| RexError::Io {
        path: cwd.to_path_buf(),
        source,
    })?;
    let parent = repo.parent().and_then(parent_to_open);
    let mut settings = safety_settings(parent.as_deref(), options.is_strict);
    if !options.is_without_hooks {
        settings.insert("hooks".into(), hook_registration());
    }
    Ok(settings)
}

/// `parent`, the folder that holds the repo, as text, so agents may write in
/// it and edit sibling repos. `None`, with a warning, when it holds the home
/// folder or the home folder is unknown, because agents could then write to
/// keys, shell profiles and installed programs.
fn parent_to_open(parent: &Path) -> Option<String> {
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

/// Make sure the settings file of the repo at `cwd` holds the safety settings
/// and, unless `options.is_without_hooks`, the hook registration. Every key
/// and value of the person's is kept, in its order, and only what is missing
/// is added. When nothing is missing, the file is not written at all.
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
    options: InitOptions,
    failures: &mut Vec<InitError>,
) -> Result<FileChange, RexError> {
    let path = cwd.join(SETTINGS_FILENAME);
    let to_add = settings_to_add(cwd, options)?;

    let (settings, change) = match read_settings(&path) {
        Err(failure) => {
            failures.push(failure);
            return Ok(FileChange::Unchanged);
        }
        Ok(None) => (to_add, FileChange::Created),
        Ok(Some(mut settings)) => {
            if options.is_without_hooks && holds_rex_hooks(&settings) {
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
        parent_key: &str,
    ) {
        for (name, needed) in to_add {
            let key = if parent_key.is_empty() {
                name.clone()
            } else {
                format!("{parent_key}.{name}")
            };
            if let Some(found) = settings.get_mut(name) {
                self.value(found, needed, key);
            } else {
                settings.insert(name.clone(), needed.clone());
                self.is_changed = true;
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
    print_line_to_stderr(&format!(
        "rex init: warning: hooks left out, but hook entries from an earlier run are still in {SETTINGS_FILENAME}. Remove the entries that run rex hook to turn telemetry off."
    ));
}

/// The one safety setting outside `sandbox`.
const BLOCK_WRITES_KEY: &str = "permissions.blockWritesOutsideWorkingDirectories";

/// A sandbox setting, the block on writes outside the repo, or a key above
/// that block. A weaker value for one of these turns a protection off. A
/// value of another type above the block, such as `permissions` as a text,
/// keeps rex from setting the block at all, which is as weak.
fn is_safety_key(key: &str) -> bool {
    let is_in_sandbox = key == "sandbox" || key.starts_with("sandbox.");
    let is_at_or_above_block = BLOCK_WRITES_KEY
        .strip_prefix(key)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('.'));
    is_in_sandbox || is_at_or_above_block
}

/// The setting that lets an agent run a command again outside the sandbox.
const UNSANDBOXED_KEY: &str = "sandbox.allowUnsandboxedCommands";

/// Tell the person about a value of theirs that rex kept. A weaker safety
/// setting fails the run, so it cannot go unseen; any other value is a
/// warning. Making the safety case a warning too is a change here only.
fn report_kept_value(kept: KeptValue) -> Option<InitError> {
    let KeptValue { key, found, needed } = kept;
    // The one safety value of the person's that is stricter than the one rex
    // sets: it turns no protection off, so it does not fail the run.
    if key == UNSANDBOXED_KEY && found == Value::Bool(false) {
        print_line_to_stderr(&format!(
            "rex init: warning: kept {key} = false in {SETTINGS_FILENAME}, so no agent can run a command outside the sandbox, such as a test that opens its own network connection; set it to true to let agents do that."
        ));
        return None;
    }
    if is_safety_key(&key) {
        return Some(InitError::SafetySettingKept {
            key,
            found: found.to_string(),
            needed: needed.to_string(),
        });
    }
    print_line_to_stderr(&format!(
        "rex init: warning: kept {key} = {found} in {SETTINGS_FILENAME}; SuperRex sets {needed}."
    ));
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
    // `{:#}` prints a JSON value as pretty JSON with a 2-space indent. A
    // value printed into a string cannot fail, so there is no error to handle.
    let text = format!("{settings:#}\n");
    fs::write(path, text).map_err(|source| RexError::Io {
        path: path.to_path_buf(),
        source,
    })
}
