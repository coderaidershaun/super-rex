//! The skills embedded in the binary are checked with the same rules as
//! `rex skills check`, so that `rex init` never installs a broken skill.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super_rex::bundle::walk;
use super_rex::commands::skills::{KnownSkills, Rule, Skill, SkillFile, check_skill};

const SKILL_FILE: &str = "SKILL.md";

/// Each skill folder of the bundle with its Markdown files, and the skills
/// and Markdown files around them. The bundle is Markdown, so a Markdown file
/// that is not UTF-8 is a mistake.
fn bundle_skills() -> (Vec<Skill>, KnownSkills) {
    let mut files_by_folder: BTreeMap<String, Vec<SkillFile>> = BTreeMap::new();
    let mut known = KnownSkills::default();

    for (path, bytes) in walk() {
        let Ok(in_skills_folder) = path.strip_prefix("skills") else {
            continue;
        };
        if in_skills_folder.extension().is_none_or(|ext| ext != "md") {
            continue;
        }
        let text = String::from_utf8(bytes.to_vec())
            .unwrap_or_else(|_| panic!("{} is not UTF-8", path.display()));
        known.files.insert(in_skills_folder.to_path_buf());

        let mut parts = in_skills_folder.components();
        let Some(folder) = parts.next() else {
            continue;
        };
        let in_skill: PathBuf = parts.as_path().to_path_buf();
        if in_skill.as_os_str().is_empty() {
            continue;
        }
        let folder = folder.as_os_str().to_string_lossy().into_owned();
        if in_skill == Path::new(SKILL_FILE) {
            known.names.insert(folder.clone());
        }
        files_by_folder.entry(folder).or_default().push(SkillFile {
            path: in_skill,
            text,
        });
    }

    let skills = files_by_folder
        .into_iter()
        .map(|(folder, files)| Skill { folder, files })
        .collect();
    (skills, known)
}

/// Each finding of one of `rules`, as `<skill>/<file>: <rule>: <message>`.
fn findings_of(skills: &[Skill], known: &KnownSkills, rules: &[Rule]) -> Vec<String> {
    skills
        .iter()
        .flat_map(|skill| {
            check_skill(skill, known)
                .into_iter()
                .filter(|finding| rules.contains(&finding.rule))
                .map(|finding| {
                    format!(
                        "{}/{}: {}: {}",
                        skill.folder,
                        finding.file.display(),
                        finding.rule.id(),
                        finding.message
                    )
                })
        })
        .collect()
}

// This test must never be deleted. A skill whose name does not match its
// folder, whose description is missing, or whose header or text breaks
// another rule is skipped or cut short by Claude Code, and nothing reports it.
// It also fails on an empty bundle, which would pass every other check.
#[test]
fn every_skill_has_a_name_that_is_its_folder_and_a_description() {
    let (skills, known) = bundle_skills();
    assert!(!skills.is_empty(), "the bundle must hold skills");

    let rules = [
        Rule::H1,
        Rule::H2,
        Rule::H3,
        Rule::H4,
        Rule::H5,
        Rule::H7,
        Rule::H8,
        Rule::B1,
        Rule::B2,
        Rule::B4,
    ];
    let findings = findings_of(&skills, &known, &rules);
    assert!(
        findings.is_empty(),
        "the bundled skills break these rules:\n{}",
        findings.join("\n")
    );
}

// This test must never be deleted. A space followed by `#` starts a comment in
// a plain YAML value, so `Inline #[cfg(test)]` cuts a description or a
// `when_to_use` down to its first words, and the skill stops loading when a
// person asks for it.
#[test]
fn a_plain_description_is_not_shortened_by_yaml() {
    let (skills, known) = bundle_skills();

    let findings = findings_of(&skills, &known, &[Rule::H6]);
    assert!(
        findings.is_empty(),
        "YAML cuts these header values short:\n{}",
        findings.join("\n")
    );
}

#[test]
fn every_markdown_link_to_a_file_in_the_bundle_resolves() {
    let (skills, known) = bundle_skills();

    let findings = findings_of(&skills, &known, &[Rule::B3]);
    assert!(
        findings.is_empty(),
        "these links point to no file in the bundle:\n{}",
        findings.join("\n")
    );
}
