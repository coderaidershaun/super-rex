//! `rex skills check`: reads each skill from disk, runs the rules on it, and
//! prints `ok` or the findings. Every file is read before anything is
//! printed, so a bad path stops the command with no partial output.

use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::SkillsError;
use super::list;
use super::rules::{KnownSkills, Skill, SkillFile, check_skill};
use crate::commands::stdout::print_line_to_stdout;

const SKILL_FILE: &str = "SKILL.md";

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

        let (skill_file, written_folder) = if metadata.is_dir() {
            let skill_file = path.join(SKILL_FILE);
            if !skill_file.is_file() {
                return Err(SkillsError::NoSkillFile {
                    path: written.to_path_buf(),
                });
            }
            (skill_file, written.to_path_buf())
        } else if path.file_name() == Some(OsStr::new(SKILL_FILE)) {
            let folder = written.parent().unwrap_or(Path::new(""));
            (path, folder.to_path_buf())
        } else {
            return Err(SkillsError::NotSkillFile {
                path: written.to_path_buf(),
            });
        };

        let folder = skill_file.parent().unwrap_or(Path::new("."));
        let folder = folder.canonicalize().map_err(SkillsError::read(written))?;
        let mut files = Vec::new();
        for file in markdown_files(&folder)? {
            let text = read_text(&folder.join(&file))?;
            files.push(SkillFile { path: file, text });
        }
        let skills_folder = folder.parent().unwrap_or(&folder);

        Ok(LoadedSkill {
            written: written.to_path_buf(),
            written_folder,
            skill: Skill {
                folder: file_name(&folder),
                files,
            },
            neighbours: KnownSkills {
                names: skill_names(skills_folder)?,
                files: markdown_files(skills_folder)?.into_iter().collect(),
            },
        })
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
            // A link to a folder is not followed, so a loop of links cannot
            // make the walk run forever. A link to a file is read.
            if file_type.is_dir() {
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

fn skill_names(skills_folder: &Path) -> Result<BTreeSet<String>, SkillsError> {
    let folders = list::skill_folders(skills_folder)?;
    Ok(folders.iter().map(|folder| file_name(folder)).collect())
}

/// The names of the skills that `rex skills list` finds from `cwd`, and of
/// the skills in the bundle that `rex init` installs.
fn names_outside(cwd: &Path) -> Result<BTreeSet<String>, SkillsError> {
    let mut names = BTreeSet::new();
    for place in list::places(cwd)? {
        names.extend(skill_names(&place.folder)?);
    }
    for (path, _) in crate::bundle::walk() {
        let parts: Vec<&OsStr> = path.iter().collect();
        if let [skills, name, file] = parts.as_slice()
            && *skills == "skills"
            && *file == SKILL_FILE
        {
            names.insert(name.to_string_lossy().into_owned());
        }
    }
    Ok(names)
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
    let names_outside = names_outside(cwd)?;

    let mut findings = 0;
    let mut skills_with_findings = 0;
    for loaded in &skills {
        let mut known = loaded.neighbours.clone();
        known.names.extend(names_outside.iter().cloned());
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
