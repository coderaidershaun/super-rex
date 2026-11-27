//! The rules that hold the map to the shared vocabulary: `kind`, `category`,
//! `claims` and `scope` use only its terms (rules M1, M2, M3 and M5).

use yaml_rust2::Yaml;

use crate::commands::skills::Rule;
use crate::commands::skills::header::Metadata;

use super::shown;
use super::vocabulary::{Fact, FactType, Group, Vocabulary};

/// The fields of `claims`. `facts` takes facts; each other field takes the
/// terms of the group whose header names it.
const CLAIMS_FIELDS: [&str; 5] = ["facts", "activity", "touches", "quality", "artefact"];

pub(super) fn findings(map: &Metadata) -> Vec<(Rule, String)> {
    let vocabulary = Vocabulary::embedded();
    let mut findings = Vec::new();
    let required = [
        (Rule::M1, "kind", &vocabulary.kinds),
        (Rule::M2, "category", &vocabulary.categories),
    ];
    for (rule, key, group) in required {
        findings.extend(term_problem(map, key, group).map(|message| (rule, message)));
    }
    if let Some(claims) = map.get("claims") {
        let messages = claims_problems(claims, vocabulary);
        findings.extend(messages.into_iter().map(|message| (Rule::M3, message)));
    }
    if let Some(scope) = map.get("scope") {
        let messages = scope_problems(scope, vocabulary);
        findings.extend(messages.into_iter().map(|message| (Rule::M5, message)));
    }
    findings
}

/// What is wrong with the required field `key`, which holds one term of
/// `group`.
fn term_problem(map: &Metadata, key: &str, group: &Group) -> Option<String> {
    let terms = group.list();
    let Some(value) = map.get(key) else {
        return Some(format!("{key} is missing; give one of {terms}"));
    };
    let Some(term) = value.as_str() else {
        return Some(format!("{key} is not one text value; give one of {terms}"));
    };
    (!group.has(term)).then(|| format!("{key} `{term}` is not one of {terms}"))
}

fn claims_problems(claims: &Yaml, vocabulary: &Vocabulary) -> Vec<String> {
    let Some(claims) = claims.as_hash() else {
        return vec![format!("claims is {}, not a map", shown(claims))];
    };
    let mut problems = Vec::new();
    for (field, value) in claims {
        let Some(field) = field.as_str().filter(|field| CLAIMS_FIELDS.contains(field)) else {
            problems.push(format!(
                "claims holds {}, which is not one of {}",
                shown(field),
                CLAIMS_FIELDS.join(", ")
            ));
            continue;
        };
        match vocabulary.claims_group(field) {
            Some(group) => problems.extend(claimed_terms_problems(field, value, group)),
            None => problems.extend(claimed_facts_problems(value, vocabulary)),
        }
    }
    problems
}

fn claimed_terms_problems(field: &str, value: &Yaml, group: &Group) -> Vec<String> {
    let Some(items) = value.as_vec() else {
        return vec![format!("claims.{field} is not a list of terms")];
    };
    items
        .iter()
        .filter(|item| !item.as_str().is_some_and(|term| group.has(term)))
        .map(|item| {
            format!(
                "claims.{field} holds {}, which is not one of {}",
                shown(item),
                group.list()
            )
        })
        .collect()
}

fn claimed_facts_problems(facts: &Yaml, vocabulary: &Vocabulary) -> Vec<String> {
    let Some(facts) = facts.as_hash() else {
        return vec![format!("claims.facts is {}, not a map", shown(facts))];
    };
    let mut problems = Vec::new();
    for (name, value) in facts {
        let shown_name = shown(name);
        let Some(fact) = name.as_str().and_then(|name| vocabulary.fact(name)) else {
            problems.push(format!(
                "claims.facts names {shown_name}, which is no fact of the vocabulary"
            ));
            continue;
        };
        if !fact.claimable {
            problems.push(format!(
                "claims.facts names {shown_name}, which a skill may not claim; it is only checked against a scope"
            ));
            continue;
        }
        let place = format!("claims.facts.{}", fact.term);
        if fact.value_type == FactType::List {
            problems.extend(list_problems(&place, value, fact));
        } else if let Some(problem) = fact.value_problem(value) {
            problems.push(format!("{place} holds {problem}"));
        }
    }
    problems
}

fn scope_problems(scope: &Yaml, vocabulary: &Vocabulary) -> Vec<String> {
    let Some(scope) = scope.as_hash() else {
        return vec![format!("scope is {}, not a map", shown(scope))];
    };
    let mut problems = Vec::new();
    for (key, value) in scope {
        let named = key
            .as_str()
            .and_then(|key| Some((key, vocabulary.scope_fact(key)?)));
        let Some((key, fact)) = named else {
            problems.push(format!(
                "scope holds {}, which is no scope key of the vocabulary: {}",
                shown(key),
                vocabulary.scope_key_list()
            ));
            continue;
        };
        let place = format!("scope.{key}");
        problems.extend(list_problems(&place, value, fact));
    }
    problems
}

/// What is wrong with `value` at `place`, which must be a list of at least
/// one value of `fact`.
fn list_problems(place: &str, value: &Yaml, fact: &Fact) -> Vec<String> {
    let items = value.as_vec().filter(|items| !items.is_empty());
    let Some(items) = items else {
        return vec![format!(
            "{place} is not a list of at least one value; write it as [<{}>]",
            fact.item_type()
        )];
    };
    items
        .iter()
        .filter_map(|item| fact.value_problem(item))
        .map(|problem| format!("{place} holds {problem}"))
        .collect()
}
