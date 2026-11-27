//! The shared vocabulary as a search request reads it, embedded in the binary: which words are
//! terms, which facts and values exist, and which free words the alias table maps onto a term.

use std::sync::LazyLock;

use serde::Deserialize;
use serde_json::Value;

const TERMS_JSON: &str = include_str!("../../../../plugin/vocab/terms.json");
const ALIASES_JSON: &str = include_str!("../../../../plugin/vocab/aliases.json");

// SMELL: `rex skills check` reads terms.json through a type of its own, which
// sees the header side of each group. This one sees the request side. Both
// must change together when the form of the file changes.
static VOCABULARY: LazyLock<Vocabulary> = LazyLock::new(|| {
    let terms: TermsFile = serde_json::from_str(TERMS_JSON)
        .expect("the embedded plugin/vocab/terms.json must have the form its contract gives");
    let aliases: AliasesFile = serde_json::from_str(ALIASES_JSON)
        .expect("the embedded plugin/vocab/aliases.json must have the form its contract gives");
    assert_eq!(
        aliases.vocabulary, terms.version,
        "the embedded alias table must be written for the embedded terms version"
    );
    Vocabulary {
        groups: vec![
            (TermGroup::Kinds, terms.kinds.names()),
            (TermGroup::Categories, terms.categories.names()),
            (TermGroup::Activities, terms.activities.names()),
            (TermGroup::Touches, terms.touches.names()),
            (TermGroup::Qualities, terms.qualities.names()),
            (TermGroup::Artefacts, terms.artefacts.names()),
        ],
        facts: terms.facts.terms,
        terms_version: terms.version,
        aliases_version: aliases.version,
        aliases: aliases.aliases,
        broader_terms: aliases.broader_terms,
    }
});

/// The terms of `plugin/vocab/terms.json` and the table of
/// `plugin/vocab/aliases.json`.
#[derive(Debug)]
pub struct Vocabulary {
    terms_version: String,
    aliases_version: String,
    groups: Vec<(TermGroup, Vec<String>)>,
    facts: Vec<Fact>,
    aliases: Vec<Alias>,
    broader_terms: Vec<BroaderTerm>,
}

/// A group of terms, named as both files name it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TermGroup {
    Kinds,
    Categories,
    Activities,
    Touches,
    Qualities,
    Artefacts,
}

/// What an alias word is to its term.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Relation {
    /// The word means the term.
    Synonym,
    /// The word is one case of the term.
    Narrower,
    /// The word covers the term and other terms.
    Broader,
}

/// One row of the alias table: a free word and the term it maps onto.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Alias {
    /// Lower case, with single spaces between its words.
    pub word: String,
    pub relation: Relation,
    pub group: TermGroup,
    pub term: String,
}

/// One term and the broader term it sits under, such as `latency` under the
/// category `performance`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct BroaderTerm {
    pub group: TermGroup,
    pub term: String,
    pub broader_group: TermGroup,
    pub broader_term: String,
}

impl Vocabulary {
    #[must_use]
    pub fn embedded() -> &'static Vocabulary {
        &VOCABULARY
    }

    /// The `version` of terms.json, which `head` reports as `vocabulary`.
    #[must_use]
    pub fn terms_version(&self) -> &str {
        &self.terms_version
    }

    /// The `version` of aliases.json. A change to the table changes keyword
    /// results, though `head` does not report it.
    #[must_use]
    pub fn aliases_version(&self) -> &str {
        &self.aliases_version
    }

    #[must_use]
    pub fn has_term(&self, group: TermGroup, term: &str) -> bool {
        self.groups
            .iter()
            .any(|(known, terms)| *known == group && terms.iter().any(|known| known == term))
    }

    /// True for a fact name of the vocabulary, such as `writes_code`.
    #[must_use]
    pub fn has_fact(&self, fact: &str) -> bool {
        self.facts.iter().any(|known| known.term == fact)
    }

    /// The allowed text values of a closed fact, such as the languages.
    /// `None` for an open fact, a boolean fact, or a name that is no fact.
    #[must_use]
    pub fn allowed_values(&self, fact: &str) -> Option<Vec<&str>> {
        let allowed = self
            .facts
            .iter()
            .find(|known| known.term == fact)?
            .allowed
            .as_ref()?;
        let values: Vec<&str> = allowed.iter().filter_map(Value::as_str).collect();
        (values.len() == allowed.len()).then_some(values)
    }

    /// Every row of the alias table, in the order of the file.
    #[must_use]
    pub fn aliases(&self) -> &[Alias] {
        &self.aliases
    }

    /// The rows of `group` whose word is `word`, which must be lower case.
    #[must_use]
    pub fn aliases_of(&self, group: TermGroup, word: &str) -> Vec<&Alias> {
        self.aliases
            .iter()
            .filter(|alias| alias.group == group && alias.word == word)
            .collect()
    }

    /// The term that `term` of `group` sits under, when the table names one.
    #[must_use]
    pub fn broader_of(&self, group: TermGroup, term: &str) -> Option<&BroaderTerm> {
        self.broader_terms
            .iter()
            .find(|broader| broader.group == group && broader.term == term)
    }
}

