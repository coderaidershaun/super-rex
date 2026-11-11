//! The rules a skill must pass, run over text in memory. `rex skills check`
//! gives it files from disk; the bundle tests give it files embedded in the
//! binary, so both use exactly the same rules.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use super::SKILL_FILE;
use super::header::{Header, HeaderError, Parts};

/// Claude Code cuts a skill's listing entry, `description - when_to_use`, at
/// this many characters.
const LISTING_LIMIT: usize = 1536;

/// The length of ` - `, which Claude Code puts between `description` and
/// `when_to_use`.
const LISTING_SEPARATOR_LENGTH: usize = 3;

const MAX_NAME_LENGTH: usize = 64;

const MAX_SKILL_LINES: usize = 500;

const MAX_BODY_WORDS: usize = 1500;

const MAX_HEADER_WORDS: usize = 60;

/// The "use when" phrases that belong in `when_to_use`, not in `description`.
const USE_WHEN_PHRASES: [&str; 5] = [
    "Use when",
    "Use this skill when",
    "Use it when",
    "Use it also when",
    "Use whenever",
];

/// The header keys Claude Code reads. Claude Code ignores any other key and
/// says nothing, so a misspelt key is lost.
const KNOWN_KEYS: [&str; 20] = [
    "name",
    "description",
    "when_to_use",
    "argument-hint",
    "arguments",
    "disable-model-invocation",
    "user-invocable",
    "allowed-tools",
    "disallowed-tools",
    "model",
    "effort",
    "context",
    "agent",
    "background",
    "hooks",
    "paths",
    "shell",
    "metadata",
    "license",
    "compatibility",
];

/// The two folders of a repo that hold skills, as a skill's text names them.
const SKILLS_FOLDER_TEXT: &str = ".claude/skills/";
const LIBRARY_FOLDER_TEXT: &str = ".rex/skills/";

/// One rule of a skill. The order is the order findings are given in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rule {
    /// The header opens on line 1 with `---` and is closed by `---`. Claude Code
    /// reads a header only when `---` is the first line.
    H1,
    /// The header is valid YAML and a mapping. Claude Code loads a skill with a
    /// broken header with no fields set, and says nothing.
    H2,
    /// `name` is the folder name, in lowercase words joined by hyphens, because
    /// `rex init` and the bundle tests find a skill by its folder.
    H3,
    /// `description` is not empty and holds no "use when" phrase. Claude Code
    /// shows both keys, so that text would be given twice.
    H4,
    /// `when_to_use` is not empty and starts with `Use when`. The builder
    /// chooses skills by this line.
    H5,
    /// A plain `description` or `when_to_use` is not cut short by YAML.
    H6,
    /// The listing entry fits in Claude Code's limit.
    H7,
    /// Every header key is one Claude Code reads.
    H8,
    /// `description` and `when_to_use` together have at most 60 words. Claude
    /// Code lists every skill in every session, and drops descriptions when
    /// the list grows too long.
    H9,
    /// `SKILL.md` has text after the header.
    B1,
    /// `SKILL.md` has at most 500 lines. A loaded skill stays in context, so
    /// long material belongs in files that the skill links to.
    B2,
    /// Each link to a Markdown file points to a file in the skills folder. A
    /// link out of that folder breaks once the skill is installed in another repo.
    B3,
    /// Each `.claude/skills/<name>/<file>.md` and `.rex/skills/<name>/<file>.md`
    /// names a Markdown file of a known skill in that folder: its `SKILL.md`,
    /// or a file in its folder or below it. An agent told to read a file that
    /// is not there goes on without it.
    B4,
    /// The text after the header of `SKILL.md` has at most 1,500 words. An
    /// agent reads all of `SKILL.md`, so the text for one step or one case
    /// belongs in a sibling file that the agent reads only when it needs it.
    B5,
}

impl Rule {
    /// The permanent rule ID, such as `H1`.
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Rule::H1 => "H1",
            Rule::H2 => "H2",
            Rule::H3 => "H3",
            Rule::H4 => "H4",
            Rule::H5 => "H5",
            Rule::H6 => "H6",
            Rule::H7 => "H7",
            Rule::H8 => "H8",
            Rule::H9 => "H9",
            Rule::B1 => "B1",
            Rule::B2 => "B2",
            Rule::B3 => "B3",
            Rule::B4 => "B4",
            Rule::B5 => "B5",
        }
    }
}

