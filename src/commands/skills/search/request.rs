//! The arguments of a search as they arrive, and each request form as the lookups read it:
//! vocabulary terms, free-text needs, and the words that the vocabulary does not hold.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::vocabulary::TermGroup;

/// How many items one result holds when the caller gives no `limit`.
pub const DEFAULT_LIMIT: usize = 10;

/// The most free-text needs one form may hold.
pub const MAX_NEEDS: usize = 3;

/// The arguments of the `search` tool, and the JSON that `rex skills search`
/// reads on stdin.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchRequest {
    pub requests: Vec<RequestForm>,
    /// Applies to each request. `None` means [`DEFAULT_LIMIT`].
    #[serde(default)]
    pub limit: Option<usize>,
    /// Continues an earlier answer for one request. `requests` must then
    /// hold exactly that one request, as it was sent.
    #[serde(default)]
    pub cursor: Option<String>,
    #[serde(default)]
    pub kinds: Option<Vec<String>>,
    #[serde(default)]
    pub categories: Option<Vec<String>>,
}

/// One request form: what one task or role asks of the library.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestForm {
    pub id: String,
    /// Fact names of the vocabulary and their values. Kept as sent, so that
    /// a name the vocabulary does not hold can be reported, not refused.
    #[serde(default)]
    pub facts: Map<String, Value>,
    #[serde(default)]
    pub activity: Option<String>,
    #[serde(default)]
    pub touches: Vec<String>,
    #[serde(default)]
    pub must_have: Vec<String>,
    #[serde(default)]
    pub artefacts: Vec<String>,
    /// Free text, at most [`MAX_NEEDS`] items.
    #[serde(default)]
    pub needs: Vec<String>,
    #[serde(default)]
    pub evidence: Vec<Evidence>,
}

/// A quote from the task that justifies one tick of the form.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub term: String,
    pub quote: String,
}

/// The facts of the vocabulary. A request states them; a skill claims all
/// but `latency_scale`, which only a scope reads.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Facts {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub writes_code: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub languages: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub repo_has: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub repo_dependencies: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latency_scale: Option<String>,
}

/// One request form read against the vocabulary. Lookups take this, never
/// the form as sent.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FormTerms {
    pub id: String,
    /// Only the facts the vocabulary holds, with values it allows.
    pub facts: Facts,
    /// The terms of `must_have`, `artefacts`, `activity` and `touches`, in
    /// that order and then in the order sent.
    pub terms: Vec<AskedTerm>,
    pub needs: Vec<String>,
    /// The lower-case words of `needs` and of each evidence quote, in order:
    /// the task's own words, which name a tool skill.
    pub task_words: Vec<String>,
    /// `<field>:<word>` for each word the vocabulary does not hold, in the
    /// order of the form.
    pub unrecognised: Vec<String>,
    /// One keyword query for each need and each unrecognised word of
    /// `must_have`, `artefacts`, `activity` and `touches`, in that order.
    pub keyword_queries: Vec<TextQuery>,
}

/// One vocabulary term that a form asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AskedTerm {
    pub field: TermField,
    pub term: String,
    /// The word of the form that the alias table mapped onto `term`, or
    /// `None` when the form named the term itself.
    pub from_alias: Option<String>,
}

/// A field of a form that takes the terms that lookup B matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TermField {
    Quality,
    Artefact,
    Activity,
    Touches,
}

/// One need of a form that a found skill fills. The first need of a
/// candidate gives its slot.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Need {
    /// Lookup A: the claimed facts that hold, joined by `+`, such as
    /// `writes_code+languages`.
    Fact(String),
    /// Lookup B: a term the form asks for.
    Term { field: TermField, term: String },
    /// Lookup C: the need at this position of `needs`.
    Text(usize),
    /// Lookup C: an unrecognised word, as `<field>:<word>`.
    Word(String),
    /// A skill that the named skill requires.
    Required(String),
}

/// The text of one keyword query and the need it serves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextQuery {
    pub need: Need,
    pub text: String,
}

impl SearchRequest {
    /// The number of items one result may hold.
    #[must_use]
    pub fn limit(&self) -> usize {
        self.limit.unwrap_or(DEFAULT_LIMIT)
    }
}

impl TermField {
    /// The field of a request form that takes this field's terms.
    #[must_use]
    pub fn request_name(self) -> &'static str {
        match self {
            TermField::Quality => "must_have",
            TermField::Artefact => "artefacts",
            TermField::Activity => "activity",
            TermField::Touches => "touches",
        }
    }

    /// The field of `metadata.claims` that takes this field's terms.
    #[must_use]
    pub fn claim_name(self) -> &'static str {
        match self {
            TermField::Quality => "quality",
            TermField::Artefact => "artefact",
            TermField::Activity => "activity",
            TermField::Touches => "touches",
        }
    }

    /// The group of the vocabulary whose terms this field takes.
    #[must_use]
    pub fn group(self) -> TermGroup {
        match self {
            TermField::Quality => TermGroup::Qualities,
            TermField::Artefact => TermGroup::Artefacts,
            TermField::Activity => TermGroup::Activities,
            TermField::Touches => TermGroup::Touches,
        }
    }
}
