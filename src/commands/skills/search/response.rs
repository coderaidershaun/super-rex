//! What search gives back: the candidates of each lookup, the answer built
//! from them, and the errors a call can answer with.

use std::fmt;
use std::io;
use std::path::PathBuf;

use serde::{Serialize, Serializer};

use super::index::{FileRole, IndexVersion, Kind, OpenWhen, Placement};
use super::request::Need;
use crate::commands::skills::serve::ServeError;
use crate::error::FailureClass;

/// The whole answer of one search call.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SearchResponse {
    pub index_version: IndexVersion,
    /// One result for each request form, in the order sent.
    pub results: Vec<RequestResult>,
}

/// The answer for one request form.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RequestResult {
    pub request: String,
    pub items: Vec<Item>,
    /// How many relevant skills come after this page.
    pub more: usize,
    /// Continues this result. `None` when `more` is 0.
    pub cursor: Option<String>,
    pub gaps: Vec<Gap>,
    pub unrecognised: Vec<String>,
    pub dropped: Vec<Dropped>,
}

/// The card of one found skill. Never the skill's text. The fields after
/// `confirm` are the optional ones that the size limit may leave out.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Item {
    pub skill: String,
    pub library: String,
    /// `None` for a skill with no `metadata`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<Kind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    pub slot: Slot,
    pub tier: Tier,
    pub why: String,
    pub card: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_when: Option<String>,
    /// A yes or no question. Every tier C item has one, and so does a tier B
    /// item whose scope is unknown and a skill that such an item requires.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirm: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placement: Option<Placement>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub words: Option<usize>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<FileItem>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub open_when: Vec<OpenWhen>,
}

/// One file of a found skill that matches the request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FileItem {
    pub path: String,
    pub role: FileRole,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
}

/// A need that nothing matched, or a skill that only its scope or the
/// task's language left out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Gap {
    pub need: String,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub library: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_when: Option<String>,
}

/// A found skill that another skill replaces or shadows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Dropped {
    pub skill: String,
    pub library: String,
    pub reason: String,
}

/// The need an item fills, as a label: `rule`, `method`, `tool`,
/// `reference`, `keyword`, `quality:<term>` or `artefact:<term>`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Slot {
    Kind(Kind),
    /// A tier C item of a skill with no `metadata`, so with no kind.
    Keyword,
    Quality(String),
    Artefact(String),
}

/// Which lookup found an item: A rules, B declared matches, C keyword search.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub enum Tier {
    A,
    B,
    C,
}

/// One skill that one lookup found for one form, before the answer orders,
/// filters and pages it.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    /// The position of the skill in `Index::skills`.
    pub skill: usize,
    pub tier: Tier,
    /// The needs it fills, the one that gives its slot first. Never empty.
    pub needs: Vec<Need>,
    /// `<head>` or `<head>; <detail>`. A shortened `why` keeps the head.
    pub why: String,
    /// What the caller must answer yes to before it uses the skill. The
    /// answer writes them as the item's `confirm`, and asks each one once.
    pub questions: Vec<Question>,
    /// Lookup B: true when every term it matched came through an alias.
    pub through_alias: bool,
    /// Lookup C: minus the bm25 score, so higher is better. `None` for the
    /// other lookups, which never rank by corpus statistics.
    pub relevance: Option<f64>,
}

/// One yes or no question of a candidate.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Question {
    /// The form leaves out a fact that the skill's scope names: does the
    /// task have one of these values?
    Scope { fact: String, values: Vec<String> },
    /// Lookup C proposed the skill for one need: is it right for that need?
    Keyword { skill: String, need: Need },
}

/// What one lookup gives.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Found {
    pub candidates: Vec<Candidate>,
    pub gaps: Vec<Gap>,
}

/// The files of one found skill: those that match, and the topic files to
/// list as open-when items when none matches.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileChoice {
    pub files: Vec<FileItem>,
    pub open_when: Vec<OpenWhen>,
}

/// Failure of a search, an index, or the loading of a library. The `search`
/// tool answers it as `{"code": ..., "message": ...}` with `isError` true.
#[derive(thiserror::Error, Debug)]
pub enum SearchError {
    #[error("the search arguments are not valid: {message}")]
    BadArguments { message: String },

    #[error("the cursor cannot be read: {reason}; search again without it")]
    BadCursor { reason: String },

    #[error("the library changed since the cursor was made; search again without it")]
    StaleCursor,

    #[error("cannot read the library folder {}", path.display())]
    ReadFolder {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("cannot read {skill}/{file}: {reason}")]
    UnreadableFile {
        skill: String,
        file: String,
        reason: String,
    },

    #[error(transparent)]
    Library(#[from] ServeError),

    #[error("the keyword search failed")]
    KeywordSearch {
        #[source]
        source: rusqlite::Error,
    },

    #[error("cannot read the search arguments on stdin")]
    Stdin(#[source] io::Error),
}

impl SearchError {
    /// The `code` of the tool error.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            SearchError::BadArguments { .. } | SearchError::Stdin(_) => "bad_arguments",
            SearchError::BadCursor { .. } => "bad_cursor",
            SearchError::StaleCursor => "stale_cursor",
            SearchError::ReadFolder { .. }
            | SearchError::UnreadableFile { .. }
            | SearchError::Library(_) => "unreadable",
            SearchError::KeywordSearch { .. } => "search_failed",
        }
    }

    /// What the caller does next. A folder that cannot be listed may come
    /// back; every other failure is about this one call.
    #[must_use]
    pub fn failure_class(&self) -> FailureClass {
        match self {
            SearchError::ReadFolder { .. } => FailureClass::Retry,
            SearchError::BadArguments { .. }
            | SearchError::BadCursor { .. }
            | SearchError::StaleCursor
            | SearchError::UnreadableFile { .. }
            | SearchError::Library(_)
            | SearchError::KeywordSearch { .. }
            | SearchError::Stdin(_) => FailureClass::Reject,
        }
    }
}

impl fmt::Display for Slot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Slot::Kind(Kind::Rule) => formatter.write_str("rule"),
            Slot::Kind(Kind::Method) => formatter.write_str("method"),
            Slot::Kind(Kind::Tool) => formatter.write_str("tool"),
            Slot::Kind(Kind::Reference) => formatter.write_str("reference"),
            Slot::Keyword => formatter.write_str("keyword"),
            Slot::Quality(term) => write!(formatter, "quality:{term}"),
            Slot::Artefact(term) => write!(formatter, "artefact:{term}"),
        }
    }
}

impl Serialize for Slot {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}
