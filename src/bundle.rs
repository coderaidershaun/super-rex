//! The skills and the agent grid that `rex init` installs, embedded in the
//! binary so that it needs no network or source checkout.

use std::path::{Path, PathBuf};

use include_dir::{Dir, include_dir};

// Two roots, not one of all of `.claude`, so that nothing else in that folder,
// such as a local settings file, can be embedded.
static EMBEDDED_SKILLS: Dir = include_dir!("$CARGO_MANIFEST_DIR/.claude/skills");
static EMBEDDED_AGENTS: Dir = include_dir!("$CARGO_MANIFEST_DIR/.claude/agents");

/// List every embedded file, paired with its contents, as a path relative to
/// `.claude`: `skills/<skill>/<file>` or `agents/<file>`.
#[must_use]
pub fn walk() -> Vec<(PathBuf, &'static [u8])> {
    let mut entries = Vec::new();
    walk_dir(&EMBEDDED_SKILLS, Path::new("skills"), &mut entries);
    walk_dir(&EMBEDDED_AGENTS, Path::new("agents"), &mut entries);
    entries
}

fn walk_dir(
    dir: &'static Dir<'static>,
    root_folder: &Path,
    entries: &mut Vec<(PathBuf, &'static [u8])>,
) {
    // File::path() is relative to the include_dir! root, not to `dir`, so it
    // is joined onto the root's folder name and never onto a per-level prefix.
    for file in dir.files() {
        entries.push((root_folder.join(file.path()), file.contents()));
    }
    for subdir in dir.dirs() {
        walk_dir(subdir, root_folder, entries);
    }
}
