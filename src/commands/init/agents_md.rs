//! Puts the rex sections on top of `AGENTS.md`, so agents run `rex codebase`
//! and `rex smells` instead of walking the tree by hand.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use super::file_change::{FileChange, ProjectWrite, contains_marker, write_unless_linked};
use crate::error::RexError;

pub(super) const AGENTS_FILENAME: &str = "AGENTS.md";

/// The rex sections start with this line. Finding it in `AGENTS.md` is what
/// stops a second run from adding the sections again.
const CODEBASE_MARKER: &str = "<!-- rex:codebase -->";

const CODEBASE_SECTION_BODY: &str = "\
## Codebase map

Run one of these commands before you search the tree by hand. Each one prints a tree of the working directory and skips the files that git ignores.

- `rex codebase`: every file.
- `rex codebase --only code`: only source files (Rust, Go, C, C++, Python, JavaScript, TypeScript, shell).
- `rex codebase --only go`: only one language. `--rust-only` is the same as `--only rust`.
- `rex codebase --with-context`: adds the first sentence of each source file's opening comment to its line.
- `rex codebase --only code --with-context`: the source files and what each one is for. Start here.

Add `--for-human` to write the tree to `CODEBASE.md` instead of printing it, for example `rex codebase --for-human --only code --with-context`.

Every source file opens with a comment of one or two lines that says what the file is for. A file with no sentence after its name in the tree is missing that comment. When you create or change a file, write the comment or bring it up to date.
";

const SMELLS_SECTION_BODY: &str = "\
## Code smells

Run this command to find each known problem that was flagged and left in the code. It reads every `.rs` file and skips the files that git ignores.

- `rex smells`: one `path:line` for each `SMELL` flag comment in the Rust files, then the count of them all.
";

/// Make sure `cwd/AGENTS.md` has the rex sections: create the file if it is
/// missing, put the sections above the existing text if it has none yet, and
/// leave the file alone if it does.
///
/// A path that is a symbolic link is left alone as well; see
/// [`write_unless_linked`].
pub(super) fn ensure_rex_sections(cwd: &Path) -> Result<FileChange, RexError> {
    let path = cwd.join(AGENTS_FILENAME);

    // The file is read as bytes because a file that is not valid UTF-8 must
    // not fail the command after the bundle is already installed.
    let existing = match fs::read(&path) {
        Ok(bytes) => Some(bytes),
        Err(source) if source.kind() == ErrorKind::NotFound => None,
        Err(source) => return Err(RexError::Io { path, source }),
    };

    let mut contents =
        format!("{CODEBASE_MARKER}\n{CODEBASE_SECTION_BODY}\n{SMELLS_SECTION_BODY}").into_bytes();
    let change = match existing {
        None => FileChange::Created,
        Some(old) if contains_marker(&old, CODEBASE_MARKER) => return Ok(FileChange::Unchanged),
        Some(old) => {
            contents.push(b'\n');
            contents.extend_from_slice(&old);
            FileChange::Updated
        }
    };

    match write_unless_linked(&path, &contents)? {
        ProjectWrite::Written => Ok(change),
        ProjectWrite::SkippedLink => Ok(FileChange::Unchanged),
    }
}