#[derive(Deserialize)]
struct TermsFile {
    version: String,
    kinds: GroupFile,
    categories: GroupFile,
    facts: FactsFile,
    activities: GroupFile,
    touches: GroupFile,
    qualities: GroupFile,
    artefacts: GroupFile,
}

#[derive(Deserialize)]
struct GroupFile {
    terms: Vec<TermFile>,
}

#[derive(Deserialize)]
struct TermFile {
    term: String,
}

#[derive(Deserialize)]
struct FactsFile {
    terms: Vec<Fact>,
}

#[derive(Debug, Deserialize)]
struct Fact {
    term: String,
    /// The allowed values of a closed fact; `None` for an open one.
    #[serde(default)]
    allowed: Option<Vec<Value>>,
}

#[derive(Deserialize)]
struct AliasesFile {
    version: String,
    vocabulary: String,
    aliases: Vec<Alias>,
    broader_terms: Vec<BroaderTerm>,
}

impl GroupFile {
    fn names(self) -> Vec<String> {
        self.terms.into_iter().map(|term| term.term).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_alias_maps_onto_a_term_of_its_own_group() {
        let vocabulary = Vocabulary::embedded();
        let broken: Vec<&Alias> = vocabulary
            .aliases()
            .iter()
            .filter(|alias| !vocabulary.has_term(alias.group, &alias.term))
            .collect();
        assert!(broken.is_empty(), "aliases onto no term: {broken:?}");
    }

    #[test]
    fn every_broader_term_names_two_terms_of_the_vocabulary() {
        let vocabulary = Vocabulary::embedded();
        for broader in &vocabulary.broader_terms {
            assert!(
                vocabulary.has_term(broader.group, &broader.term)
                    && vocabulary.has_term(broader.broader_group, &broader.broader_term),
                "{broader:?} names a term the vocabulary does not hold"
            );
        }
    }

    #[test]
    fn both_embedded_files_load_and_name_their_versions() {
        let vocabulary = Vocabulary::embedded();
        assert!(!vocabulary.terms_version().is_empty());
        assert!(!vocabulary.aliases_version().is_empty());
    }

    #[test]
    fn a_free_word_maps_onto_its_term_and_the_term_sits_under_its_category() {
        let vocabulary = Vocabulary::embedded();

        let p99 = vocabulary.aliases_of(TermGroup::Qualities, "p99");
        let broader = vocabulary.broader_of(TermGroup::Qualities, "latency");

        assert_eq!(
            p99.iter()
                .map(|alias| alias.term.as_str())
                .collect::<Vec<_>>(),
            ["latency"]
        );
        assert_eq!(p99[0].relation, Relation::Synonym);
        let broader = broader.expect("latency sits under a category");
        assert_eq!(
            (broader.broader_group, broader.broader_term.as_str()),
            (TermGroup::Categories, "performance")
        );
        assert!(vocabulary.aliases_of(TermGroup::Touches, "p99").is_empty());
    }

    #[test]
    fn a_word_of_several_terms_maps_onto_each_of_them() {
        let vocabulary = Vocabulary::embedded();

        let tests: Vec<&str> = vocabulary
            .aliases_of(TermGroup::Artefacts, "tests")
            .iter()
            .map(|alias| alias.term.as_str())
            .collect();

        assert_eq!(
            tests,
            [
                "unit-test",
                "contract-test",
                "fitness-test",
                "integration-test"
            ]
        );
    }

    #[test]
    fn languages_is_a_closed_fact_and_role_is_an_open_one() {
        let vocabulary = Vocabulary::embedded();

        let languages = vocabulary
            .allowed_values("languages")
            .expect("languages is closed");

        assert!(languages.contains(&"rust") && languages.contains(&"go"));
        assert!(!languages.contains(&"golang"));
        assert_eq!(vocabulary.allowed_values("role"), None);
        assert_eq!(vocabulary.allowed_values("writes_code"), None);
        assert!(vocabulary.has_fact("repo_dependencies"));
        assert!(!vocabulary.has_fact("frobnicate"));
        assert!(vocabulary.has_term(TermGroup::Kinds, "tool"));
        assert!(!vocabulary.has_term(TermGroup::Touches, "tool"));
    }
}
