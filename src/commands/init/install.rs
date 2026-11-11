//! Writes the embedded skills and agents into a folder: the files that are missing,
//! and the ones that differ when the caller asks for it.

use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

use super::print_line_to_stderr;
use crate::bundle;
use crate::commands::links::is_symlink;
use crate::error::RexError;

/// Skills that an earlier SuperRex installed and this one no longer ships.
/// Their text now lives inside `orchestration-builder`.
const RETIRED_SKILLS: [&str; 4] = [
    "orchestration-nuance-selection",
    "orchestration-work-breakdown",
    "orchestration-selection",
    "orchestration-decision-challenge",
];

/// The folders of retired skills still under `claude_dir`. rex names them and
/// never deletes them: a person may have edited one.
pub(super) fn retired_skill_folders(claude_dir: &Path) -> Vec<PathBuf> {
    RETIRED_SKILLS
        .iter()
        .map(|name| claude_dir.join("skills").join(name))
        .filter(|folder| folder.is_dir())
        .collect()
}

/// How many bundle files an install wrote, overwrote and left alone.
#[derive(Clone, Copy, Default)]
pub(super) struct InstallCounts {
    written: u32,
    overwritten: u32,
    /// Files that were already right or were left alone, including `differing`.
    skipped: u32,
    /// Files left alone although they differ from the bundle.
    differing: u32,
}

/// Write each bundle file that is not on disk yet under `target_root`. A file
/// that is there but differs from the bundle is overwritten only when
/// `is_forced`.
///
/// A file is never written through a symbolic link, with or without `is_forced`,
/// because the write would land on whatever the link points at. That covers the
/// file itself and every folder between it and its root folder, `skills` or
/// `agents`. The root folders are followed, because people link a shared one on
/// purpose.
///
/// A file that is there but cannot be read is left alone and named on stderr.
/// Its contents are unknown, so rex can neither call it up to date nor replace
/// it, and one such file must not stop the install.
pub(super) fn install_bundle(
    target_root: &Path,
    is_forced: bool,
) -> Result<InstallCounts, RexError> {
    let mut counts = InstallCounts::default();

    for (bundle_path, contents) in bundle::walk() {
        let dest = target_root.join(&bundle_path);

        let existing = match fs::read(&dest) {
            Ok(existing) => Some(existing),
            Err(source) if source.kind() == ErrorKind::NotFound => None,
            Err(source) => {
                warn_unreadable(&dest, &source);
                counts.skipped += 1;
                continue;
            }
        };

        let is_present = existing.is_some();
        if existing.is_some_and(|bytes| bytes == contents) {
            counts.skipped += 1;
        } else if is_present && !is_forced {
            counts.skipped += 1;
            counts.differing += 1;
        } else if is_linked(&dest, &root_folder(target_root, &bundle_path)) {
            warn_linked(&dest);
            counts.skipped += 1;
        } else {
            write_file(&dest, contents)?;
            if is_present {
                counts.overwritten += 1;
            } else {
                counts.written += 1;
            }
        }
    }

    Ok(counts)
}

/// The folder under `target_root` that holds the bundle file at `bundle_path`:
/// `skills` or `agents`, the first part of every bundle path.
fn root_folder(target_root: &Path, bundle_path: &Path) -> PathBuf {
    let first_part = bundle_path.iter().next().unwrap_or_default();
    target_root.join(first_part)
}

/// True when `dest`, or a folder between it and `root_folder`, is a symbolic
/// link, whether or not the link points at something that exists.
fn is_linked(dest: &Path, root_folder: &Path) -> bool {
    // Warning: `dest` must lie below `root_folder`. If it did not, the walk
    // would go on up to `/` and refuse a file under any linked system folder,
    // such as `/var` on macOS.
    dest.ancestors()
        .take_while(|path| *path != root_folder)
        .any(is_symlink)
}

fn warn_unreadable(path: &Path, source: &io::Error) {
    print_line_to_stderr(&format!("rex init: skipped {}: {source}", path.display()));
}

fn warn_linked(path: &Path) {
    print_line_to_stderr(&format!(
        "rex init: skipped {}: it or a folder above it is a symbolic link",
        path.display()
    ));
}

fn write_file(dest: &Path, contents: &[u8]) -> Result<(), RexError> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|source| RexError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    fs::write(dest, contents).map_err(|source| RexError::Io {
        path: dest.to_path_buf(),
        source,
    })
}

/// The one line `rex init` prints for the skills and agents. The note about
/// files that differ is added only when some do.
pub(super) fn summary(counts: InstallCounts) -> String {
    let InstallCounts {
        written,
        overwritten,
        skipped,
        differing,
    } = counts;

    let base = if overwritten == 0 {
        format!("rex init: {written} written, {skipped} skipped")
    } else {
        format!("rex init: {written} written, {overwritten} overwritten, {skipped} skipped")
    };
    match differing {
        0 => base,
        1 => format!("{base} (1 differs from this version; use --force to overwrite)"),
        _ => format!("{base} ({differing} differ from this version; use --force to overwrite)"),
    }
}
