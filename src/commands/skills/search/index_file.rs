//! `rex skills index`: the static index of a library as one JSON document, with the fields of the
//! Agent Skills Discovery draft and, beside them, the fields a skill library search reads.

use std::path::PathBuf;

use serde::Serialize;

use super::index::{FileEntry, FileRole, Index, SkillEntry, SkillMetadata};
use super::response::SearchError;
use crate::commands::skills::serve::library::Libraries;
use crate::commands::skills::{SKILL_FILE, SkillsError};
use crate::commands::stdout::print_line_to_stdout;

/// The protocol that `head` of `rex skills serve` reports.
const PROTOCOL: &str = "skills/1";

/// The document that `rex skills index` prints.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct IndexFile {
    /// `skills/1`, as `head` reports it.
    pub protocol: &'static str,
    pub library: String,
    /// The `version` of the vocabulary beside the first folder, as `head`
    /// reports it.
    pub vocabulary: Option<String>,
    /// The served skills, in name order.
    pub skills: Vec<IndexFileSkill>,
}

/// One served skill of the document.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct IndexFileSkill {
    pub name: String,
    /// `skill-md`: the entry is one `SKILL.md`.
    #[serde(rename = "type")]
    pub entry_type: &'static str,
    pub description: String,
    /// The path of `SKILL.md` from the library folder.
    pub url: String,
    /// `sha256:` and the hex SHA-256 of `SKILL.md`.
    pub digest: String,
    pub library: String,
    pub when_to_use: String,
    pub card: String,
    pub words: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<SkillMetadata>,
    pub files: Vec<IndexFileFile>,
}

/// One file of a served skill, other than `SKILL.md`, with its `files` item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IndexFileFile {
    pub path: String,
    pub url: String,
    pub digest: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<FileRole>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub intents: Vec<String>,
}

impl IndexFile {
    /// The document of the served entries of `index`.
    #[must_use]
    pub fn of(index: &Index, library: &str, vocabulary: Option<&str>) -> IndexFile {
        let mut served: Vec<&SkillEntry> = index
            .skills
            .iter()
            .filter(|entry| entry.shadowed_by.is_none())
            .collect();
        served.sort_by(|left, right| left.name.cmp(&right.name));
        IndexFile {
            protocol: PROTOCOL,
            library: library.to_owned(),
            vocabulary: vocabulary.map(str::to_owned),
            skills: served.into_iter().map(IndexFileSkill::of).collect(),
        }
    }
}

impl IndexFileSkill {
    fn of(entry: &SkillEntry) -> IndexFileSkill {
        IndexFileSkill {
            name: entry.name.clone(),
            entry_type: "skill-md",
            description: entry.description.clone(),
            url: format!("{}/{SKILL_FILE}", entry.name),
            digest: format!("sha256:{}", entry.sha256),
            library: entry.library.clone(),
            when_to_use: entry.when_to_use.clone(),
            card: entry.card.clone(),
            words: entry.words,
            metadata: entry.metadata.clone(),
            files: entry
                .files
                .iter()
                .map(|file| IndexFileFile::of(&entry.name, file))
                .collect(),
        }
    }
}

impl IndexFileFile {
    fn of(skill: &str, file: &FileEntry) -> IndexFileFile {
        IndexFileFile {
            path: file.path.clone(),
            url: format!("{skill}/{}", file.path),
            digest: format!("sha256:{}", file.sha256),
            role: file.role,
            language: file.language.clone(),
            when: file.when.clone(),
            intents: file.intents.clone(),
        }
    }
}

/// Print the index of the library in `libraries` on stdout.
pub(crate) fn run(folders: &[PathBuf]) -> Result<(), SkillsError> {
    let libraries = Libraries::open(None, folders).map_err(SearchError::from)?;
    let index = Index::load(&libraries)?;
    let vocabulary = libraries.vocabulary_version().map_err(SearchError::from)?;
    let document = IndexFile::of(&index, libraries.name(), vocabulary.as_deref());
    let text =
        serde_json::to_string_pretty(&document).map_err(|source| SkillsError::Json { source })?;
    print_line_to_stdout(&text)?;
    Ok(())
}
