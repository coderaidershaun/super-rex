//! The rules for the `metadata` map of a skill header, which a skill library
//! fills so that a search can find the skill (rules M1 to M13).

mod fields;
mod files;
mod terms;
mod vocabulary;

use std::path::PathBuf;

use yaml_rust2::Yaml;
use yaml_rust2::yaml::Hash;

use crate::commands::skills::{KnownSkills, Rule};

use super::header::Header;

/// The skill that the rules read the map of.
pub(super) struct SkillFolder<'a> {
    /// Every file of the skill folder, relative to it, `SKILL.md` too.
    pub(super) files: Vec<PathBuf>,
    pub(super) known: &'a KnownSkills,
}

/// The findings of rules M1 to M13 for the map of `header`. A header with no
/// map, or a map with no key that a search reads, gives none.
pub(super) fn findings(header: &Header, skill: &SkillFolder) -> Vec<(Rule, String)> {
    let Some(metadata) = header.metadata() else {
        return Vec::new();
    };
    let mut findings = Vec::new();
    findings.extend(terms::findings(&metadata));
    findings.extend(fields::findings(&metadata));
    findings.extend(files::findings(&metadata, skill));
    findings
}

fn get<'a>(map: &'a Hash, key: &str) -> Option<&'a Yaml> {
    map.get(&Yaml::String(key.to_owned()))
}

/// A YAML value for a message: a scalar in backticks, or the name of its type.
fn shown(value: &Yaml) -> String {
    match value {
        Yaml::String(text) | Yaml::Real(text) => format!("`{text}`"),
        Yaml::Integer(number) => format!("`{number}`"),
        Yaml::Boolean(value) => format!("`{value}`"),
        Yaml::Array(_) => "a list".to_owned(),
        Yaml::Hash(_) => "a map".to_owned(),
        Yaml::Null | Yaml::Alias(_) | Yaml::BadValue => "nothing".to_owned(),
    }
}

/// The items of `value` when it is a list whose items are all text.
fn text_list(value: &Yaml) -> Option<Vec<&str>> {
    value.as_vec()?.iter().map(Yaml::as_str).collect()
}
