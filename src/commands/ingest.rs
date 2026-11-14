//! `rex ingest`: the one way the plugin writes the store. It reads one batch of
//! events on standard input and prints the session's last sequence number.

use std::fs;
use std::io::{self, ErrorKind, Read};
use std::path::Path;

use serde_json::Value;

use super::skills::describe;
use super::stdout::print_line_to_stdout;
use crate::ingest::{IngestError, ingest};
use crate::store::{Store, StoreError, StorePaths};

/// The plugin's write path: take one batch of events into the store.
#[derive(clap::Args)]
#[command(
    after_help = "Reads one batch as JSON on standard input and prints `last_sequence=<n>`. A refused batch changes nothing: exit 2 for a batch that does not fit (its form, a gap in seq, a conflict), 3 for a wrong secret, 4 when storage is off or there is no store. Agents may not run it."
)]
pub struct IngestOptions {}

/// # Errors
/// [`IngestError`]: a refusal with its own exit code, or a failure to read
/// the batch, the config or the store.
pub fn run(cwd: &Path, _options: IngestOptions) -> Result<(), IngestError> {
    check_storage_on(cwd)?;
    let paths = StorePaths::discover(cwd).map_err(|error| match error {
        StoreError::NotFound { tried } => IngestError::NoStore {
            path: tried
                .into_iter()
                .next()
                .map(|tried| tried.path)
                .unwrap_or_default(),
        },
        other => other.into(),
    })?;
    let mut store = Store::open(&paths)?;
    let mut input = Vec::new();
    io::stdin()
        .read_to_end(&mut input)
        .map_err(|source| IngestError::Read { source })?;
    let last = ingest(&mut store, &input, &describe)?;
    print_line_to_stdout(&format!("last_sequence={last}"))?;
    Ok(())
}

/// Storage is on only when `.rex/config.json` of the repo says so; a repo with
/// no config keeps no record.
fn check_storage_on(cwd: &Path) -> Result<(), IngestError> {
    let root = StorePaths::locate(cwd)?.root;
    let text = match fs::read(root.join(".rex/config.json")) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => return Err(IngestError::StorageOff),
        Err(source) => return Err(IngestError::ConfigUnreadable { root, source }),
    };
    // A config the plugin cannot read leaves it inert, so storage is off too.
    let is_on = serde_json::from_slice::<Value>(&text)
        .ok()
        .and_then(|config| config.get("storage").cloned())
        == Some(Value::from("on"));
    if is_on {
        Ok(())
    } else {
        Err(IngestError::StorageOff)
    }
}
