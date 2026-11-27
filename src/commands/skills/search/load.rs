//! Builds the index from the served library folders: one entry for each
//! skill and one for each of its files, read from the skill's header.

use std::path::PathBuf;

use yaml_rust2::Yaml;
use yaml_rust2::yaml::Hash;

use super::index::{
    Claims, FileEntry, FileRole, Index, Kind, OpenWhen, Placement, Scope, SkillEntry, SkillMetadata,
};
use super::request::Facts;
use super::response::SearchError;
use crate::commands::skills::SKILL_FILE;
use crate::commands::skills::header::{Header, Metadata, Parts, one_line};
use crate::commands::skills::serve::library::{Libraries, Library, ReadError};

/// The most words of the one line a card shows.
pub const CARD_WORDS: usize = 30;

/// One skill folder to read into an index entry.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkillSource {
    pub name: String,
    pub library: String,
    pub library_priority: usize,
    /// The library of the earlier copy of the same name, when one hides it.
    pub shadowed_by: Option<String>,
    /// The whole text of `SKILL.md`.
    pub skill_md: String,
    /// Every served file of the folder, `SKILL.md` too, as `(path, sha256)`.
    pub files: Vec<(String, String)>,
}

impl Index {
    /// The index of the skills in `folders`, or of the embedded library when
    /// there are none.
    ///
    /// # Errors
    /// [`SearchError::Library`] for a folder that is missing, and
    /// [`SearchError::ReadFolder`] or [`SearchError::UnreadableFile`] when a
    /// library cannot be read.
    pub fn from_folders(folders: &[PathBuf]) -> Result<Index, SearchError> {
        let libraries = Libraries::open(None, folders)?;
        Index::load(&libraries)
    }

    pub(crate) fn load(libraries: &Libraries) -> Result<Index, SearchError> {
        // SMELL: `skill_places` does not say which library folder it could
        // not list, so the error names the first one. It needs to carry the
        // folder in its error.
        let places = libraries
            .skill_places()
            .map_err(|source| SearchError::ReadFolder {
                path: folder_of(libraries.library(0), ""),
                source,
            })?;
        let mut skills = Vec::with_capacity(places.len());
        for place in places {
            let library = libraries.library(place.library);
            let skill_md = library
                .read(&place.skill, SKILL_FILE)
                .map_err(|error| unreadable(&place.skill, error))?;
            let files = library
                .files(&place.skill)
                .map_err(|source| SearchError::ReadFolder {
                    path: folder_of(library, &place.skill),
                    source,
                })?;
            skills.push(SkillEntry::read(SkillSource {
                name: place.skill,
                library: library.name(),
                library_priority: place.library,
                shadowed_by: place
                    .shadowed_by
                    .map(|position| libraries.library(position).name()),
                skill_md: String::from_utf8_lossy(&skill_md).into_owned(),
                files,
            }));
        }
        Ok(Index { skills })
    }
}

fn folder_of(library: &Library, skill: &str) -> PathBuf {
    match library {
        Library::Folder(root) => root.join(skill),
        Library::Embedded => PathBuf::from(library.name()).join(skill),
    }
}

fn unreadable(skill: &str, error: ReadError) -> SearchError {
    let reason = match error {
        ReadError::NotFound => "the file is not there".to_owned(),
        ReadError::Unreadable(message) => {
            let prefix = format!("cannot read {skill}/{SKILL_FILE}: ");
            message
                .strip_prefix(&prefix)
                .map_or_else(|| message.clone(), str::to_owned)
        }
    };
    SearchError::UnreadableFile {
        skill: skill.to_owned(),
        file: SKILL_FILE.to_owned(),
        reason,
    }
}

impl SkillEntry {
    /// The entry of one skill. A header that cannot be read gives an entry
    /// with no `metadata`, never an error.
    #[must_use]
    pub fn read(source: SkillSource) -> SkillEntry {
        let header = Header::of_skill_file(&source.skill_md);
        let text = |key: &str| {
            header
                .as_ref()
                .and_then(|header| header.text(key))
                .unwrap_or_default()
        };
        let description = text("description");
        let when_to_use = text("when_to_use");
        let raw = header.as_ref().and_then(Header::metadata);
        let metadata = raw.as_ref().and_then(read_metadata);
        let listed = match (&metadata, &raw) {
            (Some(_), Some(raw)) => listed_files(raw),
            _ => Vec::new(),
        };
        let sha256 = source
            .files
            .iter()
            .find(|(path, _)| path == SKILL_FILE)
            .map(|(_, sha256)| sha256.clone())
            .unwrap_or_default();
        let mut files: Vec<FileEntry> = source
            .files
            .into_iter()
            .filter(|(path, _)| path != SKILL_FILE)
            .map(|(path, sha256)| file_entry(path, sha256, &listed))
            .collect();
        files.sort_by(|left, right| left.path.cmp(&right.path));
        SkillEntry {
            name: source.name,
            library: source.library,
            library_priority: source.library_priority,
            shadowed_by: source.shadowed_by,
            card: card_of(&description),
            description,
            when_to_use,
            sha256,
            words: body_words(&source.skill_md),
            metadata,
            files,
        }
    }
}

fn card_of(description: &str) -> String {
    let words: Vec<&str> = description.split_whitespace().take(CARD_WORDS).collect();
    words.join(" ")
}

fn body_words(skill_md: &str) -> usize {
    match Parts::split(skill_md) {
        Some(parts) => parts.body.split_whitespace().count(),
        None => skill_md.split_whitespace().count(),
    }
}

