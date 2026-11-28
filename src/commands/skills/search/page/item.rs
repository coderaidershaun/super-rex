//! The item of one found skill on a page: its card with every optional field filled, and its
//! `confirm`, where a question that an earlier item of the page asked is not asked again.

use std::collections::BTreeSet;

use super::super::files::choose_files;
use super::super::index::{Index, SkillEntry};
use super::super::request::{FormTerms, Need, TermField};
use super::super::response::{Candidate, Item, Question, SearchError, Slot};

/// Write the `confirm` of each item of a page. A question that an earlier
/// item of the page asked is not asked again; the item points to that item.
pub(crate) fn write_confirms(page: &mut [Item], candidates: &[Candidate]) {
    let mut asked: Vec<(&Question, usize)> = Vec::new();
    for (at, candidate) in candidates.iter().enumerate() {
        let mut left = Vec::new();
        let mut askers = BTreeSet::new();
        for question in &candidate.questions {
            match asked.iter().find(|(earlier, _)| *earlier == question) {
                Some((_, asker)) => {
                    askers.insert(*asker);
                }
                None => left.push(question),
            }
        }
        asked.extend(left.iter().map(|question| (*question, at)));
        let text = questions_text(&left);
        let names: Vec<&str> = askers
            .iter()
            .map(|&asker| page[asker].skill.as_str())
            .collect();
        let names = names.join(", ");
        page[at].confirm = match (text.is_empty(), names.is_empty()) {
            (true, true) => None,
            (false, true) => Some(text),
            (false, false) => Some(format!("{text} Also as for {names}.")),
            (true, false) => Some(format!("As for {names}.")),
        };
    }
}

fn questions_text(questions: &[&Question]) -> String {
    let mut scope = Vec::new();
    let mut keyword = Vec::new();
    for question in questions {
        match question {
            Question::Scope { fact, values } => {
                scope.push(format!("{fact} {}", values.join(" or ")));
            }
            Question::Keyword { skill, need } => {
                let need = match need {
                    Need::Text(position) => format!("need {}", position + 1),
                    Need::Word(word) => word.clone(),
                    other => {
                        unreachable!("a keyword question names a need or a word, not {other:?}")
                    }
                };
                keyword.push(format!("Is {skill} right for {need}?"));
            }
        }
    }
    let mut sentences = Vec::new();
    if !scope.is_empty() {
        sentences.push(format!("Does the task have {}?", scope.join(" and ")));
    }
    sentences.extend(keyword);
    sentences.join(" ")
}

/// The item of one candidate of a page, with every optional field filled.
pub(crate) fn item(
    index: &Index,
    terms: &FormTerms,
    candidate: &Candidate,
) -> Result<Item, SearchError> {
    let entry = &index.skills[candidate.skill];
    let metadata = entry.metadata.as_ref();
    let mut files = Vec::new();
    let mut open_when = Vec::new();
    if let Some(metadata) = metadata {
        let choice = choose_files(entry, terms)?;
        files = choice.files;
        // A topic file's `when` and a declared item can name the same file
        // and case; the item lists it once.
        for open in metadata.open_when.iter().cloned().chain(choice.open_when) {
            if !open_when.contains(&open) {
                open_when.push(open);
            }
        }
    }
    Ok(Item {
        skill: entry.name.clone(),
        library: entry.library.clone(),
        kind: metadata.map(|metadata| metadata.kind),
        category: metadata.map(|metadata| metadata.category.clone()),
        slot: slot(entry, &candidate.needs[0]),
        tier: candidate.tier,
        why: candidate.why.clone(),
        score: None,
        best_score: None,
        card: entry.card.clone(),
        not_when: metadata.and_then(|metadata| metadata.not_when.clone()),
        confirm: None,
        placement: metadata.map(|metadata| metadata.placement),
        sha256: Some(entry.sha256.clone()),
        words: Some(entry.words),
        files,
        open_when,
    })
}

fn slot(entry: &SkillEntry, first_need: &Need) -> Slot {
    match first_need {
        Need::Term {
            field: TermField::Quality,
            term,
        } => Slot::Quality(term.clone()),
        Need::Term {
            field: TermField::Artefact,
            term,
        } => Slot::Artefact(term.clone()),
        _ => entry
            .metadata
            .as_ref()
            .map_or(Slot::Keyword, |metadata| Slot::Kind(metadata.kind)),
    }
}
