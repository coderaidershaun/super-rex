//! Writes `.rex/config.json`, which the plugin reads when a session starts: whether
//! the record is kept, whether jobs merge with no question, the depth in force and
//! the oldest Claude Code SuperRex runs on.

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

/// The line a run prints while auto-merge is on, so the person sees on each run what agents may do.
pub(super) const AUTO_MERGE_NOTICE: &str = "rex init: auto-merge is on, so every job merges to its base branch with no merge question once its checks pass, a builder's own job and each job of a master tree; a policy with merge never, and your own words merge: no, still hold; run rex init --no-auto-merge to turn it off";

/// What the config holds after a run.
#[derive(Clone, Copy, Debug)]
pub(super) struct After {
    /// The record of jobs is kept, which decides whether the store is set up.
    pub(super) is_storage_on: bool,
    /// Every job merges with no merge question.
    pub(super) is_auto_merge: bool,
}

/// What this run of `rex init` asks of the config.
pub(super) struct Wanted {
    /// Storage after the flags and the question. A config that already turns storage on keeps it on.
    pub(super) is_storage_on: bool,
    /// The depth in force in the settings file after this run.
    pub(super) depth: u32,
    /// `--auto-merge` gives `Some(true)`, `--no-auto-merge` `Some(false)`; with neither flag the config keeps what it holds, and a new one gets `false`.
    pub(super) auto_merge: Option<bool>,
}

/// Make sure `.rex/config.json` holds what `wanted` asks, with every other
/// key of the file kept. Gives what the config holds after the run.
///
/// # Errors
/// [`InitError::ConfigUnreadable`], [`InitError::ConfigNotJson`] or
/// [`InitError::ConfigNotObject`] for a file that cannot be merged, which is
/// left as it is; [`InitError::Rex`] when the folder or the file cannot be written.
pub(super) fn ensure_config(cwd: &Path, wanted: Wanted) -> Result<(FileChange, After), InitError> {
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
    let is_auto_merge = wanted
        .auto_merge
        .unwrap_or_else(|| config.get("auto_merge") == Some(&json!(true)));
    config.insert("auto_merge".into(), json!(is_auto_merge));
    let after = After {
        is_storage_on,
        is_auto_merge,
    };
    config.insert(
        "min_claude_code".into(),
        json!(MIN_CLAUDE_CODE_VERSION.to_string()),
    );
    if config == before {
        return Ok((FileChange::Unchanged, after));
    }

    let folder = cwd.join(".rex");
    fs::create_dir_all(&folder).map_err(|source| RexError::Io {
        path: folder,
        source,
    })?;
    let text = format!("{:#}\n", Value::Object(config));
    match write_unless_linked(&path, text.as_bytes())? {
        ProjectWrite::Written => Ok((change, after)),
        ProjectWrite::SkippedLink => Ok((FileChange::Unchanged, after)),
    }
}

/// True when the config in `cwd` already keeps the record of jobs. A missing or
/// unreadable config keeps none; the config step reports a file it cannot read.
pub(super) fn keeps_record(cwd: &Path) -> bool {
    matches!(read_config(&cwd.join(CONFIG_FILENAME)), Ok(Some(config)) if config.get("storage") == Some(&json!("on")))
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
