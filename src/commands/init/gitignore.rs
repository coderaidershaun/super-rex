//! Adds a block to `.gitignore`, or puts it in place of an older release's, so the
//! store, the local settings and the worktrees are never committed, while the repo's own skills and config are.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use super::file_change::{FileChange, ProjectWrite, contains_marker, write_unless_linked};
use super::print_line_to_stderr;
use crate::commands::stdout::print_line_to_stdout;
use crate::error::RexError;

pub(super) const GITIGNORE_FILENAME: &str = ".gitignore";

/// The first line of the block. Finding it in `.gitignore` is what stops a
/// second run from adding the block again.
const IGNORE_MARKER: &str = "# rex:ignore superrex";

/// The whole block, marker line included, ending in a newline.
///
/// Claude Code's own worktree isolation puts worktrees in `.claude/worktrees/`.
/// A job's worktree now goes in `<repo>.worktrees/`, outside the repo, but
/// `/.rex/worktrees/` stays: a repo set up by rex 0.7 can still hold a job
/// worktree there until it is removed. Untracked, any of them makes the
/// checkout not clean, and a job then refuses to work in it. The rest of
/// `.rex/` is tracked: the skills a job or feedback writes in `.rex/skills/`,
/// and `.rex/config.json`, which a fresh clone needs to turn the plugin on.
const IGNORE_BLOCK: &str = "# rex:ignore superrex\n.claude/settings.local.json\n/.claude/worktrees/\n/.rex/rex.db*\n/.rex/events.jsonl\n/.rex/orchestrators/\n/.rex/plans/\n/.rex/worktrees/\n/.rex/live/\n";

/// The blocks older releases wrote, byte for byte: 0.1.0 to 0.3.0, then 0.4.0
/// to 0.7.0. Both ignore `.claude/skills/`, `.claude/agents/` and all of
/// `.rex/`, so `.rex/config.json` and the repo's own `.rex/skills/` could not
/// be committed, and the commit line `rex init` prints could not be followed.
const OLD_BLOCKS: [&str; 2] = [
    "# rex:ignore superrex\n.claude/skills/\n.claude/agents/\n.claude/settings.local.json\n/.rex/\n/.claude/worktrees/\n",
    "# rex:ignore superrex\n.claude/skills/\n.claude/agents/\n.claude/settings.local.json\n/.rex/*\n/.claude/worktrees/\n",
];

/// The lines of the old blocks that this version's block leaves out.
const OLD_ONLY_LINES: [&str; 4] = [".claude/skills/", ".claude/agents/", "/.rex/", "/.rex/*"];

/// Make sure `cwd/.gitignore` has the rex block: create the file if it is
/// missing, append the block after the existing lines if it is not there yet,
/// put this version's block in place of an old release's, and leave the file
/// alone otherwise. A block the person changed is kept, with a warning that
/// names each line to change, and never fails the run.
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
        Some(old) if contains_marker(&old, IGNORE_MARKER) => match replace_old_block(&old) {
            Some(replaced) => {
                print_line_to_stdout(
                    "rex init: replaced the rex block in .gitignore with this version's: the old one also ignored .rex/config.json, .rex/skills/, .claude/skills/ and .claude/agents/",
                )?;
                (replaced, FileChange::Updated)
            }
            None => {
                warn_changed_block(&old);
                return Ok(FileChange::Unchanged);
            }
        },
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

/// The byte offset where the line that starts with the marker begins.
fn block_start(contents: &[u8]) -> Option<usize> {
    let marker = IGNORE_MARKER.as_bytes();
    (0..contents.len())
        .find(|&at| (at == 0 || contents[at - 1] == b'\n') && contents[at..].starts_with(marker))
}

/// `contents` with an old release's block, as that release wrote it, swapped
/// for this version's; every line around it stays. `None` when the block is
/// this version's or the person changed it.
fn replace_old_block(contents: &[u8]) -> Option<Vec<u8>> {
    let start = block_start(contents)?;
    let rest = &contents[start..];
    let old = OLD_BLOCKS
        .iter()
        .find(|old| rest.starts_with(old.as_bytes()))?;
    let mut replaced = contents[..start].to_vec();
    replaced.extend_from_slice(IGNORE_BLOCK.as_bytes());
    replaced.extend_from_slice(&rest[old.len()..]);
    Some(replaced)
}

/// Name each line to delete from, and add to, a rex block the person
/// changed, read up to its first blank line. Nothing is said when the block
/// already ignores what this version's does and no more of rex's own.
fn warn_changed_block(contents: &[u8]) {
    let Some(start) = block_start(contents) else {
        return;
    };
    let text = String::from_utf8_lossy(&contents[start..]);
    let theirs: Vec<&str> = text
        .lines()
        .skip(1)
        .take_while(|line| !line.trim().is_empty())
        .map(str::trim_end)
        .collect();
    let deletes: Vec<&str> = theirs
        .iter()
        .copied()
        .filter(|line| OLD_ONLY_LINES.contains(line))
        .collect();
    let adds: Vec<&str> = IGNORE_BLOCK
        .lines()
        .skip(1)
        .filter(|line| !theirs.contains(line))
        .collect();
    if deletes.is_empty() && adds.is_empty() {
        return;
    }
    print_line_to_stderr(
        "rex init: warning: kept the rex block in .gitignore, which differs from the block any rex wrote; to finish the upgrade, change it by hand:",
    );
    for line in deletes {
        print_line_to_stderr(&format!("rex init:   delete the line {line}"));
    }
    for line in adds {
        print_line_to_stderr(&format!("rex init:   add the line {line}"));
    }
}
