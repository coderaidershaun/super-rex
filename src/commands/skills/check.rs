//! `rex skills check`: reads each skill from disk, runs the rules on it, and
//! prints `ok` or the findings. Every file is read before anything is
//! printed, so a bad path stops the command with no partial output.

use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::list::{self, Source};
use super::rules::{KnownSkills, Skill, SkillFile, check_skill};
use super::{SKILL_FILE, SkillsError};
use crate::bundle::{self, LIBRARY_ROOT, SKILLS_ROOT};
use crate::commands::stdout::print_line_to_stdout;

/// One skill read from disk, with the paths to print it by.
struct LoadedSkill {
    /// The path as the user wrote it.
    written: PathBuf,
    /// The skill folder as the user wrote it. A finding's path is this joined
    /// with the file's path inside the skill.
    written_folder: PathBuf,
    skill: Skill,
    /// The skills and Markdown files of the folder that holds the skill.
    neighbours: KnownSkills,
}

impl LoadedSkill {
    fn load(cwd: &Path, written: &Path) -> Result<LoadedSkill, SkillsError> {
        let (skill_file, written_folder) = skill_file_of(cwd, written)?;
        let folder = absolute_skill_folder(&skill_file).map_err(SkillsError::read(written))?;
        let mut files = Vec::new();
        for file in markdown_files(&folder)? {
            let text = read_text(&folder.join(&file))?;
            files.push(SkillFile { path: file, text });
        }
        let skills_folder = folder.parent().unwrap_or(&folder);
        let mut neighbours = KnownSkills {
            files: markdown_files(skills_folder)?.into_iter().collect(),
            ..KnownSkills::default()
        };
        // Any skills folder that is not a skill library counts as a
        // `.claude/skills/`, so a skill checked outside a repo still knows
        // its neighbours.
        let neighbour_paths = skill_paths(skills_folder)?;
        if skills_folder.ends_with(LIBRARY_ROOT) {
            neighbours.library_paths = neighbour_paths;
        } else {
            neighbours.skill_paths = neighbour_paths;
        }

        Ok(LoadedSkill {
            written: written.to_path_buf(),
            written_folder,
            skill: Skill {
                folder: file_name(&folder),
                files,
            },
            neighbours,
        })
    }
}

/// The `SKILL.md` that `written` names, and the skill folder as the user
/// wrote it.
fn skill_file_of(cwd: &Path, written: &Path) -> Result<(PathBuf, PathBuf), SkillsError> {
    let path = cwd.join(written);
    let metadata = match fs::metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Err(SkillsError::Missing {
                path: written.to_path_buf(),
            });
        }
        Err(source) => {
            return Err(SkillsError::Read {
                path: written.to_path_buf(),
                source,
            });
        }
    };

    if metadata.is_dir() {
        let skill_file = path.join(SKILL_FILE);
        if !skill_file.is_file() {
            return Err(SkillsError::NoSkillFile {
                path: written.to_path_buf(),
            });
        }
        return Ok((skill_file, written.to_path_buf()));
    }
    if path.file_name() == Some(OsStr::new(SKILL_FILE)) {
        let folder = written.parent().unwrap_or(Path::new(""));
        return Ok((path, folder.to_path_buf()));
    }
    Err(SkillsError::NotSkillFile {
        path: written.to_path_buf(),
    })
}

/// The absolute path of the folder that holds `skill_file`. A skill folder
/// that is a link keeps its own name and its place, because Claude Code
/// names a skill by its entry in the skills folder.
fn absolute_skill_folder(skill_file: &Path) -> io::Result<PathBuf> {
    let folder = skill_file.parent().unwrap_or(Path::new("."));
    match (folder.parent(), folder.file_name()) {
        (Some(parent), Some(name)) if !parent.as_os_str().is_empty() => {
            Ok(parent.canonicalize()?.join(name))
        }
        // Known limitation: a path such as `.` names the working folder, which
        // is always the real folder, so a linked skill checked from inside
        // itself is named by the folder that the link points to.
        _ => folder.canonicalize(),
    }
}

