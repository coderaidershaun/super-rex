//! Merges the safety settings and the hooks into the person's own settings
//! file, so nothing of theirs is lost and a weaker setting is never hidden.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use serde_json::{Map, Value, json};

use super::InitOptions;
use super::error::InitError;
use super::file_change::FileChange;
use super::print_line_to_stderr;
use super::safety::{
    forbids_unsandboxed, is_safety_key, parent_to_open, safety_settings, stricter_value_effect,
};
use crate::capture::{REX_COMMANDS, hook_registration};
use crate::commands::links::is_symlink;
use crate::commands::test_output::{FILTER_COMMAND, PIPEFAIL_COMMAND};
use crate::error::RexError;

/// The settings file, as every message names it.
pub(super) const SETTINGS_FILENAME: &str = ".claude/settings.local.json";

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
        // The test hook puts `set -o pipefail;` in front of a test command and
        // pipes it into the filter. Claude Code checks each part of such a
        // command against the allow rules, so without these two rules each
        // test run would ask the person again.
        if let Some(Value::Object(permissions)) = settings.get_mut("permissions") {
            let allow =
                [FILTER_COMMAND, PIPEFAIL_COMMAND].map(|command| format!("Bash({command})"));
            permissions.insert("allow".into(), json!(allow));
        }
    }
    Ok(settings)
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
            let mut to_add = to_add;
            if forbids_unsandboxed(&settings) {
                // A file that keeps every command in the sandbox is never made
                // weaker, so no command is added that would run outside it.
                if let Some(Value::Object(sandbox)) = to_add.get_mut("sandbox") {
                    sandbox.remove("excludedCommands");
                }
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
                for group in needed {
                    let is_present = rex_commands_of(group)
                        .any(|command| found.iter().any(|kept| runs_command(kept, command)));
                    if !is_present {
                        found.push(group.clone());
                        self.is_changed = true;
                    }
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

/// The command of each hook in `group`.
fn commands_of(group: &Value) -> impl Iterator<Item = &str> {
    group
        .get("hooks")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|hook| hook.get("command").and_then(Value::as_str))
}

/// Each of rex's hook commands that `group` runs.
fn rex_commands_of(group: &Value) -> impl Iterator<Item = &str> {
    commands_of(group).filter(|command| REX_COMMANDS.contains(command))
}

/// True when a hook in `group` runs `command`, whatever else the person
/// changed in it, so an entry of rex's that they changed is not added again.
fn runs_command(group: &Value, command: &str) -> bool {
    commands_of(group).any(|found| found == command)
}

/// True when a hook in `group` runs one of rex's hook commands.
fn holds_rex_hook(group: &Value) -> bool {
    rex_commands_of(group).next().is_some()
}

/// True when a hook of any event in `settings` runs one of rex's hook commands.
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
/// file is the person's, so they are told how to turn each hook off.
fn warn_hooks_left_in() {
    print_line_to_stderr(&format!(
        "rex init: warning: hooks left out, but hook entries from an earlier run are still in {SETTINGS_FILENAME}. Remove the entries that run rex hook, rex gate or rex test-hook to turn telemetry, the spend gate and the test output filter off."
    ));
}

/// Tell the person about a value of theirs that rex kept. A weaker safety
/// setting fails the run, so it cannot go unseen; any other value is a
/// warning. Making the safety case a warning too is a change here only.
fn report_kept_value(kept: KeptValue) -> Option<InitError> {
    let KeptValue { key, found, needed } = kept;
    if let Some(effect) = stricter_value_effect(&key, &found) {
        print_line_to_stderr(&format!(
            "rex init: warning: kept {key} = {found} in {SETTINGS_FILENAME}, {effect}."
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