/// One broken rule in one file of a skill.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// The file the finding is in, relative to the skill folder.
    pub file: PathBuf,
    pub rule: Rule,
    /// What is wrong, in words for a person.
    pub message: String,
}

/// One file of a skill: `SKILL.md` or any other Markdown file in the skill
/// folder or below it.
#[derive(Debug, Clone)]
pub struct SkillFile {
    /// The path relative to the skill folder, such as `SKILL.md`.
    pub path: PathBuf,
    pub text: String,
}

/// One skill to check.
#[derive(Debug, Clone)]
pub struct Skill {
    /// The name of the skill folder.
    pub folder: String,
    /// `SKILL.md` and the skill's other Markdown files.
    pub files: Vec<SkillFile>,
}

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

/// Checks one skill against every rule and gives the findings in rule order,
/// then in the order they appear. When the header cannot be read (H1, H2),
/// the other header rules are not run. A skill with no `SKILL.md` in its
/// files breaks H1.
#[must_use]
pub fn check_skill(skill: &Skill, known: &KnownSkills) -> Vec<Finding> {
    let skill_md = Path::new(SKILL_FILE);
    let mut findings = Vec::new();
    let mut finding = |file: &Path, rule: Rule, message: String| {
        findings.push(Finding {
            file: file.to_path_buf(),
            rule,
            message,
        });
    };

    match skill.files.iter().find(|file| file.path == skill_md) {
        Some(file) => {
            for (rule, message) in skill_md_findings(&skill.folder, &file.text) {
                finding(skill_md, rule, message);
            }
        }
        None => finding(skill_md, Rule::H1, "SKILL.md is missing".to_owned()),
    }
    for file in &skill.files {
        let in_skills_folder = Path::new(&skill.folder).join(&file.path);
        for message in broken_links(&in_skills_folder, &file.text, &known.files) {
            finding(&file.path, Rule::B3, message);
        }
        for message in unknown_skill_paths(&file.text, known) {
            finding(&file.path, Rule::B4, message);
        }
    }

    findings.sort_by_key(|finding| finding.rule);
    findings
}

/// The findings of every rule that reads `SKILL.md` alone: H1 to H9, B1, B2
/// and B5.
fn skill_md_findings(folder: &str, text: &str) -> Vec<(Rule, String)> {
    let mut findings = Vec::new();
    let body = match Parts::split(text) {
        None => {
            findings.push((
                Rule::H1,
                "line 1 must be `---`, and a later line `---` must close the header".to_owned(),
            ));
            text.to_owned()
        }
        Some(parts) => {
            match Header::parse(&parts.header) {
                Ok(header) => findings.extend(header_findings(folder, &header)),
                Err(HeaderError::Yaml(error)) => {
                    findings.push((Rule::H2, format!("the header is not valid YAML: {error}")));
                }
                Err(HeaderError::NotMapping) => {
                    findings.push((Rule::H2, "the header is not a YAML mapping".to_owned()));
                }
            }
            parts.body
        }
    };

    if body.trim().is_empty() {
        findings.push((Rule::B1, "there is no text after the header".to_owned()));
    }
    let lines = text.lines().count();
    if lines > MAX_SKILL_LINES {
        findings.push((
            Rule::B2,
            format!("{lines} lines, more than {MAX_SKILL_LINES}; move long material into a reference file"),
        ));
    }
    let words = word_count(&body);
    if words > MAX_BODY_WORDS {
        findings.push((
            Rule::B5,
            format!("{words} words after the header, more than {MAX_BODY_WORDS}; move the text for one step or one case into a sibling file, and say in SKILL.md when to read it"),
        ));
    }
    findings
}

fn header_findings(folder: &str, header: &Header) -> Vec<(Rule, String)> {
    let mut findings = Vec::new();
    findings.extend(name_problem(folder, header).map(|message| (Rule::H3, message)));
    findings.extend(description_problem(header).map(|message| (Rule::H4, message)));
    findings.extend(when_to_use_problem(header).map(|message| (Rule::H5, message)));
    let keys = header.keys();
    for key in keys
        .iter()
        .filter(|key| ["description", "when_to_use"].contains(&key.as_str()))
    {
        findings.extend(shortened_by_yaml(header, key).map(|message| (Rule::H6, message)));
    }
    findings.extend(listing_too_long(header).map(|message| (Rule::H7, message)));
    findings.extend(header_too_wordy(header).map(|message| (Rule::H9, message)));
    for key in keys {
        if !KNOWN_KEYS.contains(&key.as_str()) {
            let message = format!("`{key}` is not a key Claude Code reads, so it is ignored");
            findings.push((Rule::H8, message));
        }
    }
    findings
}

