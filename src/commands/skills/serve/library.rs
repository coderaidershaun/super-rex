//! The folders (or the embedded copy) that the server reads skills from, with shadowing, hashing and safe file reads.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use super::{ServeError, folder_name, vocabulary};
use crate::bundle;

/// The name `head` reports for the library embedded in the binary.
const EMBEDDED_NAME: &str = "super-rex";

pub(super) enum Library {
    Folder(PathBuf),
    Embedded,
}

/// Why `get` could not give a file.
pub(super) enum ReadError {
    NotFound,
    Unreadable(String),
}

/// The libraries in priority order. A skill name in an earlier one hides the
/// whole skill of that name in every later one.
pub(super) struct Libraries {
    list: Vec<Library>,
    name: String,
    vocabulary_file: Option<PathBuf>,
}

pub(super) fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// A skill folder name: lower-case letters, digits and hyphens.
pub(super) fn is_skill_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

impl Libraries {
    pub(super) fn open(folders: &[PathBuf]) -> Result<Libraries, ServeError> {
        if folders.is_empty() {
            return Ok(Libraries {
                list: vec![Library::Embedded],
                name: EMBEDDED_NAME.to_string(),
                vocabulary_file: None,
            });
        }
        let mut list = Vec::new();
        for folder in folders {
            let canonical = folder.canonicalize().ok().filter(|path| path.is_dir());
            let Some(canonical) = canonical else {
                return Err(ServeError::NoLibrary {
                    path: folder.clone(),
                });
            };
            list.push(Library::Folder(canonical));
        }
        let Some(Library::Folder(first)) = list.first() else {
            unreachable!("a non-empty folder list gives folder libraries");
        };
        Ok(Libraries {
            name: folder_name(first),
            vocabulary_file: first.parent().map(|beside| beside.join("vocab/terms.json")),
            list,
        })
    }

    pub(super) fn name(&self) -> &str {
        &self.name
    }

    /// The vocabulary version, or `None` for the embedded library and for a
    /// folder with no `vocab/terms.json` beside it.
    pub(super) fn vocabulary_version(&self) -> Result<Option<String>, ServeError> {
        match &self.vocabulary_file {
            Some(path) => vocabulary::version_of(path),
            None => Ok(None),
        }
    }

    /// Every served file as `(<skill>/<file>, file sha256)`, sorted by path:
    /// the files of the first library that has each skill.
    pub(super) fn served_files(&self) -> io::Result<Vec<(String, String)>> {
        let mut owners: BTreeMap<String, &Library> = BTreeMap::new();
        for library in &self.list {
            for skill in library.skill_names()? {
                owners.entry(skill).or_insert(library);
            }
        }
        let mut files = Vec::new();
        for (skill, library) in owners {
            for (file, sha) in library.files(&skill)? {
                files.push((format!("{skill}/{file}"), sha));
            }
        }
        files.sort();
        Ok(files)
    }

    /// The text of `file` in `skill`, from the first library that has the skill.
    pub(super) fn read(&self, skill: &str, file: &str) -> Result<Vec<u8>, ReadError> {
        if !is_skill_name(skill) || !is_plain_relative(file) {
            return Err(ReadError::NotFound);
        }
        for library in &self.list {
            if library.has_skill(skill) {
                return library.read(skill, file);
            }
        }
        Err(ReadError::NotFound)
    }
}

/// A relative file path with no `.`, `..` or hidden component, so `..` never
/// reaches the file system and `.DS_Store` is never given out.
fn is_plain_relative(file: &str) -> bool {
    !file.is_empty()
        && !file.starts_with(['/', '\\'])
        && !Path::new(file).is_absolute()
        && file
            .split(['/', '\\'])
            .all(|part| !part.is_empty() && !part.starts_with('.'))
}

