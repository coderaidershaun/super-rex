//! The rules for the fields of the map that take fixed forms, not vocabulary
//! terms, and for the size of the whole map (rules M4, M6, M7, M12 and M13).

use std::ops::RangeInclusive;

use yaml_rust2::Yaml;

use crate::commands::skills::Rule;
use crate::commands::skills::header::Metadata;

use super::{shown, text_list};

const PLACEMENTS: [&str; 2] = ["full", "open-when"];

const INTENT_COUNT: RangeInclusive<usize> = 3..=5;

/// A topic file may carry intents of its own beside the skill's, so one is
/// enough.
pub(super) const FILE_INTENT_COUNT: RangeInclusive<usize> = 1..=5;

/// The most words the search fields of the map may hold. The map is read
/// for every skill of every search, so each extra word costs every caller.
const MAX_MAP_WORDS: usize = 120;

pub(super) fn findings(map: &Metadata) -> Vec<(Rule, String)> {
    let mut findings = Vec::new();
    if let Some(placement) = map.get("placement") {
        findings.extend(placement_problem(placement).map(|message| (Rule::M4, message)));
    }
    findings.extend(not_when_problem(map).map(|message| (Rule::M6, message)));
    // The check that each intent phrase finds its own skill in the top five
    // results of a search is not here: it needs every checked skill and the
    // pool, so `rex skills check` runs it after the rules of each skill.
    if let Some(intents) = map.get("intents") {
        findings.extend(
            intents_problem("intents", intents, &INTENT_COUNT).map(|message| (Rule::M7, message)),
        );
    }
    if let Some(max_topics) = map.get("max_topics") {
        findings.extend(max_topics_problem(max_topics).map(|message| (Rule::M12, message)));
    }
    findings.extend(size_problem(map).map(|message| (Rule::M13, message)));
    findings
}

fn placement_problem(placement: &Yaml) -> Option<String> {
    let is_form = placement
        .as_str()
        .is_some_and(|placement| PLACEMENTS.contains(&placement));
    (!is_form).then(|| {
        format!(
            "placement {} is not `full` or `open-when`",
            shown(placement)
        )
    })
}

/// `not_when` is what a caller sees when the scope leaves the skill out, so a
/// skill with a scope must have it.
fn not_when_problem(map: &Metadata) -> Option<String> {
    let has_scope = map
        .get("scope")
        .and_then(Yaml::as_hash)
        .is_some_and(|scope| !scope.is_empty());
    let Some(not_when) = map.get("not_when") else {
        return has_scope.then(|| {
            "not_when is missing; a skill with a scope needs one line that says when not to use it"
                .to_owned()
        });
    };
    let Some(text) = not_when.as_str() else {
        return Some(format!(
            "not_when is {}, not one line of text",
            shown(not_when)
        ));
    };
    let text = text.trim();
    if text.is_empty() {
        return Some("not_when is empty".to_owned());
    }
    text.contains('\n').then(|| {
        "not_when must be one line; fold it with `>` or write it on its key's line".to_owned()
    })
}

/// What is wrong with the intents at `place`: they must be `count` pieces
/// of text, none of them empty.
pub(super) fn intents_problem(
    place: &str,
    intents: &Yaml,
    count: &RangeInclusive<usize>,
) -> Option<String> {
    let Some(intents) = text_list(intents) else {
        return Some(format!("{place} is not a list of text"));
    };
    if intents.iter().any(|intent| intent.trim().is_empty()) {
        return Some(format!("{place} holds an empty intent"));
    }
    let held = intents.len();
    (!count.contains(&held)).then(|| {
        format!(
            "{place} holds {held} intents; give {} to {}",
            count.start(),
            count.end()
        )
    })
}

fn max_topics_problem(max_topics: &Yaml) -> Option<String> {
    let is_positive = max_topics.as_i64().is_some_and(|count| count > 0);
    (!is_positive).then(|| {
        format!(
            "max_topics is {}; give a positive whole number, such as 2",
            shown(max_topics)
        )
    })
}

fn size_problem(map: &Metadata) -> Option<String> {
    let words: usize = map
        .search_fields()
        .map(|(key, value)| words_of_field(key, value))
        .sum();
    (words > MAX_MAP_WORDS).then(|| {
        format!(
            "the search fields of metadata hold {words} words, more than {MAX_MAP_WORDS}; file paths do not count"
        )
    })
}

/// The words of a search field, as `wc -w` counts the values: each piece of
/// text counts its words, and each number or `true` or `false` counts one.
/// Keys do not count, and nor do the file paths of `files` and `open_when`.
fn words_of_field(key: &str, value: &Yaml) -> usize {
    let file_key = match key {
        "files" => Some("path"),
        "open_when" => Some("file"),
        _ => None,
    };
    let Some(file_key) = file_key else {
        return words_of(value);
    };
    let Some(items) = value.as_vec() else {
        return words_of(value);
    };
    items
        .iter()
        .map(|item| match item.as_hash() {
            Some(item) => item
                .iter()
                .filter(|(key, _)| key.as_str() != Some(file_key))
                .map(|(_, value)| words_of(value))
                .sum(),
            None => words_of(item),
        })
        .sum()
}

fn words_of(value: &Yaml) -> usize {
    match value {
        Yaml::String(text) => text.split_whitespace().count(),
        Yaml::Real(_) | Yaml::Integer(_) | Yaml::Boolean(_) => 1,
        Yaml::Array(items) => items.iter().map(words_of).sum(),
        Yaml::Hash(map) => map.values().map(words_of).sum(),
        Yaml::Null | Yaml::Alias(_) | Yaml::BadValue => 0,
    }
}
