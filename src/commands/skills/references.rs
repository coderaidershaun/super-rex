//! The rules about the files a skill's text points to: each Markdown link and
//! each skills-folder path must name a file that exists (rules B3 and B4).

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use super::SKILL_FILE;

/// The two folders of a repo that hold skills, as a skill's text names them.
const SKILLS_FOLDER_TEXT: &str = ".claude/skills/";
const LIBRARY_FOLDER_TEXT: &str = ".rex/skills/";

/// What exists around the skill being checked.
#[derive(Debug, Clone, Default)]
pub struct KnownSkills {
    /// Each Markdown file of each skill that a `.claude/skills/` path may name,
    /// relative to that folder, such as `rex-deliver/SKILL.md`. A skill is
    /// known when its `SKILL.md` is here (rule B4).
    pub skill_paths: BTreeSet<PathBuf>,
    /// The same for the skill library, which a `.rex/skills/` path names, such
    /// as `rex-code-tdd/SKILL.md`.
    pub library_paths: BTreeSet<PathBuf>,
    /// Each Markdown file in the skills folder, relative to it, such as
    /// `rex-code-tdd/SKILL.md`. A link may point into another skill (rule B3).
    pub files: BTreeSet<PathBuf>,
}

/// A message for each Markdown link in `text` to a `.md` file that is not in
/// `existing`. `file` is the path of the file that holds the links, relative
/// to the skills folder.
pub(super) fn broken_links(file: &Path, text: &str, existing: &BTreeSet<PathBuf>) -> Vec<String> {
    let mut messages = Vec::new();
    for (at, _) in text.match_indices("](") {
        let rest = &text[at + 2..];
        let Some(link) = rest.find(')').map(|end| &rest[..end]) else {
            continue;
        };
        let target_file = link.split('#').next().unwrap_or_default();
        if !target_file.ends_with(".md") || target_file.contains("://") {
            continue;
        }
        let resolved = resolve(file, target_file);
        if !resolved.is_some_and(|target| existing.contains(&target)) {
            messages.push(format!(
                "link `{link}` does not point to a file in the skills folder"
            ));
        }
    }
    messages
}

/// The path of `target`, written relative to the folder of `from`, inside
/// the skills folder. `None` when it leaves the skills folder.
fn resolve(from: &Path, target: &str) -> Option<PathBuf> {
    let mut resolved = from.parent()?.to_path_buf();
    for component in Path::new(target).components() {
        match component {
            Component::Normal(part) => resolved.push(part),
            Component::ParentDir => {
                if !resolved.pop() {
                    return None;
                }
            }
            Component::CurDir => {}
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(resolved)
}

/// A message for each `.claude/skills/<name>/<file>.md` and
/// `.rex/skills/<name>/<file>.md` in `text` that names no file of a known
/// skill in that folder, in the order they appear.
pub(super) fn unknown_skill_paths(text: &str, known: &KnownSkills) -> Vec<String> {
    let skills = (SKILLS_FOLDER_TEXT, &known.skill_paths);
    let library = (LIBRARY_FOLDER_TEXT, &known.library_paths);
    let mut problems = folder_problems(text, skills, library);
    problems.extend(folder_problems(text, library, skills));
    problems.sort_by_key(|(at, _)| *at);
    problems.into_iter().map(|(_, message)| message).collect()
}

/// Each path below `folder` in `text` that names no file of a skill known
/// there, with the place it starts. A path that names a skill of `other`
/// instead is in the wrong folder. A placeholder such as `<name>` or
/// `<file>` makes the path a template, which names no file, so it is not
/// checked.
fn folder_problems(
    text: &str,
    (folder, known): (&str, &BTreeSet<PathBuf>),
    (other, other_known): (&str, &BTreeSet<PathBuf>),
) -> Vec<(usize, String)> {
    let mut problems = Vec::new();
    for (at, _) in text.match_indices(folder) {
        let after = &text[at + folder.len()..];
        let name_length = after
            .bytes()
            .take_while(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
            .count();
        let name = &after[..name_length];
        let Some(rest) = after[name_length..].strip_prefix('/') else {
            continue;
        };
        let file_length = rest
            .bytes()
            .take_while(|byte| byte.is_ascii_alphanumeric() || b"-_./".contains(byte))
            .count();
        // A path at the end of a sentence is followed by its full stop.
        let file = rest[..file_length].trim_end_matches('.');
        if name.is_empty() || !file.ends_with(".md") {
            continue;
        }

        let written = format!("{folder}{name}/{file}");
        let skill_file = Path::new(name).join(SKILL_FILE);
        let message = if known.contains(&skill_file) {
            if resolve(&skill_file, file).is_some_and(|target| known.contains(&target)) {
                continue;
            }
            format!("{written} is not a file of the skill `{name}`")
        } else if other_known.contains(&skill_file) {
            format!("{written} names `{name}`, a skill in {other}; write {other}{name}/{file}")
        } else if file == SKILL_FILE {
            format!("{written} is not a known skill")
        } else {
            format!("{written} is in `{name}`, which is not a known skill")
        };
        problems.push((at, message));
    }
    problems
}