impl Library {
    fn skill_names(&self) -> io::Result<Vec<String>> {
        match self {
            Library::Embedded => {
                let mut names: Vec<String> = embedded_files()
                    .filter(|(_, file, _)| file == "SKILL.md")
                    .map(|(skill, _, _)| skill)
                    .collect();
                names.sort();
                Ok(names)
            }
            Library::Folder(root) => {
                let mut names = Vec::new();
                for entry in fs::read_dir(root)? {
                    let entry = entry?;
                    let name = entry.file_name().to_string_lossy().into_owned();
                    if entry.file_type()?.is_dir()
                        && is_skill_name(&name)
                        && entry.path().join("SKILL.md").is_file()
                    {
                        names.push(name);
                    }
                }
                names.sort();
                Ok(names)
            }
        }
    }

    fn has_skill(&self, skill: &str) -> bool {
        match self {
            Library::Embedded => embedded_files().any(|(s, f, _)| s == skill && f == "SKILL.md"),
            Library::Folder(root) => {
                let folder = root.join(skill);
                folder.is_dir() && !folder.is_symlink() && folder.join("SKILL.md").is_file()
            }
        }
    }

    /// The files of one skill as `(path inside the skill, sha256)`. Hidden names
    /// and symbolic links are left out, at any depth.
    fn files(&self, skill: &str) -> io::Result<Vec<(String, String)>> {
        match self {
            Library::Embedded => Ok(embedded_files()
                .filter(|(s, _, _)| s == skill)
                .map(|(_, file, bytes)| (file, sha256_hex(bytes)))
                .collect()),
            Library::Folder(root) => {
                let mut files = Vec::new();
                folder_files(&root.join(skill), "", &mut files)?;
                Ok(files)
            }
        }
    }

    fn read(&self, skill: &str, file: &str) -> Result<Vec<u8>, ReadError> {
        match self {
            Library::Embedded => embedded_files()
                .find(|(s, f, _)| s == skill && f == file)
                .map(|(_, _, bytes)| bytes.to_vec())
                .ok_or(ReadError::NotFound),
            Library::Folder(root) => {
                let skill_folder = root
                    .join(skill)
                    .canonicalize()
                    .map_err(|_| ReadError::NotFound)?;
                let path = skill_folder
                    .join(file)
                    .canonicalize()
                    .map_err(|_| ReadError::NotFound)?;
                if !path.starts_with(&skill_folder) || !path.is_file() {
                    return Err(ReadError::NotFound);
                }
                fs::read(&path).map_err(|error| {
                    ReadError::Unreadable(format!("cannot read {skill}/{file}: {error}"))
                })
            }
        }
    }
}

fn folder_files(folder: &Path, prefix: &str, files: &mut Vec<(String, String)>) -> io::Result<()> {
    for entry in fs::read_dir(folder)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let kind = entry.file_type()?;
        let relative = format!("{prefix}{name}");
        if kind.is_dir() {
            folder_files(&entry.path(), &format!("{relative}/"), files)?;
        } else if kind.is_file() {
            files.push((relative, sha256_hex(&fs::read(entry.path())?)));
        }
    }
    Ok(())
}

/// The embedded library as `(skill, path inside the skill, bytes)`, for each
/// skill that holds a `SKILL.md`.
fn embedded_files() -> impl Iterator<Item = (String, String, &'static [u8])> {
    let root = Path::new(bundle::LIBRARY_ROOT);
    let all: Vec<(String, String, &'static [u8])> = bundle::walk()
        .into_iter()
        .filter_map(|(path, bytes)| {
            let inside = path.strip_prefix(root).ok()?;
            let (skill, file) = inside.to_str()?.split_once('/')?;
            Some((skill.to_string(), file.to_string(), bytes))
        })
        .collect();
    let skills: Vec<String> = all
        .iter()
        .filter(|(_, file, _)| file == "SKILL.md")
        .map(|(skill, _, _)| skill.clone())
        .collect();
    all.into_iter()
        .filter(move |(skill, _, _)| skills.contains(skill))
}