fn name_problem(folder: &str, header: &Header) -> Option<String> {
    let Some(name) = header.value("name") else {
        return Some("name is missing".to_owned());
    };
    let Some(name) = name.as_str() else {
        return Some("name is not a string".to_owned());
    };
    if name != folder {
        return Some(format!("name `{name}` is not the folder name `{folder}`"));
    }
    let words_are_valid = name.split('-').all(|word| {
        !word.is_empty()
            && word
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    });
    if !words_are_valid || name.len() > MAX_NAME_LENGTH {
        return Some(format!(
            "name `{name}` must be lowercase letters and digits in groups joined by single hyphens, at most {MAX_NAME_LENGTH} characters"
        ));
    }
    None
}

/// The text of `key`, or what is wrong with it: missing, not a string, or
/// empty.
fn required_text(header: &Header, key: &str) -> Result<String, String> {
    let Some(value) = header.value(key) else {
        return Err(format!("{key} is missing"));
    };
    if value.as_str().is_none() {
        return Err(format!("{key} is not a string"));
    }
    let text = header.text(key).unwrap_or_default();
    if text.is_empty() {
        return Err(format!("{key} is empty"));
    }
    Ok(text)
}

fn description_problem(header: &Header) -> Option<String> {
    let text = match required_text(header, "description") {
        Ok(text) => text,
        Err(problem) => return Some(problem),
    };
    let phrase = USE_WHEN_PHRASES
        .iter()
        .find(|phrase| text.contains(*phrase))?;
    Some(format!(
        "description holds `{phrase}`; move the \"use when\" text into when_to_use"
    ))
}

fn when_to_use_problem(header: &Header) -> Option<String> {
    let text = match required_text(header, "when_to_use") {
        Ok(text) => text,
        Err(problem) => return Some(problem),
    };
    (!text.starts_with("Use when")).then(|| "when_to_use must start with `Use when`".to_owned())
}

/// In a plain YAML value, ` #` starts a comment, so YAML can give back less
/// text than was written.
fn shortened_by_yaml(header: &Header, key: &str) -> Option<String> {
    let written = header.written_plain_text(key)?;
    let parsed = header.text(key)?;
    (parsed != written).then(|| {
        format!(
            "YAML reads {key} as `{parsed}`, not as written; fold it with `>` or quote it, because ` #` starts a comment"
        )
    })
}

fn listing_too_long(header: &Header) -> Option<String> {
    let description = header.text("description").unwrap_or_default();
    let when_to_use = header.text("when_to_use").unwrap_or_default();
    let length =
        description.chars().count() + LISTING_SEPARATOR_LENGTH + when_to_use.chars().count();
    (length > LISTING_LIMIT).then(|| {
        format!(
            "description and when_to_use make a listing entry of {length} characters; Claude Code cuts it at {LISTING_LIMIT}"
        )
    })
}

/// Words as `wc -w` counts them: runs of characters between white space.
fn word_count(text: &str) -> usize {
    text.split_whitespace().count()
}

fn header_too_wordy(header: &Header) -> Option<String> {
    let words: usize = ["description", "when_to_use"]
        .iter()
        .map(|key| word_count(&header.text(key).unwrap_or_default()))
        .sum();
    (words > MAX_HEADER_WORDS).then(|| {
        format!(
            "description and when_to_use have {words} words together, more than {MAX_HEADER_WORDS}"
        )
    })
}

/// A message for each Markdown link in `text` to a `.md` file that is not in
/// `existing`. `file` is the path of the file that holds the links, relative
/// to the skills folder.
fn broken_links(file: &Path, text: &str, existing: &BTreeSet<PathBuf>) -> Vec<String> {
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
fn unknown_skill_paths(text: &str, known: &KnownSkills) -> Vec<String> {
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