fn file_entry(path: String, sha256: String, listed: &[FileEntry]) -> FileEntry {
    let item = listed.iter().find(|item| item.path == path);
    match item {
        Some(item) => FileEntry {
            sha256,
            ..item.clone()
        },
        None => FileEntry {
            path,
            sha256,
            ..FileEntry::default()
        },
    }
}

/// The search map of a header, or `None` when `kind`, `category`, `claims`
/// or `scope` breaks the form: a wrong claim must never select or exclude a
/// skill, so such a skill is found by keyword only.
fn read_metadata(raw: &Metadata) -> Option<SkillMetadata> {
    let kind = match raw.get("kind")?.as_str()? {
        "rule" => Kind::Rule,
        "method" => Kind::Method,
        "tool" => Kind::Tool,
        "reference" => Kind::Reference,
        _ => return None,
    };
    let category = raw.get("category")?.as_str()?;
    let mut metadata = SkillMetadata::new(kind, category);
    if let Some(claims) = raw.get("claims") {
        metadata.claims = read_claims(claims)?;
    }
    if let Some(scope) = raw.get("scope") {
        metadata.scope = read_scope(scope)?;
    }
    if raw.get("placement").and_then(Yaml::as_str) == Some("open-when") {
        metadata.placement = Placement::OpenWhen;
    }
    metadata.not_when = raw.get("not_when").and_then(Yaml::as_str).map(one_line);
    metadata.intents = optional_list(raw.get("intents"));
    metadata.open_when = raw
        .get("open_when")
        .and_then(Yaml::as_vec)
        .map(|items| items.iter().filter_map(open_when_item).collect())
        .unwrap_or_default();
    metadata.requires = optional_list(raw.get("requires"));
    metadata.replaces = optional_list(raw.get("replaces"));
    metadata.max_topics = raw
        .get("max_topics")
        .and_then(Yaml::as_i64)
        .and_then(|count| usize::try_from(count).ok())
        .filter(|count| *count > 0);
    Some(metadata)
}

fn read_claims(value: &Yaml) -> Option<Claims> {
    let mut claims = Claims::default();
    for (key, value) in value.as_hash()? {
        match key.as_str()? {
            "facts" => claims.facts = read_facts(value)?,
            "activity" => claims.activity = text_list(value)?,
            "touches" => claims.touches = text_list(value)?,
            "quality" => claims.quality = text_list(value)?,
            "artefact" => claims.artefact = text_list(value)?,
            _ => return None,
        }
    }
    Some(claims)
}

fn read_facts(value: &Yaml) -> Option<Facts> {
    let mut facts = Facts::default();
    for (key, value) in value.as_hash()? {
        match key.as_str()? {
            "writes_code" => facts.writes_code = Some(value.as_bool()?),
            "languages" => facts.languages = text_list(value)?,
            "role" => facts.role = Some(value.as_str()?.to_owned()),
            "repo_has" => facts.repo_has = text_list(value)?,
            "repo_dependencies" => facts.repo_dependencies = text_list(value)?,
            _ => return None,
        }
    }
    Some(facts)
}

fn read_scope(value: &Yaml) -> Option<Scope> {
    let mut scope = Scope::default();
    for (key, value) in value.as_hash()? {
        match key.as_str()? {
            "writes_code" => {
                scope.writes_code = value
                    .as_vec()?
                    .iter()
                    .map(Yaml::as_bool)
                    .collect::<Option<_>>()?;
            }
            "languages" => scope.languages = text_list(value)?,
            "latency_scale" => scope.latency_scale = text_list(value)?,
            _ => return None,
        }
    }
    Some(scope)
}

/// The strings of a YAML list, or `None` when it is not a list of strings.
fn text_list(value: &Yaml) -> Option<Vec<String>> {
    value
        .as_vec()?
        .iter()
        .map(|item| item.as_str().map(str::to_owned))
        .collect()
}

/// A list field that is read on its own: a broken one is empty.
fn optional_list(value: Option<&Yaml>) -> Vec<String> {
    value.and_then(text_list).unwrap_or_default()
}

fn field<'a>(map: &'a Hash, key: &str) -> Option<&'a Yaml> {
    map.get(&Yaml::String(key.to_owned()))
}

/// An optional text field of an item: `Some(None)` when it is absent, and
/// `None` when it is not text, which breaks the item.
fn optional_text(map: &Hash, key: &str) -> Option<Option<String>> {
    match field(map, key) {
        None => Some(None),
        Some(value) => value.as_str().map(|text| Some(text.to_owned())),
    }
}

fn open_when_item(item: &Yaml) -> Option<OpenWhen> {
    let map = item.as_hash()?;
    let when = field(map, "when")?.as_str()?.to_owned();
    let skill = optional_text(map, "skill")?;
    let file = optional_text(map, "file")?;
    if skill.is_none() && file.is_none() {
        return None;
    }
    Some(OpenWhen { skill, file, when })
}

/// The `files` items of the map, each as a file entry with no hash yet. An
/// item with no `path` text is left out; its other fields are read one by one.
fn listed_files(raw: &Metadata) -> Vec<FileEntry> {
    let Some(items) = raw.get("files").and_then(Yaml::as_vec) else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            let map = item.as_hash()?;
            let text = |key: &str| field(map, key).and_then(Yaml::as_str).map(str::to_owned);
            let role = match field(map, "role").and_then(Yaml::as_str) {
                Some("topic") => Some(FileRole::Topic),
                Some("language") => Some(FileRole::Language),
                Some("code") => Some(FileRole::Code),
                _ => None,
            };
            Some(FileEntry {
                path: text("path")?,
                sha256: String::new(),
                role,
                language: text("language"),
                when: text("when"),
                intents: optional_list(field(map, "intents")),
            })
        })
        .collect()
}
