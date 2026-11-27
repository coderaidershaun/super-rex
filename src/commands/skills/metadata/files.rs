//! The rules for what the map points to: the files of the skill folder, and
//! the skills and files that `open_when`, `requires` and `replaces` name
//! (rules M8 to M11). An agent sent to a file that is not there goes on
//! without it.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use yaml_rust2::Yaml;
use yaml_rust2::yaml::Hash;

use crate::commands::skills::header::Metadata;
use crate::commands::skills::references::SKILL_FOLDERS_TEXT;
use crate::commands::skills::{Rule, SKILL_FILE};

use super::fields::{FILE_INTENT_COUNT, intents_problem};
use super::vocabulary::Vocabulary;
use super::{SkillFolder, get, shown, text_list};

const ROLES: [&str; 3] = ["topic", "language", "code"];

/// The roles of a file that holds the text of one language.
const LANGUAGE_ROLES: [&str; 2] = ["language", "code"];

pub(super) fn findings(map: &Metadata, skill: &SkillFolder) -> Vec<(Rule, String)> {
    let mut findings = Vec::new();
    let (listed, problems) = match map.get("files") {
        None => (Some(BTreeSet::new()), Vec::new()),
        Some(files) => listed_files(files),
    };
    findings.extend(problems.into_iter().map(|message| (Rule::M8, message)));
    if let Some(listed) = listed {
        let messages = unlisted_or_missing(&listed, skill);
        findings.extend(messages.into_iter().map(|message| (Rule::M9, message)));
    }
    if let Some(open_when) = map.get("open_when") {
        let messages = open_when_problems(open_when, skill);
        findings.extend(messages.into_iter().map(|message| (Rule::M10, message)));
    }
    for key in ["requires", "replaces"] {
        if let Some(names) = map.get(key) {
            let messages = skill_names_problems(key, names, skill);
            findings.extend(messages.into_iter().map(|message| (Rule::M11, message)));
        }
    }
    findings
}

/// The paths that `files` lists, and what is wrong with its items. The paths
/// are `None` when `files` is not a list, so that no file is called unlisted
/// for that.
fn listed_files(files: &Yaml) -> (Option<BTreeSet<PathBuf>>, Vec<String>) {
    let Some(items) = files.as_vec() else {
        return (None, vec![format!("files is {}, not a list", shown(files))]);
    };
    let mut listed = BTreeSet::new();
    let mut problems = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let Some(item) = item.as_hash() else {
            problems.push(format!(
                "files item {} is {}, not a map; write it as {{ path: <file>, role: topic }}",
                index + 1,
                shown(item)
            ));
            continue;
        };
        let Some(path) = get(item, "path").and_then(Yaml::as_str) else {
            problems.push(format!("files item {} has no `path`", index + 1));
            continue;
        };
        if path == SKILL_FILE {
            problems.push(
                "files lists SKILL.md, which is the entry file of the skill and is never listed"
                    .to_owned(),
            );
            continue;
        }
        listed.insert(PathBuf::from(path));
        let messages = file_item_problems(item);
        problems.extend(
            messages
                .into_iter()
                .map(|problem| format!("files item `{path}` {problem}")),
        );
    }
    (Some(listed), problems)
}

/// What is wrong with the fields of one `files` item, other than its path.
fn file_item_problems(item: &Hash) -> Vec<String> {
    let mut problems = Vec::new();
    match get(item, "role") {
        None => problems.push(format!("has no `role`; give {}", ROLES.join(", "))),
        Some(role) => {
            let role_text = role.as_str().filter(|role| ROLES.contains(role));
            match role_text {
                None => problems.push(format!(
                    "has role {}, which is not one of {}",
                    shown(role),
                    ROLES.join(", ")
                )),
                Some(role) if LANGUAGE_ROLES.contains(&role) => {
                    problems.extend(language_problem(item, role));
                }
                Some(_) => {}
            }
        }
    }
    if let Some(when) = get(item, "when").filter(|when| when.as_str().is_none()) {
        problems.push(format!("has a `when` that is {}, not text", shown(when)));
    }
    if let Some(intents) = get(item, "intents") {
        problems.extend(intents_problem("intents", intents, &FILE_INTENT_COUNT));
    }
    problems
}

