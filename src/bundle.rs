//! The skills and the agent grid that `rex init` installs, embedded in the
//! binary so that it needs no network or source checkout.

use std::path::{Path, PathBuf};

use include_dir::{Dir, include_dir};

// Two roots, not one of all of `.claude`, so that nothing else in that folder,
// such as a local settings file, can be embedded.
static EMBEDDED_SKILLS: Dir = include_dir!("$CARGO_MANIFEST_DIR/.claude/skills");
static EMBEDDED_AGENTS: Dir = include_dir!("$CARGO_MANIFEST_DIR/.claude/agents");

/// List every embedded file, paired with its contents, as a path relative to
/// `.claude`: `skills/<skill>/<file>` or `agents/<file>`. A file or folder
/// whose name starts with `.` is left out, at any depth.
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
        walk_dir(&ROOT, Path::new("skills"), &mut entries);
        let paths: Vec<PathBuf> = entries.into_iter().map(|(path, _)| path).collect();

        assert_eq!(
            paths,
            [
                PathBuf::from("skills/README.md"),
                PathBuf::from("skills/skill/SKILL.md")
            ]
        );
    }
}
