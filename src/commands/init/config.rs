//! Writes `.rex/config.json`, which the plugin reads when a session starts: whether
//! the record is kept, the depth in force and the oldest Claude Code SuperRex runs on.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use serde_json::{Map, Value, json};

use super::error::InitError;
use super::file_change::{FileChange, ProjectWrite, write_unless_linked};
use super::preflight::MIN_CLAUDE_CODE_VERSION;
use crate::error::RexError;

/// The config file, as every message names it.
pub(super) const CONFIG_FILENAME: &str = ".rex/config.json";

/// The form of the file this rex writes.
const CONFIG_VERSION: u64 = 1;

/// What this run of `rex init` asks of the config.
pub(super) struct Wanted {
    /// `--with-storage`. Without it, a config that already turns storage on keeps it on.
    pub(super) is_storage_on: bool,
    /// The depth in force in the settings file after this run.
    pub(super) depth: u32,
}

/// Make sure `.rex/config.json` holds what `wanted` asks, with every other
/// key of the file kept. Gives whether storage is on after the run, which
/// decides whether the store is set up.
///
/// # Errors
/// [`InitError::ConfigUnreadable`], [`InitError::ConfigNotJson`] or
/// [`InitError::ConfigNotObject`] for a file that cannot be merged, which is
/// left as it is; [`InitError::Rex`] when the folder or the file cannot be written.
pub(super) fn ensure_config(cwd: &Path, wanted: Wanted) -> Result<(FileChange, bool), InitError> {
    let path = cwd.join(CONFIG_FILENAME);
    let (mut config, change) = match read_config(&path)? {
        None => (Map::new(), FileChange::Created),
        Some(config) => (config, FileChange::Updated),
    };
    let before = config.clone();

    let was_on = config.get("storage") == Some(&json!("on"));
    let is_storage_on = wanted.is_storage_on || was_on;
    let storage = if is_storage_on { "on" } else { "off" };
    config.insert("version".into(), json!(CONFIG_VERSION));
    config.insert("storage".into(), json!(storage));
    config.insert("depth".into(), json!(wanted.depth));
    config.entry("jobs_from_sdk").or_insert_with(|| json!(true));
    config.insert(
        "min_claude_code".into(),
        json!(MIN_CLAUDE_CODE_VERSION.to_string()),
    );
    if config == before {
        return Ok((FileChange::Unchanged, is_storage_on));
    }

    let folder = cwd.join(".rex");
    fs::create_dir_all(&folder).map_err(|source| RexError::Io {
        path: folder,
        source,
    })?;
    let text = format!("{:#}\n", Value::Object(config));
    match write_unless_linked(&path, text.as_bytes())? {
        ProjectWrite::Written => Ok((change, is_storage_on)),
        ProjectWrite::SkippedLink => Ok((FileChange::Unchanged, is_storage_on)),
    }
}

/// The config, or `None` when there is no file. A link is read through, which
/// changes nothing.
fn read_config(path: &Path) -> Result<Option<Map<String, Value>>, InitError> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(source) if source.kind() == ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(InitError::ConfigUnreadable { source }),
    };
    match serde_json::from_slice(&bytes) {
        Ok(Value::Object(config)) => Ok(Some(config)),
        Ok(_) => Err(InitError::ConfigNotObject),
        Err(source) => Err(InitError::ConfigNotJson { source }),
    }
}
