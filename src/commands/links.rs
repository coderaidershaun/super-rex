//! Rex never writes through a symbolic link, because the write lands on
//! whatever the link points at, which can be a file outside the project.

use std::fs;
use std::path::Path;

/// True when `path` is a symbolic link, whether or not the link points at
/// something that exists.
pub(super) fn is_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink())
}
