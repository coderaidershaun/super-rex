//! The rules about the files a skill's text points to: each Markdown link
//! must name a file of the same skill, and each skill file named as
//! `<skill>/<file>.md` must exist (rules B3 and B4).

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use super::SKILL_FILE;

/// The folders whose skills a skill's text may name, as a person reads them
/// in a message. `rex_skill` finds a skill in one of them.
const SKILL_FOLDERS_TEXT: &str = "plugin/skills, plugin/library or .rex/skills";

/// The folders that 0.7 skill text named a skill by. A path that starts with
/// one names a folder that changes with each plugin version.
const FOLDER_PREFIXES: [&str; 4] = [
    ".claude/skills/",
    ".rex/skills/",
    "plugin/skills/",
    "plugin/library/",
];

/// The start of every library skill and gap skill name. An unknown first
/// folder with it is a missing skill; any other unknown first folder is a
/// path in the repo that the skill works on, such as
/// `tests/integration/README.md`.
const SKILL_NAME_PREFIX: &str = "rex-";

/// The skills that a skill's text may name.
#[derive(Debug, Clone, Default)]
pub struct KnownSkills {
    /// Each Markdown file of each skill, as text names it: `<skill>/<file>`,
    /// such as `rex-code-tdd/mocking.md`. A skill is known when its
    /// `SKILL.md` is here (rule B4).
    pub paths: BTreeSet<PathBuf>,
}

/// A message for each Markdown link in `text` to a `.md` file that is not one
/// of `own_files`. `file` is the path of the file that holds the links, and
/// `own_files` the skill's Markdown files, both written as `<skill>/<file>`.
pub(super) fn broken_links(file: &Path, text: &str, own_files: &BTreeSet<PathBuf>) -> Vec<String> {
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
        if !resolved.is_some_and(|target| own_files.contains(&target)) {
            messages.push(format!(
                "link `{link}` does not point to a file of this skill; \
                 name a file of another skill as `<skill>/<file>.md`"
            ));
        }
    }
    messages
}

/// The path of `target`, written relative to the folder of `from`. `None`
/// when it climbs above the top of `from`.
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

/// A message for each code span of `text` that names a skill file that is
/// not there, or names one by its folder, in the order they appear.
pub(super) fn unknown_skill_paths(text: &str, known: &KnownSkills) -> Vec<String> {
    code_spans(text)
        .filter_map(|span| span_problem(span, known))
        .collect()
}

/// The text of each code span on one line, such as `rex-code-tdd/mocking.md`
/// for `` `rex-code-tdd/mocking.md` ``. A fence of three backticks makes no
/// span, because the span between its first two backticks is empty.
fn code_spans(text: &str) -> impl Iterator<Item = &str> {
    text.lines().flat_map(|line| {
        let parts: Vec<&str> = line.split('`').collect();
        let closed = parts.len().saturating_sub(1) / 2 * 2;
        parts
            .into_iter()
            .take(closed + 1)
            .skip(1)
            .step_by(2)
            .filter(|span| !span.is_empty())
    })
}

/// What is wrong with the skill file that `span` names, if it names one. A
/// span with a placeholder such as `<skill>` is a template, which names no
/// file, so its characters never fit.
fn span_problem(span: &str, known: &KnownSkills) -> Option<String> {
    if let Some(in_folder) = FOLDER_PREFIXES
        .iter()
        .find_map(|prefix| span.strip_prefix(prefix))
        && skill_file_name(in_folder).is_some()
    {
        return Some(format!(
            "{span} names a skill file by its folder, which changes with each plugin version; \
             write {in_folder} and read it with rex_skill"
        ));
    }

    let (name, file) = skill_file_name(span)?;
    let skill_file = Path::new(name).join(SKILL_FILE);
    if known.paths.contains(&skill_file) {
        // A `..` stays in the joined path and matches no known file, because
        // `rex_skill` refuses a file that climbs out of its skill.
        let is_there = known.paths.contains(&Path::new(name).join(file));
        return (!is_there).then(|| format!("{span} is not a file of the skill `{name}`"));
    }
    name.starts_with(SKILL_NAME_PREFIX)
        .then(|| format!("{span} names `{name}`, which is no skill in {SKILL_FOLDERS_TEXT}"))
}

/// The skill and the file of `span` when it has the form `<skill>/<file>.md`:
/// a skill name, then a path of letters, digits and `._/-` that ends in `.md`.
fn skill_file_name(span: &str) -> Option<(&str, &str)> {
    let (name, file) = span.split_once('/')?;
    let is_name = !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    let is_file = file.ends_with(".md")
        && file
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_./".contains(&byte));
    (is_name && is_file).then_some((name, file))
}
