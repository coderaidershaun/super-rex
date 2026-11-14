//! Merges the safety settings, the deny rules and the sub-agent depth into the
//! person's own settings file, so nothing of theirs is lost and a weaker setting is never hidden.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use serde_json::{Map, Value};

use super::error::InitError;
use super::file_change::FileChange;
use super::safety::{
    forbids_unsandboxed, is_safety_key, parent_to_open, safety_settings, stricter_value_effect,
};
use super::{InitOptions, MASTER_DEPTH, print_line_to_stderr};
use crate::commands::links::is_symlink;
use crate::commands::worktree::{SETTINGS_FILE, SettingsLock, rex_worktrees, worktrees_root};
use crate::error::RexError;

/// The settings file, as every message names it.
pub(super) const SETTINGS_FILENAME: &str = SETTINGS_FILE;

/// Entries that ran every git command, and so its hooks, outside the sandbox.
/// A person added them as a stopgap while git could not commit in a job
/// worktree in the sandbox. Rex now lists its worktrees, so git works there,
/// and these are taken out rather than kept. Narrower git rules stay.
const SUPERSEDED_EXCLUDED: [&str; 2] = ["git *", "git:*"];
const SUPERSEDED_ALLOW: [&str; 2] = ["Bash(git *)", "Bash(git:*)"];

/// Why a superseded entry was taken out, as the notice says it.
const SUPERSEDED_REASON: &str = "it ran git and its hooks outside the sandbox, and rex now registers its worktrees, so git works inside it";

/// The variable of Claude Code that caps how deep sub-agents may start sub-agents.
const DEPTH_VARIABLE: &str = "CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH";

/// The hook commands that rex 0.7 registered. Those commands are gone, so each
/// entry would fail on every event; an entry is rex's when its command is exactly one of these.
const OLD_HOOK_COMMANDS: [&str; 3] = [
    "rex hook >/dev/null 2>&1 || true",
    "rex gate 2>/dev/null || true",
    "rex test-hook 2>/dev/null || true",
];

/// The places of the repo at `cwd` that its settings name, from its
/// canonical path: the folder above it as text, that folder again when agents
/// may write in it, the job worktrees folder, and the worktrees in it.
struct RepoPlaces {
    parent: Option<String>,
    root: Option<std::path::PathBuf>,
    worktrees: Vec<String>,
    opened_parent: Option<String>,
}

impl RepoPlaces {
    fn of(cwd: &Path) -> Result<Self, RexError> {
        let repo = cwd.canonicalize().map_err(|source| RexError::Io {
            path: cwd.to_path_buf(),
            source,
        })?;
        let parent = repo.parent();
        Ok(Self {
            parent: parent.and_then(Path::to_str).map(str::to_owned),
            root: worktrees_root(&repo),
            // A path that is not UTF-8 cannot be written into the file.
            worktrees: rex_worktrees(&repo)
                .iter()
                .filter_map(|dir| dir.to_str().map(str::to_owned))
                .collect(),
            opened_parent: parent.and_then(parent_to_open),
        })
    }
}

/// What rex adds to the settings file of the repo at `places`, in the order
/// it is added. The depth is not here: it has a rule of its own.
fn settings_to_add(places: &RepoPlaces, options: &InitOptions) -> Map<String, Value> {
    safety_settings(
        places.opened_parent.as_deref(),
        &places.worktrees,
        options.is_strict,
    )
}

/// Make sure the settings file of the repo at `cwd` holds the safety settings,
/// the deny rules, the job worktrees and the sub-agent depth, and none of the
/// hook entries of rex 0.7 or the stopgap entries that ran git outside the
/// sandbox; each stopgap entry and each worktree that is gone is taken out
/// with a notice on stderr, and never fails the run. Every other key and value of the person's is kept, in its order, and
/// only what is missing is added. When nothing changes, the file is not
/// written at all. Gives the depth in force after the merge.
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
    options: &InitOptions,
    failures: &mut Vec<InitError>,
) -> Result<(FileChange, u32), RexError> {
    let path = cwd.join(SETTINGS_FILENAME);
    let places = RepoPlaces::of(cwd)?;
    let mut to_add = settings_to_add(&places, options);
    // `rex worktree` writes the same file; the lock keeps one write from
    // dropping the other's.
    let _lock = SettingsLock::take(cwd)?;

    let (mut settings, change) = match read_settings(&path) {
        Err(failure) => {
            failures.push(failure);
            return Ok((FileChange::Unchanged, options.depth.unwrap_or(MASTER_DEPTH)));
        }
        Ok(None) => (Map::new(), FileChange::Created),
        Ok(Some(settings)) => (settings, FileChange::Updated),
    };
    if forbids_unsandboxed(&settings) {
        // A file that keeps every command in the sandbox is never made
        // weaker, so no command is added that would run outside it.
        if let Some(Value::Object(sandbox)) = to_add.get_mut("sandbox") {
            sandbox.remove("excludedCommands");
        }
        if let Some(Value::Object(permissions)) = to_add.get_mut("permissions") {
            permissions.remove("allow");
        }
    }
    let mut merge = Merge::default();
    merge.is_changed |= remove_old_hooks(&mut settings);
    for notice in remove_superseded_entries(&mut settings, places.parent.as_deref()) {
        merge.is_changed = true;
        print_line_to_stderr(&notice);
    }
    if let Some(root) = &places.root {
        for notice in remove_gone_worktrees(&mut settings, root, &places.worktrees) {
            merge.is_changed = true;
            print_line_to_stderr(&notice);
        }
    }
    merge.object(&mut settings, &to_add, "");
    let depth = merge.depth(&mut settings, options.depth);
    failures.extend(merge.kept.into_iter().filter_map(report_kept_value));
    if !merge.is_changed {
        return Ok((FileChange::Unchanged, depth));
    }

    if let Some(link) = linked_part(cwd) {
        failures.push(InitError::SettingsLinked { link });
        return Ok((FileChange::Unchanged, depth));
    }
    write_settings(&path, &Value::Object(settings))?;
    Ok((change, depth))
}