fn language_problem(item: &Hash, role: &str) -> Option<String> {
    let Some(language) = get(item, "language") else {
        return Some(format!("is a {role} file with no `language`"));
    };
    let languages = Vocabulary::embedded()
        .fact("languages")
        .expect("the vocabulary names the fact `languages`, whose values a language file takes");
    languages
        .value_problem(language)
        .map(|problem| format!("names the language {problem}"))
}

/// A message for each file of the folder that `listed` leaves out, and for
/// each listed file that is not in the folder. A file whose name starts with
/// `.`, such as `.DS_Store`, is never part of a skill.
fn unlisted_or_missing(listed: &BTreeSet<PathBuf>, skill: &SkillFolder) -> Vec<String> {
    let in_folder: BTreeSet<&PathBuf> = skill
        .files
        .iter()
        .filter(|file| file.as_path() != Path::new(SKILL_FILE) && !is_hidden(file))
        .collect();
    let unlisted = in_folder
        .iter()
        .filter(|file| !listed.contains(**file))
        .map(|file| {
            format!(
                "`{}` is in the skill folder, but files does not list it",
                file.display()
            )
        });
    let missing = listed
        .iter()
        .filter(|file| !in_folder.contains(file))
        .map(|file| {
            format!(
                "files lists `{}`, which is not in the skill folder",
                file.display()
            )
        });
    unlisted.chain(missing).collect()
}

fn is_hidden(file: &Path) -> bool {
    file.components().any(|component| {
        matches!(component, Component::Normal(name) if name.as_encoded_bytes().starts_with(b"."))
    })
}

fn open_when_problems(open_when: &Yaml, skill: &SkillFolder) -> Vec<String> {
    let Some(items) = open_when.as_vec() else {
        return vec![format!("open_when is {}, not a list", shown(open_when))];
    };
    let mut problems = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let number = index + 1;
        let Some(item) = item.as_hash() else {
            problems.push(format!(
                "open_when item {number} is {}, not a map; write it as {{ skill: <skill>, when: <case> }}",
                shown(item)
            ));
            continue;
        };
        let has_when = get(item, "when")
            .and_then(Yaml::as_str)
            .is_some_and(|when| !when.trim().is_empty());
        if !has_when {
            problems.push(format!(
                "open_when item {number} has no `when` text that says the case"
            ));
        }
        let target = (get(item, "skill"), get(item, "file"));
        problems.extend(target_problem(number, target, skill));
    }
    problems
}

/// What is wrong with the target of one `open_when` item: another skill, a
/// file of this skill, or a file of another skill.
fn target_problem(
    number: usize,
    (skill_name, file): (Option<&Yaml>, Option<&Yaml>),
    skill: &SkillFolder,
) -> Option<String> {
    let as_text = |value, key| {
        optional_text(value).map_err(|value| {
            format!("open_when item {number} has a `{key}` that is {value}, not text")
        })
    };
    let (skill_name, file) = match (as_text(skill_name, "skill"), as_text(file, "file")) {
        (Ok(skill_name), Ok(file)) => (skill_name, file),
        (Err(problem), _) | (_, Err(problem)) => return Some(problem),
    };
    match (skill_name, file) {
        (None, None) => Some(format!(
            "open_when item {number} names no `skill` and no `file`"
        )),
        (Some(name), _) if !skill.known.has_skill(name) => Some(no_skill("open_when", name)),
        (Some(name), Some(file)) => (!skill.known.has_file(name, file)).then(|| {
            format!("open_when names `{name}/{file}`, which is not a file of the skill `{name}`")
        }),
        (Some(_), None) => None,
        (None, Some(file)) => (!skill.files.contains(&PathBuf::from(file)))
            .then(|| format!("open_when names `{file}`, which is not a file of this skill")),
    }
}

/// The text of an optional field, or the field shown for a message when it
/// is not text.
fn optional_text(value: Option<&Yaml>) -> Result<Option<&str>, String> {
    match value {
        None => Ok(None),
        Some(value) => value.as_str().map(Some).ok_or_else(|| shown(value)),
    }
}

fn skill_names_problems(key: &str, names: &Yaml, skill: &SkillFolder) -> Vec<String> {
    let Some(names) = text_list(names) else {
        return vec![format!("{key} is not a list of skill names")];
    };
    names
        .into_iter()
        .filter(|name| !skill.known.has_skill(name))
        .map(|name| no_skill(key, name))
        .collect()
}

fn no_skill(key: &str, name: &str) -> String {
    format!("{key} names `{name}`, which is no skill in {SKILL_FOLDERS_TEXT}")
}
