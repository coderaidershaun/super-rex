//! The repo's list of registered worktrees, kept in `permissions.additionalDirectories`
//! of `.claude/settings.local.json`; Claude Code's sandbox lets git write only for listed worktrees.

use std::fs::{self, File, OpenOptions};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use super::error::WorktreeError;
use crate::commands::links::is_symlink;
use crate::error::RexError;

/// The settings file, from the repo root.
pub(crate) const SETTINGS_FILE: &str = ".claude/settings.local.json";

/// The lock that every rex writer of the settings file holds, from the repo
/// root. It is in `.git`, so it is never committed or shown as untracked.
const LOCK_FILE: &str = ".git/rex-settings.lock";

/// Held while one rex reads and writes the settings file of a repo, so two
/// writers at once never drop each other's change. The operating system
/// releases it when this value drops or the process ends, so a crash never
/// leaves it held.
pub(crate) struct SettingsLock {
    _file: File,
}

impl SettingsLock {
    /// Wait until no other rex holds the lock of the repo at `repo`, then take it.
    ///
    /// # Errors
    /// [`RexError::SymbolicLink`] if the lock file is a link, or
    /// [`RexError::Io`] if it cannot be opened or locked.
    pub(crate) fn take(repo: &Path) -> Result<Self, RexError> {
        let path = repo.join(LOCK_FILE);
        if is_symlink(&path) {
            return Err(RexError::SymbolicLink { path });
        }
        let io = |source| RexError::Io {
            path: path.clone(),
            source,
        };
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&path)
            .map_err(io)?;
        file.lock().map_err(io)?;
        Ok(Self { _file: file })
    }
}

/// What a change of the list did to the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Change {
    Added,
    Removed,
    Unchanged,
}

impl Change {
    pub(super) fn word(self) -> &'static str {
        match self {
            Change::Added => "added",
            Change::Removed => "removed",
            Change::Unchanged => "unchanged",
        }
    }
}

/// The settings file of one repo, read under its lock, so that it can be
/// checked before any worktree is made and written once after.
pub(super) struct Registry {
    _lock: SettingsLock,
    path: PathBuf,
    settings: Map<String, Value>,
}

impl Registry {
    /// Take the lock of the repo at `repo`, then read its settings file. A
    /// file that is not there reads as empty.
    ///
    /// # Errors
    /// The file or `.claude` is a link, the file cannot be read, is not a JSON
    /// object, or holds a `permissions` or `additionalDirectories` of another type.
    pub(super) fn open(repo: &Path) -> Result<Self, WorktreeError> {
        let lock = SettingsLock::take(repo)?;
        for part in [".claude", SETTINGS_FILE] {
            let path = repo.join(part);
            if is_symlink(&path) {
                return Err(RexError::SymbolicLink { path }.into());
            }
        }
        let path = repo.join(SETTINGS_FILE);
        let settings = match fs::read(&path) {
            Err(source) if source.kind() == ErrorKind::NotFound => Map::new(),
            Err(source) => return Err(RexError::Io { path, source }.into()),
            Ok(bytes) => match serde_json::from_slice(&bytes) {
                Ok(Value::Object(settings)) => settings,
                Ok(_) => return Err(WorktreeError::SettingsNotObject),
                Err(source) => return Err(WorktreeError::SettingsNotJson { source }),
            },
        };
        let mut registry = Self {
            _lock: lock,
            path,
            settings,
        };
        registry.entries()?;
        Ok(registry)
    }

    /// The list, made when it is not there yet.
    fn entries(&mut self) -> Result<&mut Vec<Value>, WorktreeError> {
        let permissions = self
            .settings
            .entry("permissions")
            .or_insert_with(|| Value::Object(Map::new()));
        let Value::Object(permissions) = permissions else {
            return Err(WorktreeError::SettingsNotList);
        };
        let entries = permissions
            .entry("additionalDirectories")
            .or_insert_with(|| Value::Array(Vec::new()));
        let Value::Array(entries) = entries else {
            return Err(WorktreeError::SettingsNotList);
        };
        Ok(entries)
    }

    /// Add `dir` to the list and write the file, unless it is listed already.
    ///
    /// # Errors
    /// The file cannot be written.
    pub(super) fn add(mut self, dir: &str) -> Result<Change, WorktreeError> {
        let entry = Value::String(dir.to_owned());
        let entries = self.entries()?;
        if entries.contains(&entry) {
            return Ok(Change::Unchanged);
        }
        entries.push(entry);
        self.write()?;
        Ok(Change::Added)
    }

    /// Take `dir` out of the list and write the file, when it is listed.
    ///
    /// # Errors
    /// The file cannot be written.
    pub(super) fn remove(mut self, dir: &str) -> Result<Change, WorktreeError> {
        let entries = self.entries()?;
        let before = entries.len();
        entries.retain(|entry| entry.as_str() != Some(dir));
        if entries.len() == before {
            return Ok(Change::Unchanged);
        }
        self.write()?;
        Ok(Change::Removed)
    }

    /// Write the file as pretty JSON through a file beside it, so Claude
    /// Code, which watches it, never reads half of it.
    fn write(&self) -> Result<(), RexError> {
        let folder = self.path.parent().unwrap_or(Path::new("."));
        fs::create_dir_all(folder).map_err(|source| RexError::Io {
            path: folder.to_path_buf(),
            source,
        })?;
        let next = self.path.with_extension("json.rex-next");
        let text = format!("{:#}\n", Value::Object(self.settings.clone()));
        fs::write(&next, text).map_err(|source| RexError::Io {
            path: next.clone(),
            source,
        })?;
        fs::rename(&next, &self.path).map_err(|source| RexError::Io {
            path: self.path.clone(),
            source,
        })
    }
}