/// Takes each hook of rex 0.7 out of `settings`, then each group and event
/// that it leaves empty. Hooks of the person's own stay. True when one was taken out.
fn remove_old_hooks(settings: &mut Map<String, Value>) -> bool {
    let Some(Value::Object(events)) = settings.get_mut("hooks") else {
        return false;
    };
    let mut is_changed = false;
    for groups in events.values_mut() {
        let Value::Array(groups) = groups else {
            continue;
        };
        for group in groups.iter_mut() {
            let Some(Value::Array(hooks)) = group.get_mut("hooks") else {
                continue;
            };
            let before = hooks.len();
            hooks.retain(|hook| !is_old_hook(hook));
            is_changed |= hooks.len() != before;
        }
        groups.retain(
            |group| !matches!(group.get("hooks"), Some(Value::Array(hooks)) if hooks.is_empty()),
        );
    }
    if !is_changed {
        return false;
    }
    events.retain(|_, groups| !matches!(groups, Value::Array(groups) if groups.is_empty()));
    if events.is_empty() {
        settings.remove("hooks");
    }
    true
}

/// Takes the stopgap entries out of `settings`: every git command outside the
/// sandbox, and the rule that let an agent `cd` anywhere in `parent`, the
/// folder above the repo. Gives one notice for each entry taken out.
fn remove_superseded_entries(
    settings: &mut Map<String, Value>,
    parent: Option<&str>,
) -> Vec<String> {
    let cd_rule = parent.map(|parent| format!("Bash(cd {parent}/*)"));
    let allow: Vec<&str> = SUPERSEDED_ALLOW
        .into_iter()
        .chain(cd_rule.as_deref())
        .collect();
    let mut notices = Vec::new();
    for (section, key, superseded) in [
        ("sandbox", "excludedCommands", &SUPERSEDED_EXCLUDED[..]),
        ("permissions", "allow", &allow[..]),
    ] {
        let Some(Value::Array(entries)) = settings
            .get_mut(section)
            .and_then(|section| section.get_mut(key))
        else {
            continue;
        };
        entries.retain(|entry| {
            let is_superseded = entry.as_str().is_some_and(|text| superseded.contains(&text));
            if is_superseded {
                notices.push(format!(
                    "rex init: removed {entry} from {section}.{key} in {SETTINGS_FILENAME}: {SUPERSEDED_REASON}."
                ));
            }
            !is_superseded
        });
    }
    notices
}

/// Takes out of `permissions.additionalDirectories` each entry inside `root`,
/// the job worktrees folder, that is not one of `live`, the worktrees there
/// now: the worktree was removed, or a job stopped before it removed it.
/// An entry outside `root` is the person's and stays. Gives one notice for
/// each entry taken out.
fn remove_gone_worktrees(
    settings: &mut Map<String, Value>,
    root: &Path,
    live: &[String],
) -> Vec<String> {
    let Some(Value::Array(entries)) = settings
        .get_mut("permissions")
        .and_then(|permissions| permissions.get_mut("additionalDirectories"))
    else {
        return Vec::new();
    };
    let mut notices = Vec::new();
    entries.retain(|entry| {
        let Some(text) = entry.as_str() else {
            return true;
        };
        let path = Path::new(text);
        let is_gone = path != root && path.starts_with(root) && !live.iter().any(|dir| dir == text);
        if is_gone {
            notices.push(format!(
                "rex init: removed {entry} from permissions.additionalDirectories in {SETTINGS_FILENAME}: it is no longer a worktree of this repo."
            ));
        }
        !is_gone
    });
    notices
}

