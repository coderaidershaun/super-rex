//! What one search knows about each served skill and each of its files. The
//! loader fills it from the library folders; the lookups only read it.

use serde::Serialize;

use super::request::Facts;

/// The skills of one or more library folders, for one search.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Index {
    /// One entry for each skill folder of each library, shadowed copies too,
    /// in library order and then name order. A lookup names an entry by its
    /// position here.
    pub skills: Vec<SkillEntry>,
}

/// The `version` that `head` reports for the same folders: `sha256:` and 64
/// lower-case hex digits. Every answer carries it, and a cursor is bound to it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct IndexVersion(pub String);

/// One skill folder of one library.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SkillEntry {
    pub name: String,
    /// The folder name of its library, or `super-rex` for the embedded one.
    pub library: String,
    /// 0 for the first `--library` folder, 1 for the next, and so on.
    pub library_priority: usize,
    /// The library of the earlier copy of the same name that hides this one.
    /// A shadowed entry takes part in lookups A and B only to be reported
    /// under `dropped`.
    pub shadowed_by: Option<String>,
    /// The header `description`, as one line.
    pub description: String,
    /// The header `when_to_use`, as one line.
    pub when_to_use: String,
    /// The `description` cut to at most 30 words: the one line a card shows.
    pub card: String,
    /// The SHA-256 of `SKILL.md`, in lower-case hex.
    pub sha256: String,
    /// The words of `SKILL.md` after its header, as `wc -w` counts them.
    pub words: usize,
    /// `None` when the header has no `metadata` map that a search can read.
    /// Such a skill is found by lookup C only.
    pub metadata: Option<SkillMetadata>,
    /// Every served file of the skill folder but `SKILL.md`, in path order.
    pub files: Vec<FileEntry>,
}

/// The `metadata` map of a skill header, in the form of section 7.1 of the
/// skill library specification. Its `files` items are on each [`FileEntry`].
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SkillMetadata {
    pub kind: Kind,
    pub category: String,
    pub claims: Claims,
    /// `full` when the header leaves it out.
    pub placement: Placement,
    pub scope: Scope,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_when: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub intents: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub open_when: Vec<OpenWhen>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub replaces: Vec<String>,
    /// `None` when the header leaves it out.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_topics: Option<usize>,
}

/// What the author says a skill is for. Only these claims select a skill in
/// lookups A and B.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Claims {
    /// Matched by lookup A. `latency_scale` is never claimed.
    pub facts: Facts,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub activity: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub touches: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub quality: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub artefact: Vec<String>,
}

/// The conditions of lookup B. An empty list means the key is not in the
/// scope.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Scope {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub writes_code: Vec<bool>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub languages: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub latency_scale: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Rule,
    Method,
    Tool,
    Reference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum Placement {
    #[serde(rename = "full")]
    Full,
    #[serde(rename = "open-when")]
    OpenWhen,
}

/// One item of `open_when`: another skill, a file of this skill, or a file of
/// another skill, and the case in which to open it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OpenWhen {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    pub when: String,
}

/// One served file of a skill folder, other than `SKILL.md`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileEntry {
    /// The path inside the skill folder, with `/` between folders.
    pub path: String,
    pub sha256: String,
    /// `None` when `metadata.files` does not list the file.
    pub role: Option<FileRole>,
    pub language: Option<String>,
    /// The router case of a topic file.
    pub when: Option<String>,
    pub intents: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FileRole {
    Topic,
    Language,
    Code,
}

impl Index {
    /// The entry at `position` that no earlier library hides and no served
    /// skill replaces: one a search may return as an item.
    #[must_use]
    pub fn is_served(&self, position: usize) -> bool {
        self.skills[position].shadowed_by.is_none() && self.replaced_by(position).is_none()
    }

    /// The served entry whose `replaces` names the entry at `position`.
    #[must_use]
    pub fn replaced_by(&self, position: usize) -> Option<&SkillEntry> {
        let name = &self.skills[position].name;
        self.skills.iter().find(|replacer| {
            replacer.shadowed_by.is_none()
                && replacer
                    .metadata
                    .as_ref()
                    .is_some_and(|metadata| metadata.replaces.contains(name))
        })
    }
}

impl SkillEntry {
    /// An entry with a name and a library and nothing else, to fill by field.
    #[must_use]
    pub fn new(name: &str, library: &str) -> SkillEntry {
        SkillEntry {
            name: name.to_owned(),
            library: library.to_owned(),
            ..SkillEntry::default()
        }
    }
}

impl SkillMetadata {
    /// A map with a kind and a category and nothing else, to fill by field.
    #[must_use]
    pub fn new(kind: Kind, category: &str) -> SkillMetadata {
        SkillMetadata {
            kind,
            category: category.to_owned(),
            claims: Claims::default(),
            placement: Placement::Full,
            scope: Scope::default(),
            not_when: None,
            intents: Vec::new(),
            open_when: Vec::new(),
            requires: Vec::new(),
            replaces: Vec::new(),
            max_topics: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index() -> Index {
        let mut new = SkillEntry::new("diagrams", "main");
        let mut metadata = SkillMetadata::new(Kind::Tool, "diagrams");
        metadata.replaces = vec!["old-diagrams".to_owned()];
        new.metadata = Some(metadata);
        let mut hidden = SkillEntry::new("rules", "other");
        hidden.shadowed_by = Some("main".to_owned());
        Index {
            skills: vec![
                new,
                SkillEntry::new("old-diagrams", "main"),
                SkillEntry::new("rules", "main"),
                hidden,
            ],
        }
    }

    #[test]
    fn a_replaced_or_shadowed_entry_is_not_served() {
        let index = index();

        let served: Vec<bool> = (0..4).map(|position| index.is_served(position)).collect();

        assert_eq!(served, [true, false, true, false]);
        assert_eq!(
            index.replaced_by(1).map(|entry| entry.name.as_str()),
            Some("diagrams")
        );
        assert!(index.replaced_by(2).is_none());
    }

    #[test]
    fn a_shadowed_copy_replaces_nothing() {
        let mut index = index();
        index.skills[0].shadowed_by = Some("first".to_owned());

        assert!(index.replaced_by(1).is_none());
        assert!(index.is_served(1));
    }
}
