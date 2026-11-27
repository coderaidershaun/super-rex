//! The shared vocabulary of skill headers, embedded in the binary, so that
//! `rex skills check` knows every term a `metadata` map may use.

use std::sync::LazyLock;

use serde::Deserialize;
use serde_json::Value;
use yaml_rust2::Yaml;

const TERMS_JSON: &str = include_str!("../../../../plugin/vocab/terms.json");

/// The one value pattern the vocabulary uses today: a role id.
const ID_PATTERN: &str = "^[a-z0-9][a-z0-9-]*$";

static VOCABULARY: LazyLock<Vocabulary> = LazyLock::new(|| {
    let vocabulary: Vocabulary = serde_json::from_str(TERMS_JSON)
        .expect("the embedded plugin/vocab/terms.json must have the form its contract gives");
    for fact in &vocabulary.facts.terms {
        if let Some(pattern) = &fact.pattern {
            assert_eq!(
                pattern, ID_PATTERN,
                "rex skills check cannot check the pattern of the fact `{}`",
                fact.term
            );
        }
    }
    vocabulary
});

#[derive(Deserialize)]
pub(super) struct Vocabulary {
    pub(super) kinds: Group,
    pub(super) categories: Group,
    facts: Facts,
    scope_keys: ScopeKeys,
    activities: Group,
    touches: Group,
    qualities: Group,
    artefacts: Group,
}

#[derive(Deserialize)]
pub(super) struct Group {
    /// The header field that takes the terms, such as `claims.activity`.
    header: String,
    terms: Vec<Term>,
}

#[derive(Deserialize)]
struct Term {
    term: String,
}

#[derive(Deserialize)]
struct Facts {
    terms: Vec<Fact>,
}

#[derive(Deserialize)]
pub(super) struct Fact {
    pub(super) term: String,
    #[serde(rename = "type")]
    pub(super) value_type: FactType,
    /// The allowed values of a closed fact; `None` for an open fact.
    allowed: Option<Vec<Value>>,
    /// The pattern each value of an open fact matches; `None` when its values
    /// are not checked.
    pattern: Option<String>,
    pub(super) claimable: bool,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(super) enum FactType {
    Boolean,
    String,
    List,
}

#[derive(Deserialize)]
struct ScopeKeys {
    terms: Vec<ScopeKey>,
}

#[derive(Deserialize)]
struct ScopeKey {
    term: String,
    fact: String,
}

impl Vocabulary {
    pub(super) fn embedded() -> &'static Vocabulary {
        &VOCABULARY
    }

    /// The group of terms that the claims field `claims.<field>` takes, such
    /// as `activities` for `activity`. `None` for `facts`, which takes facts.
    pub(super) fn claims_group(&self, field: &str) -> Option<&Group> {
        let header = format!("claims.{field}");
        [
            &self.activities,
            &self.touches,
            &self.qualities,
            &self.artefacts,
        ]
        .into_iter()
        .find(|group| group.header == header)
    }

    pub(super) fn fact(&self, term: &str) -> Option<&Fact> {
        self.facts.terms.iter().find(|fact| fact.term == term)
    }

    /// The fact that the scope key `term` is checked against.
    pub(super) fn scope_fact(&self, term: &str) -> Option<&Fact> {
        let key = self.scope_keys.terms.iter().find(|key| key.term == term)?;
        self.fact(&key.fact)
    }

    pub(super) fn scope_key_list(&self) -> String {
        let terms: Vec<&str> = self
            .scope_keys
            .terms
            .iter()
            .map(|key| key.term.as_str())
            .collect();
        terms.join(", ")
    }
}

impl Group {
    pub(super) fn has(&self, term: &str) -> bool {
        self.terms.iter().any(|known| known.term == term)
    }

    /// The group's terms, joined by `, `, for a message.
    pub(super) fn list(&self) -> String {
        let terms: Vec<&str> = self.terms.iter().map(|known| known.term.as_str()).collect();
        terms.join(", ")
    }
}

impl Fact {
    /// What is wrong with one value of this fact, as the words that follow
    /// "holds" in a message, or `None` when the value is allowed.
    pub(super) fn value_problem(&self, value: &Yaml) -> Option<String> {
        let shown = super::shown(value);
        let is_boolean = self.takes_booleans();
        let text = match value {
            Yaml::Boolean(_) if is_boolean => None,
            Yaml::String(text) if !is_boolean => Some(text),
            _ => return Some(format!("{shown}, which is not {}", self.item_type())),
        };
        if let Some(allowed) = &self.allowed {
            let as_json = match text {
                Some(text) => Value::String(text.clone()),
                None => Value::Bool(value.as_bool().unwrap_or_default()),
            };
            return (!allowed.contains(&as_json))
                .then(|| format!("{shown}, which is not one of {}", self.allowed_list()));
        }
        let breaks_pattern = self.pattern.is_some() && text.is_some_and(|text| !is_id(text));
        breaks_pattern.then(|| format!("{shown}, which does not match {ID_PATTERN}"))
    }

    /// The YAML type of one value of this fact, for a message.
    pub(super) fn item_type(&self) -> &'static str {
        if self.takes_booleans() {
            "true or false"
        } else {
            "text"
        }
    }

    fn takes_booleans(&self) -> bool {
        self.value_type == FactType::Boolean
    }

    fn allowed_list(&self) -> String {
        let values: Vec<String> = self
            .allowed
            .iter()
            .flatten()
            .map(|value| match value {
                Value::String(text) => text.clone(),
                other => other.to_string(),
            })
            .collect();
        values.join(", ")
    }
}

/// True when `value` matches [`ID_PATTERN`].
fn is_id(value: &str) -> bool {
    let mut bytes = value.bytes();
    let starts_well = bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit());
    starts_well
        && bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

/// The top-level `version` of the embedded `terms.json`, which a library
/// server reports as the vocabulary of the library embedded in the binary.
pub(crate) fn embedded_version() -> String {
    let terms: Value = serde_json::from_str(TERMS_JSON)
        .expect("the embedded plugin/vocab/terms.json must be JSON");
    terms
        .get("version")
        .and_then(Value::as_str)
        .expect("the embedded plugin/vocab/terms.json must have a top-level string \"version\"")
        .to_owned()
}