fn is_old_hook(hook: &Value) -> bool {
    hook.get("command")
        .and_then(Value::as_str)
        .is_some_and(|command| OLD_HOOK_COMMANDS.contains(&command))
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
    /// Sets the depth of `env` and gives the depth in force. `asked`, from
    /// `--depth`, is written as given. With none, a value of the person's of at
    /// least the master depth is kept as it is; a lower one is kept with a
    /// warning, because a master tree needs that depth.
    fn depth(&mut self, settings: &mut Map<String, Value>, asked: Option<u32>) -> u32 {
        let env = settings
            .entry("env")
            .or_insert_with(|| Value::Object(Map::new()));
        let Value::Object(env) = env else {
            self.kept.push(KeptValue {
                key: "env".to_owned(),
                found: env.clone(),
                needed: serde_json::json!({ DEPTH_VARIABLE: MASTER_DEPTH.to_string() }),
            });
            return 0;
        };
        let found = env.get(DEPTH_VARIABLE).cloned();
        if asked.is_none()
            && let Some(found) = &found
        {
            let depth = depth_of(found).unwrap_or(0);
            if depth < MASTER_DEPTH {
                warn_low_depth(found);
            }
            return depth;
        }
        let wanted = asked.unwrap_or(MASTER_DEPTH);
        let value = Value::String(wanted.to_string());
        if found.as_ref() != Some(&value) {
            env.insert(DEPTH_VARIABLE.to_owned(), value);
            self.is_changed = true;
        }
        wanted
    }

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

/// The depth of a stored value: a whole number, as text (Claude Code reads
/// environment values as text) or as a JSON number.
fn depth_of(value: &Value) -> Option<u32> {
    match value {
        Value::String(text) => text.trim().parse().ok(),
        Value::Number(number) => number.as_u64().and_then(|n| u32::try_from(n).ok()),
        _ => None,
    }
}

/// A depth below the master's is the person's to keep, so it is a warning that
/// names what it costs.
fn warn_low_depth(found: &Value) {
    print_line_to_stderr(&format!(
        "rex init: warning: kept env.{DEPTH_VARIABLE} = {found} in {SETTINGS_FILENAME}, below {MASTER_DEPTH}, so /rex master refuses to start: its workers start their own sub-agents at depth {MASTER_DEPTH}; run rex init --depth {MASTER_DEPTH} to allow it."
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn object(value: Value) -> Map<String, Value> {
        let Value::Object(map) = value else {
            panic!("not an object")
        };
        map
    }

    #[test]
    fn superseded_entries_go_wherever_they_stand_and_twice_over() {
        let mut settings = object(json!({
            "sandbox": { "excludedCommands": ["git *", "rex init*", "git *", "git:*"] },
            "permissions": { "allow": ["Bash(git *)", "Bash(cd /p/*)", "Bash(cd /q/*)", "Bash(git log *)"] }
        }));

        let notices = remove_superseded_entries(&mut settings, Some("/p"));

        assert_eq!(notices.len(), 5, "got: {notices:?}");
        assert_eq!(
            Value::Object(settings),
            json!({
                "sandbox": { "excludedCommands": ["rex init*"] },
                "permissions": { "allow": ["Bash(cd /q/*)", "Bash(git log *)"] }
            })
        );
    }

    #[test]
    fn a_file_with_no_list_or_another_type_is_left_as_it_is() {
        for value in [
            json!({}),
            json!({ "sandbox": { "excludedCommands": "git *" }, "permissions": { "allow": [] } }),
            json!({ "sandbox": "on", "permissions": ["Bash(git *)"] }),
        ] {
            let mut settings = object(value.clone());

            let notices = remove_superseded_entries(&mut settings, None);

            assert!(notices.is_empty(), "got: {notices:?}");
            assert_eq!(Value::Object(settings), value);
        }
    }

    #[test]
    fn only_entries_inside_the_worktrees_folder_that_are_gone_are_taken_out() {
        let mut settings = object(json!({
            "permissions": { "additionalDirectories": [
                "/p", "/p/app.worktrees", "/p/app.worktrees/live", "/p/app.worktrees/gone",
                "/p/app.worktrees-other/x", 7
            ] }
        }));

        let notices = remove_gone_worktrees(
            &mut settings,
            Path::new("/p/app.worktrees"),
            &["/p/app.worktrees/live".to_owned()],
        );

        assert_eq!(notices.len(), 1, "got: {notices:?}");
        assert_eq!(
            settings["permissions"]["additionalDirectories"],
            json!([
                "/p",
                "/p/app.worktrees",
                "/p/app.worktrees/live",
                "/p/app.worktrees-other/x",
                7
            ])
        );
    }
}