/// Every Markdown file at or below `root`, relative to it: `SKILL.md` first,
/// then the others in path order.
fn markdown_files(root: &Path) -> Result<Vec<PathBuf>, SkillsError> {
    let mut files = Vec::new();
    let mut folders = vec![root.to_path_buf()];
    while let Some(folder) = folders.pop() {
        for entry in fs::read_dir(&folder).map_err(SkillsError::read(&folder))? {
            let entry = entry.map_err(SkillsError::read(&folder))?;
            let path = entry.path();
            let file_type = entry.file_type().map_err(SkillsError::read(&path))?;
            // A link to a folder is followed only at the top, where a skills
            // folder may link to a skill. Deeper links are not followed, so a
            // loop of links cannot make the walk run forever.
            let is_linked_folder = folder == root && file_type.is_symlink() && path.is_dir();
            if file_type.is_dir() || is_linked_folder {
                folders.push(path);
            } else if path.extension() == Some(OsStr::new("md")) && path.is_file() {
                let relative = path.strip_prefix(root).unwrap_or(&path);
                files.push(relative.to_path_buf());
            }
        }
    }
    files.sort_by_key(|file| (file != Path::new(SKILL_FILE), file.clone()));
    Ok(files)
}

fn read_text(path: &Path) -> Result<String, SkillsError> {
    let bytes = fs::read(path).map_err(SkillsError::read(path))?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

/// Each Markdown file of each skill in `skills_folder`, relative to it, such
/// as `rex-deliver/SKILL.md`.
fn skill_paths(skills_folder: &Path) -> Result<BTreeSet<PathBuf>, SkillsError> {
    let mut paths = BTreeSet::new();
    for folder in list::skill_folders(skills_folder)? {
        let name = file_name(&folder);
        for file in markdown_files(&folder)? {
            paths.insert(Path::new(&name).join(file));
        }
    }
    Ok(paths)
}

/// [`skill_paths`] of the skills that `rex skills list` finds from `cwd`, and
/// of the skills in the bundle that `rex init` installs, each in the set of
/// its folder: the library's, or that of a `.claude/skills/`.
fn skill_paths_outside(cwd: &Path) -> Result<KnownSkills, SkillsError> {
    let mut known = KnownSkills::default();
    for place in list::places(cwd)? {
        let paths = skill_paths(&place.folder)?;
        match place.source {
            Source::Library => known.library_paths.extend(paths),
            Source::Repo | Source::User => known.skill_paths.extend(paths),
        }
    }
    for (path, _) in bundle::walk() {
        if path.extension() != Some(OsStr::new("md")) {
            continue;
        }
        if let Ok(in_library) = path.strip_prefix(LIBRARY_ROOT) {
            known.library_paths.insert(in_library.to_path_buf());
        } else if let Ok(in_skills_folder) = path.strip_prefix(SKILLS_ROOT) {
            known.skill_paths.insert(in_skills_folder.to_path_buf());
        }
    }
    Ok(known)
}

/// Check each skill in `paths` and print one `ok` line for each skill that
/// passes, and one line for each finding.
///
/// # Errors
/// - [`SkillsError::Missing`], [`SkillsError::NoSkillFile`],
///   [`SkillsError::NotSkillFile`] or [`SkillsError::Read`] for the first
///   path that is not a readable skill. Nothing is printed then.
/// - [`SkillsError::Findings`] with the counts when any rule is broken.
/// - [`SkillsError::Rex`] if standard output cannot be written.
pub(super) fn run(cwd: &Path, paths: &[PathBuf]) -> Result<(), SkillsError> {
    let mut skills = Vec::new();
    for path in paths {
        skills.push(LoadedSkill::load(cwd, path)?);
    }
    let outside = skill_paths_outside(cwd)?;

    let mut findings = 0;
    let mut skills_with_findings = 0;
    for loaded in &skills {
        let mut known = loaded.neighbours.clone();
        known
            .skill_paths
            .extend(outside.skill_paths.iter().cloned());
        known
            .library_paths
            .extend(outside.library_paths.iter().cloned());
        let found = check_skill(&loaded.skill, &known);
        if found.is_empty() {
            print_line_to_stdout(&format!("ok {}", loaded.written.display()))?;
            continue;
        }
        findings += found.len();
        skills_with_findings += 1;
        for finding in found {
            let file = loaded.written_folder.join(&finding.file);
            let line = format!(
                "{}: {}: {}",
                file.display(),
                finding.rule.id(),
                finding.message
            );
            print_line_to_stdout(&line)?;
        }
    }

    if findings > 0 {
        return Err(SkillsError::Findings {
            findings,
            skills_with_findings,
            skills: skills.len(),
        });
    }
    Ok(())
}
