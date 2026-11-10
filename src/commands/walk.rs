//! Walks the working directory the same way for every command that reads it:
//! dotfiles and gitignored paths are skipped, and entries come sorted by name.

use std::io::{self, ErrorKind, Write};
use std::path::Path;

use ignore::{DirEntry, WalkBuilder};

use crate::error::RexError;

/// Visits every entry under `cwd` that is not hidden or gitignored. A
/// directory comes before what is inside it, and `cwd` itself comes first, at
/// depth 0.
///
/// A path that cannot be read because of its permissions is skipped, with a
/// warning on stderr. The rest of the tree is still visited.
///
/// # Errors
/// An item is [`RexError::Walk`] if that step of the walk fails for another
/// reason (loop, glob). It names the path that failed when the walk reports
/// one, and `cwd` otherwise.
pub(super) fn visible_entries(
    cwd: &Path,
) -> impl Iterator<Item = Result<DirEntry, RexError>> + use<> {
    let root = cwd.to_path_buf();

    // require_git(false) so the command works in non-git dirs (tests, fresh
    // checkouts). Without it, `ignore` only honors .gitignore inside a git repo.
    WalkBuilder::new(cwd)
        .hidden(true)
        .git_ignore(true)
        .git_exclude(true)
        .git_global(true)
        .require_git(false)
        .sort_by_file_name(|a, b| a.cmp(b))
        .build()
        .filter_map(move |entry| match entry {
            Ok(entry) => Some(Ok(entry)),
            Err(source) if is_permission_denied(&source) => {
                warn_skipped(failing_path(&source).unwrap_or(&root));
                None
            }
            Err(source) => Some(Err(RexError::Walk {
                path: failing_path(&source).map_or_else(|| root.clone(), Path::to_path_buf),
                source,
            })),
        })
}

fn is_permission_denied(error: &ignore::Error) -> bool {
    error
        .io_error()
        .is_some_and(|io_error| io_error.kind() == ErrorKind::PermissionDenied)
}

fn warn_skipped(path: &Path) {
    // A warning that cannot be shown must not stop the walk.
    let _ = writeln!(
        io::stderr().lock(),
        "rex: skipped {}: permission denied",
        path.display()
    );
}

/// Turns a permission failure on `path` into `None`, with a warning on stderr,
/// so one unreadable file does not stop a command. Any other failure is an
/// error that names `path`.
pub(super) fn skip_if_denied<T>(path: &Path, result: io::Result<T>) -> Result<Option<T>, RexError> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(source) if source.kind() == ErrorKind::PermissionDenied => {
            warn_skipped(path);
            Ok(None)
        }
        Err(source) => Err(RexError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

/// The path a walk error is about, when the error holds one.
fn failing_path(error: &ignore::Error) -> Option<&Path> {
    match error {
        ignore::Error::WithPath { path, .. } => Some(path),
        ignore::Error::Loop { child, .. } => Some(child),
        ignore::Error::WithDepth { err, .. } | ignore::Error::WithLineNumber { err, .. } => {
            failing_path(err)
        }
        ignore::Error::Partial(errors) => errors.iter().find_map(failing_path),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::path::PathBuf;

    use super::*;

    fn permission_denied() -> ignore::Error {
        ignore::Error::Io(io::Error::from(io::ErrorKind::PermissionDenied))
    }

    #[test]
    fn failing_path_finds_the_path_through_depth_wrappers() {
        let error = ignore::Error::WithDepth {
            depth: 1,
            err: Box::new(ignore::Error::WithPath {
                path: PathBuf::from("./secret"),
                err: Box::new(permission_denied()),
            }),
        };

        assert_eq!(failing_path(&error), Some(Path::new("./secret")));
    }

    #[test]
    fn failing_path_names_the_child_of_a_symlink_loop() {
        let error = ignore::Error::Loop {
            ancestor: PathBuf::from("./a"),
            child: PathBuf::from("./a/b/back"),
        };

        assert_eq!(failing_path(&error), Some(Path::new("./a/b/back")));
    }

    #[test]
    fn failing_path_is_none_when_the_error_carries_no_path() {
        assert_eq!(failing_path(&permission_denied()), None);
    }
}
