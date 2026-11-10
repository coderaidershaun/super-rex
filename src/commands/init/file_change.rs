//! What `rex init` did to one project file it owns a block in, the marker
//! search that decides whether the block is already there, and the write that
//! refuses to go through a symbolic link.

use std::fs;
use std::io::{self, Write};
use std::path::Path;

use crate::commands::links::is_symlink;
use crate::error::RexError;

/// The outcome of making sure a file has the rex block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FileChange {
    Created,
    Updated,
    Unchanged,
}

/// True when `contents` holds `marker` anywhere. Works on bytes so a file
/// that is not valid UTF-8 can still be checked.
pub(super) fn contains_marker(contents: &[u8], marker: &str) -> bool {
    contents
        .windows(marker.len())
        .any(|window| window == marker.as_bytes())
}

/// The outcome of a guarded write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ProjectWrite {
    Written,
    /// The path is a symbolic link, so nothing was written through it.
    SkippedLink,
}

/// Write `contents` to `path`, unless `path` is a symbolic link.
///
/// A link is left alone and named on stderr. Writing through it would land on
/// whatever it points at: a file outside the project for a link that resolves,
/// and a new file outside the project for one that does not. Neither is what
/// the person asked rex to change.
///
/// The write replaces the file in place rather than going through a temporary
/// file and a rename, so the install never swaps a path the person set up.
///
/// # Errors
/// [`RexError::Io`] if the file cannot be written.
pub(super) fn write_unless_linked(path: &Path, contents: &[u8]) -> Result<ProjectWrite, RexError> {
    if is_symlink(path) {
        // A warning that cannot be shown must not stop the install.
        let _ = writeln!(
            io::stderr().lock(),
            "rex init: skipped {}: it is a symbolic link",
            path.display()
        );
        return Ok(ProjectWrite::SkippedLink);
    }

    fs::write(path, contents).map_err(|source| RexError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(ProjectWrite::Written)
}
