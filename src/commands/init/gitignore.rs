//! Adds a block to `.gitignore`, so what `rex init` installs, the store and
//! Claude Code's worktrees are never committed.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use super::file_change::{FileChange, ProjectWrite, contains_marker, write_unless_linked};
use crate::error::RexError;

pub(super) const GITIGNORE_FILENAME: &str = ".gitignore";

/// The first line of the block. Finding it in `.gitignore` is what stops a
/// second run from adding the block again. The older `# rex:ignore claude`
/// block of rex-skills does not count, so a repo that has it still gets the
/// new entries.
const IGNORE_MARKER: &str = "# rex:ignore superrex";

/// The whole block, marker line included, ending in a newline.
///
/// Claude Code's own worktree isolation puts worktrees in `.claude/worktrees/`.
/// Untracked, they make the checkout not clean, and a job then refuses to
/// work in it.
const IGNORE_BLOCK: &str = "# rex:ignore superrex\n.claude/skills/\n.claude/agents/\n.claude/settings.local.json\n/.rex/\n/.claude/worktrees/\n";

/// Make sure `cwd/.gitignore` has the rex block: create the file if it is
/// missing, append the block after the existing lines if it is not there yet,
/// and leave the file alone if it is.
///
/// A path that is a symbolic link is left alone as well; see
/// [`write_unless_linked`].
pub(super) fn ensure_ignore_block(cwd: &Path) -> Result<FileChange, RexError> {
    let path = cwd.join(GITIGNORE_FILENAME);

    let existing = match fs::read(&path) {
        Ok(bytes) => Some(bytes),
        Err(source) if source.kind() == ErrorKind::NotFound => None,
        Err(source) => return Err(RexError::Io { path, source }),
    };

    let (contents, change) = match existing {
        None => (IGNORE_BLOCK.as_bytes().to_vec(), FileChange::Created),
        Some(old) if contains_marker(&old, IGNORE_MARKER) => {
            return Ok(FileChange::Unchanged);
        }
        Some(mut old) => {
            if !old.ends_with(b"\n") {
                old.push(b'\n');
            }
            old.push(b'\n');
            old.extend_from_slice(IGNORE_BLOCK.as_bytes());
            (old, FileChange::Updated)
        }
    };

    match write_unless_linked(&path, &contents)? {
        ProjectWrite::Written => Ok(change),
        ProjectWrite::SkippedLink => Ok(FileChange::Unchanged),
    }
}
