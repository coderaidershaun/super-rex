//! `rex skills list`: finds the skills of this repo, in the repo's skills folder,
//! its skill library and the user's skills folder, and prints their headers.

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::header::Header;
use super::{REPO_LIBRARY_FOLDER as LIBRARY_ROOT, REPO_SKILLS_FOLDER as SKILLS_ROOT};
use super::{SKILL_FILE, SkillsError};
use crate::commands::stdout::{print_line_to_stdout, print_to_stdout};
use crate::store::StorePaths;

/// Where a skill was found. Claude Code runs the user's skill when the repo's
/// skills folder and the user's both hold one with the same name. Claude Code
/// does not load the library; an agent reads a library skill by its path.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum Source {
    Repo,
    Library,
    User,
}

impl Source {
    fn name(self) -> &'static str {
        match self {
            Source::Repo => "repo",
            Source::Library => "library",
            Source::User => "user",
        }
    }
}

/// One folder that holds skills.
pub(super) struct Place {
    pub(super) source: Source,
    pub(super) folder: PathBuf,
}

/// The repo's skills folder and skill library, for the repo that holds
/// `cwd`, then the user's skills folder. When the repo's skills folder is the
/// user's, only the user's is given. A linked worktree with a library of its
/// own gives it first, then its main checkout's library.
///
/// # Errors
/// [`SkillsError::Read`] if `cwd` or a `.git` entry above it cannot be read.
pub(super) fn places(cwd: &Path) -> Result<Vec<Place>, SkillsError> {
    let repo = Place {
        source: Source::Repo,
        folder: repo_root(cwd, SKILLS_ROOT)?.join(SKILLS_ROOT),
    };
    let library = library_places(cwd)?;
    let home = std::env::var_os("HOME").filter(|home| !home.is_empty());
    let user = home.map(|home| Place {
        source: Source::User,
        folder: PathBuf::from(home).join(SKILLS_ROOT),
    });
    let same_folder = user.as_ref().is_some_and(|user| {
        match (repo.folder.canonicalize(), user.folder.canonicalize()) {
            (Ok(repo), Ok(user)) => repo == user,
            _ => false,
        }
    });
    let mut places = Vec::new();
    if !same_folder {
        places.push(repo);
    }
    places.extend(library);
    places.extend(user);
    Ok(places)
}

/// The library of [`repo_root`], and after it the main checkout's library
/// when that root is a linked worktree. A worktree holds only the library
/// skills that Git tracks there, so a skill kept with `git add -f`, or a new
/// one, would otherwise hide every other library skill.
fn library_places(cwd: &Path) -> Result<Vec<Place>, SkillsError> {
    let root = repo_root(cwd, LIBRARY_ROOT)?;
    let mut places = vec![Place {
        source: Source::Library,
        folder: root.join(LIBRARY_ROOT),
    }];
    if let Ok(paths) = StorePaths::locate(&root)
        && paths.root != root
    {
        places.push(Place {
            source: Source::Library,
            folder: paths.root.join(LIBRARY_ROOT),
        });
    }
    Ok(places)
}

/// The nearest folder at or above `cwd` that holds `.git`, else `cwd`. A
/// linked worktree with no `skills_folder` of its own gives its main
/// checkout, the way Claude Code finds skills there.
fn repo_root(cwd: &Path, skills_folder: &str) -> Result<PathBuf, SkillsError> {
    let cwd = cwd.canonicalize().map_err(SkillsError::read(cwd))?;
    for folder in cwd.ancestors() {
        let dot_git = folder.join(".git");
        let metadata = match fs::metadata(&dot_git) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(source) => {
                return Err(SkillsError::Read {
                    path: dot_git,
                    source,
                });
            }
        };
        if metadata.is_dir() || folder.join(skills_folder).exists() {
            return Ok(folder.to_path_buf());
        }
        // A worktree whose main checkout cannot be found still lists its own
        // folder, so that one odd `.git` file never stops the list.
        let main_checkout = StorePaths::locate(folder).map(|paths| paths.root);
        return Ok(main_checkout.unwrap_or_else(|_| folder.to_path_buf()));
    }
    Ok(cwd)
}

