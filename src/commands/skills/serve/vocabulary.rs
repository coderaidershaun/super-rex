//! Reads the version of the shared vocabulary from `vocab/terms.json` beside a library folder.

use std::fs;
use std::io;
use std::path::Path;

use super::ServeError;

/// The top-level `"version"` string of the file at `path`. A file that does not
/// exist gives `None`; a file that cannot be read, is not JSON, or has no string
/// `version` is an error, because a silent `None` would change the library's
/// version without a trace.
pub(super) fn version_of(path: &Path) -> Result<Option<String>, ServeError> {
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
