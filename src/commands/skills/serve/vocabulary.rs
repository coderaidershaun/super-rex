//! The version of the shared vocabulary that `head` reports: the copy embedded in the binary,
//! or `vocab/terms.json` beside a library folder.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::ServeError;
use crate::commands::skills::metadata;

/// Where a library's vocabulary version comes from.
pub(super) enum Vocabulary {
    /// The `terms.json` that the binary embeds, for the embedded library.
    Embedded,
    /// The `vocab/terms.json` beside the first folder, which may not exist.
    Beside(PathBuf),
    /// A first folder with no parent, so nothing can sit beside it.
    Missing,
}

impl Vocabulary {
    pub(super) fn beside(folder: &Path) -> Vocabulary {
        folder.parent().map_or(Vocabulary::Missing, |parent| {
            Vocabulary::Beside(parent.join("vocab/terms.json"))
        })
    }

    /// The version, or `None` when no vocabulary file sits beside the folder.
    pub(super) fn version(&self) -> Result<Option<String>, ServeError> {
        match self {
            Vocabulary::Embedded => Ok(Some(metadata::embedded_version())),
            Vocabulary::Beside(path) => version_of(path),
            Vocabulary::Missing => Ok(None),
        }
    }
}

/// The top-level `"version"` string of the file at `path`. A file that does not
/// exist gives `None`; a file that cannot be read, is not JSON, or has no string
/// `version` is an error, because a silent `None` would change the library's
/// version without a trace.
fn version_of(path: &Path) -> Result<Option<String>, ServeError> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(ServeError::VocabularyRead {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    let invalid = |reason: String| ServeError::VocabularyInvalid {
        path: path.to_path_buf(),
        reason,
    };
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|error| invalid(error.to_string()))?;
    match value.get("version").and_then(serde_json::Value::as_str) {
        Some(version) => Ok(Some(version.to_string())),
        None => Err(invalid(
            "it has no top-level string \"version\"".to_string(),
        )),
    }
}