/// Each skill folder in `place`: an entry that is a folder, or a link to
/// one, and holds a `SKILL.md`. A place that does not exist holds none.
///
/// # Errors
/// [`SkillsError::Read`] naming `place` if it exists but cannot be read.
pub(super) fn skill_folders(place: &Path) -> Result<Vec<PathBuf>, SkillsError> {
    let entries = match fs::read_dir(place) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => {
            return Err(SkillsError::Read {
                path: place.to_path_buf(),
                source,
            });
        }
    };
    let mut folders = Vec::new();
    for entry in entries {
        let folder = entry.map_err(SkillsError::read(place))?.path();
        if folder.is_dir() && folder.join(SKILL_FILE).is_file() {
            folders.push(folder);
        }
    }
    Ok(folders)
}

/// One printed skill. The fields are in output order, which the JSON form
/// keeps.
#[derive(Serialize)]
struct Listed {
    name: String,
    description: String,
    when_to_use: String,
    source: Source,
    shadowed: bool,
    path: String,
}

impl Listed {
    fn read(folder: &Path, source: Source) -> Listed {
        let path = folder.join(SKILL_FILE);
        // One skill that cannot be read is listed with empty fields, so that
        // it never stops the list; `rex skills check` names the problem.
        let header = Header::of_skill_path(&path);
        let field = |key: &str| header.as_ref().and_then(|header| header.text(key));
        let folder_name = folder.file_name().unwrap_or_default().to_string_lossy();
        Listed {
            name: field("name")
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| folder_name.into_owned()),
            description: field("description").unwrap_or_default(),
            when_to_use: field("when_to_use").unwrap_or_default(),
            source,
            shadowed: false,
            path: path.display().to_string(),
        }
    }

    fn text(&self) -> String {
        let shadowed = if self.shadowed { "yes" } else { "no" };
        format!(
            "name={}\ndescription={}\nwhen_to_use={}\nsource={}\nshadowed={shadowed}\npath={}\n",
            self.name,
            self.description,
            self.when_to_use,
            self.source.name(),
            self.path
        )
    }
}

/// Print every skill of [`places`]: repo skills by name, then library skills
/// by name, then user skills by name, as `key=value` blocks or, with `json`,
/// as one JSON line. Only a repo skill can be shadowed. A library skill of a
/// worktree hides the main checkout's library skill of the same name.
///
/// # Errors
/// [`SkillsError::Read`] if a place exists but cannot be read,
/// [`SkillsError::Json`] if the list cannot be written as JSON, and
/// [`SkillsError::Rex`] if standard output cannot be written.
pub(super) fn run(cwd: &Path, json: bool) -> Result<(), SkillsError> {
    let mut skills: Vec<Listed> = Vec::new();
    for place in places(cwd)? {
        let mut found: Vec<Listed> = skill_folders(&place.folder)?
            .iter()
            .map(|folder| Listed::read(folder, place.source))
            .collect();
        if place.source == Source::Library {
            found.retain(|skill| {
                !skills
                    .iter()
                    .any(|listed| listed.source == Source::Library && listed.name == skill.name)
            });
        }
        skills.extend(found);
    }
    // Two folders can hold the same name, so the path settles the order.
    skills.sort_by(|left, right| {
        (left.source, left.name.as_bytes(), &left.path).cmp(&(
            right.source,
            right.name.as_bytes(),
            &right.path,
        ))
    });

    let user_names: BTreeSet<String> = skills
        .iter()
        .filter(|skill| skill.source == Source::User)
        .map(|skill| skill.name.clone())
        .collect();
    for skill in &mut skills {
        skill.shadowed = skill.source == Source::Repo && user_names.contains(&skill.name);
    }

    if json {
        let line = serde_json::to_string(&skills).map_err(|source| SkillsError::Json { source })?;
        print_line_to_stdout(&line)?;
    } else if !skills.is_empty() {
        let blocks: Vec<String> = skills.iter().map(Listed::text).collect();
        print_to_stdout(&blocks.join("\n"))?;
    }
    Ok(())
}
