//! The skills, the agent grid and the skill library that `rex init` installs,
//! embedded in the binary so that it needs no network or source checkout.

use std::path::{Path, PathBuf};

use include_dir::{Dir, include_dir};

// Three roots, not all of `.claude` and `.rex`, so that nothing else in those
// folders, such as a local settings file or the store, can be embedded.
static EMBEDDED_SKILLS: Dir = include_dir!("$CARGO_MANIFEST_DIR/.claude/skills");
static EMBEDDED_AGENTS: Dir = include_dir!("$CARGO_MANIFEST_DIR/.claude/agents");
static EMBEDDED_LIBRARY: Dir = include_dir!("$CARGO_MANIFEST_DIR/.rex/skills");

/// The folder, relative to the repo root, of each embedded root: the skills
/// Claude Code lists, the agent grid, and the library of skills that agents
/// read by path.
pub const SKILLS_ROOT: &str = ".claude/skills";
pub const AGENTS_ROOT: &str = ".claude/agents";
pub const LIBRARY_ROOT: &str = ".rex/skills";
pub const ROOTS: [&str; 3] = [SKILLS_ROOT, AGENTS_ROOT, LIBRARY_ROOT];

/// The one of [`ROOTS`] that holds `path`, a path as [`walk`] gives it.
#[must_use]
pub fn root_of(path: &Path) -> Option<&'static str> {
    ROOTS.into_iter().find(|root| path.starts_with(root))
}

/// List every embedded file, paired with its contents, as a path relative to
/// the repo root: `.claude/skills/<skill>/<file>`, `.claude/agents/<file>` or
/// `.rex/skills/<skill>/<file>`. A file or folder whose name starts with `.`
/// is left out, at any depth below its root.
#[must_use]
pub fn walk() -> Vec<(PathBuf, &'static [u8])> {
    let mut entries = Vec::new();
    walk_dir(&EMBEDDED_SKILLS, Path::new(SKILLS_ROOT), &mut entries);
    walk_dir(&EMBEDDED_AGENTS, Path::new(AGENTS_ROOT), &mut entries);
    walk_dir(&EMBEDDED_LIBRARY, Path::new(LIBRARY_ROOT), &mut entries);
    entries
}

fn walk_dir(
    dir: &'static Dir<'static>,
    root_folder: &Path,
    entries: &mut Vec<(PathBuf, &'static [u8])>,
) {
    // File::path() is relative to the include_dir! root, not to `dir`, so it
    // is joined onto the root's folder name and never onto a per-level prefix.
    for file in dir.files().filter(|file| !is_hidden(file.path())) {
        entries.push((root_folder.join(file.path()), file.contents()));
    }
    for subdir in dir.dirs().filter(|subdir| !is_hidden(subdir.path())) {
        walk_dir(subdir, root_folder, entries);
    }
}

/// True for a name that starts with `.`, such as the `.DS_Store` file that
/// macOS leaves in a folder. Such a file is never part of a skill or an agent.
fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .is_some_and(|name| name.as_encoded_bytes().starts_with(b"."))
}

#[cfg(test)]
mod tests {
    use include_dir::{DirEntry, File};

    use super::*;

    static NESTED_HIDDEN: [DirEntry<'static>; 2] = [
        DirEntry::File(File::new("skill/.DS_Store", b"x")),
        DirEntry::File(File::new("skill/SKILL.md", b"x")),
    ];
    static HIDDEN_FOLDER: [DirEntry<'static>; 1] =
        [DirEntry::File(File::new(".hidden/SKILL.md", b"x"))];
    static ROOT_ENTRIES: [DirEntry<'static>; 4] = [
        DirEntry::File(File::new(".DS_Store", b"x")),
        DirEntry::File(File::new("README.md", b"x")),
        DirEntry::Dir(Dir::new("skill", &NESTED_HIDDEN)),
        DirEntry::Dir(Dir::new(".hidden", &HIDDEN_FOLDER)),
    ];
    static ROOT: Dir<'static> = Dir::new("", &ROOT_ENTRIES);

    // The real bundle holds no hidden file, so this is the only test that
    // fails if the walk stops skipping them.
    #[test]
    fn walk_skips_hidden_files_and_folders_at_any_depth() {
        let mut entries = Vec::new();
        walk_dir(&ROOT, Path::new(LIBRARY_ROOT), &mut entries);
        let paths: Vec<PathBuf> = entries.into_iter().map(|(path, _)| path).collect();

        assert_eq!(
            paths,
            [
                PathBuf::from(".rex/skills/README.md"),
                PathBuf::from(".rex/skills/skill/SKILL.md")
            ]
        );
    }
}
