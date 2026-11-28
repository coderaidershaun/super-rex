//! The arguments of a search as they arrive, and each request form as the lookups read it:
//! vocabulary terms, free-text needs, and the words that the vocabulary does not hold.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::vocabulary::{TermGroup, Vocabulary};

/// How many items one result holds when the caller gives no `limit`.
pub const DEFAULT_LIMIT: usize = 10;

/// The most free-text needs one form may hold.
pub const MAX_NEEDS: usize = 3;

/// The token budget of one result when the caller gives no `token_budget`:
/// this many tokens for each item of `limit`, so 2,000 for a page of 10.
pub const DEFAULT_TOKENS_PER_ITEM: usize = 200;

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
    /// The most tokens that one result may cost. `None` means
    /// [`DEFAULT_TOKENS_PER_ITEM`] for each item of `limit`.
    #[serde(default)]
    pub token_budget: Option<usize>,
    /// Keep the hash of every item and file, whatever the size. Items still
    /// move to the next page to keep the budget.
    #[serde(default)]
    pub keep_hashes: bool,
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

    /// The most tokens that one result may cost.
    #[must_use]
    pub fn token_budget(&self) -> usize {
        let default = || DEFAULT_TOKENS_PER_ITEM.saturating_mul(self.limit());
        self.token_budget.unwrap_or_else(default)
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

impl FormTerms {
    /// The form read against the embedded vocabulary. A word the vocabulary
    /// does not hold is listed under `unrecognised`, never refused.
    #[must_use]
    pub fn read(form: &RequestForm) -> FormTerms {
        let vocabulary = Vocabulary::embedded();
        let mut unrecognised = Vec::new();
        let facts = read_facts(vocabulary, &form.facts, &mut unrecognised);
        let mut terms: Vec<AskedTerm> = Vec::new();
        let needs = form.needs.iter().enumerate();
        let needs = needs.map(|(position, need)| text_query(Need::Text(position), need));
        let mut keyword_queries: Vec<TextQuery> = needs.collect();
        for (field, words) in term_fields(form) {
            for sent in words {
                let asked = asked_terms(vocabulary, field, sent);
                if asked.is_empty() {
                    let need = format!("{}:{sent}", field.request_name());
                    unrecognised.push(need.clone());
                    keyword_queries.push(text_query(Need::Word(need), sent));
                }
                for asked in asked {
                    let key = (asked.field, &asked.term);
                    let is_new = terms.iter().all(|known| (known.field, &known.term) != key);
                    if is_new {
                        terms.push(asked);
                    }
                }
            }
        }
        let quotes = form.evidence.iter().map(|evidence| &evidence.quote);
        let task_words = form.needs.iter().chain(quotes).flat_map(|text| words(text));
        FormTerms {
            id: form.id.clone(),
            facts,
            terms,
            needs: form.needs.clone(),
            task_words: task_words.collect(),
            unrecognised,
            keyword_queries,
        }
    }
}

fn text_query(need: Need, text: &str) -> TextQuery {
    TextQuery {
        need,
        text: text.to_owned(),
    }
}

fn term_fields(form: &RequestForm) -> [(TermField, &[String]); 4] {
    [
        (TermField::Quality, form.must_have.as_slice()),
        (TermField::Artefact, form.artefacts.as_slice()),
        (TermField::Activity, form.activity.as_slice()),
        (TermField::Touches, form.touches.as_slice()),
    ]
}

/// The term the word names in the group of `field`, or one term for each
/// alias row of that group whose word it is. Empty for a word of no term.
fn asked_terms(vocabulary: &Vocabulary, field: TermField, sent: &str) -> Vec<AskedTerm> {
    let word = normalised(sent);
    if vocabulary.has_term(field.group(), &word) {
        let from_alias = None;
        return vec![AskedTerm {
            field,
            term: word,
            from_alias,
        }];
    }
    let aliases = vocabulary.aliases_of(field.group(), &word).into_iter();
    aliases
        .map(|alias| AskedTerm {
            field,
            term: alias.term.clone(),
            from_alias: Some(sent.to_owned()),
        })
        .collect()
}

/// The word in lower case with single spaces between its parts, as the
/// vocabulary writes its terms and alias words.
fn normalised(word: &str) -> String {
    let parts: Vec<&str> = word.split_whitespace().collect();
    parts.join(" ").to_lowercase()
}

/// The lower-case runs of letters and digits of a text.
pub(super) fn words(text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    let runs = lower.split(|character: char| !character.is_alphanumeric());
    let runs = runs.filter(|word| !word.is_empty());
    runs.map(str::to_owned).collect()
}

fn read_facts(
    vocabulary: &Vocabulary,
    sent: &Map<String, Value>,
    unrecognised: &mut Vec<String>,
) -> Facts {
    let mut facts = Facts::default();
    for (name, value) in sent {
        let is_read = vocabulary.has_fact(name)
            && read_fact(&mut facts, name, value, vocabulary, unrecognised).is_some();
        if !is_read {
            unrecognised.push(format!("facts:{name}"));
        }
    }
    facts
}

/// Puts one sent fact into `facts`. `None` when the value has the wrong type
/// for the fact, or the fact is one that `Facts` cannot hold.
/// A closed fact keeps only the values the vocabulary allows, each once, with a
/// common full name such as `typescript` read as the value it names, and lists
/// each other value as `<fact>:<value>`; an open fact keeps its value as sent.
fn read_fact(
    facts: &mut Facts,
    name: &str,
    value: &Value,
    vocabulary: &Vocabulary,
    unrecognised: &mut Vec<String>,
) -> Option<()> {
    let mut allowed = |sent: &str| {
        let Some(allowed) = vocabulary.allowed_values(name) else {
            return Some(sent.to_owned());
        };
        let word = normalised(sent);
        let aliased = vocabulary.value_of_alias(name, &word).map(str::to_owned);
        let value = aliased.unwrap_or(word);
        let is_allowed = allowed.contains(&value.as_str());
        if !is_allowed {
            unrecognised.push(format!("{name}:{sent}"));
        }
        is_allowed.then_some(value)
    };
    match name {
        "writes_code" => facts.writes_code = Some(value.as_bool()?),
        "role" => facts.role = Some(value.as_str()?.to_owned()),
        "latency_scale" => facts.latency_scale = allowed(value.as_str()?),
        "languages" => {
            for language in text_list(value)? {
                let Some(language) = allowed(&language) else {
                    continue;
                };
                if !facts.languages.contains(&language) {
                    facts.languages.push(language);
                }
            }
        }
        "repo_has" => facts.repo_has = text_list(value)?,
        "repo_dependencies" => facts.repo_dependencies = text_list(value)?,
        _ => return None,
    }
    Some(())
}

fn text_list(value: &Value) -> Option<Vec<String>> {
    let items = value.as_array()?.iter();
    items.map(|item| item.as_str().map(str::to_owned)).collect()
}
