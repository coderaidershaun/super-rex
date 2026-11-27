//! One page of a result: its items, the count of the rest, the cursor that fetches it,
//! the questions asked once per page, and the size limit that keeps an answer small.

use std::collections::BTreeSet;

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::files::choose_files;
use super::index::{Index, IndexVersion, SkillEntry};
use super::request::{DEFAULT_LIMIT, FormTerms, Need, RequestForm, SearchRequest, TermField};
use super::response::{Candidate, Item, Question, RequestResult, SearchError, Slot};
use crate::commands::skills::SKILL_FILE;
use crate::commands::skills::serve::library::library_version;

/// The token budget of one result is this many tokens for each item, and
/// never less than for a full page of the default limit: 1,200 for any page
/// of up to 10 items.
pub const MAX_TOKENS_PER_ITEM: usize = 120;

/// The estimate counts this many tokens for each five bytes of JSON that is
/// not a hash: 2.5 bytes per token, as measured on answers of words and names.
pub const TEXT_TOKENS_PER_FIVE_BYTES: usize = 2;

/// The estimate counts this many tokens for each eight bytes of a hash of 64
/// hex digits: 1.6 bytes per token, as measured.
pub const HASH_TOKENS_PER_EIGHT_BYTES: usize = 5;

const CURSOR_TAG: &str = "c1";
const BINDING_DIGITS: usize = 16;
const HASH_DIGITS: usize = 64;

/// The tokens that the compact JSON `text` costs a caller, rounded up.
#[must_use]
pub fn estimated_tokens(text: &str) -> usize {
    let mut hash_bytes = 0;
    let mut run = 0;
    for byte in text.bytes().chain(std::iter::once(b' ')) {
        if byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte) {
            run += 1;
            continue;
        }
        if run == HASH_DIGITS {
            hash_bytes += run;
        }
        run = 0;
    }
    let text_bytes = text.len() - hash_bytes;
    (text_bytes * TEXT_TOKENS_PER_FIVE_BYTES).div_ceil(5)
        + (hash_bytes * HASH_TOKENS_PER_EIGHT_BYTES).div_ceil(8)
}

impl IndexVersion {
    /// The version that `head` reports, built from the files the index read:
    /// `SKILL.md` and every other file of each served entry, then
    /// `vocabulary`, the vocabulary version beside the first folder.
    #[must_use]
    pub fn of(index: &Index, vocabulary: Option<&str>) -> IndexVersion {
        let mut files = Vec::new();
        for entry in index
            .skills
            .iter()
            .filter(|entry| entry.shadowed_by.is_none())
        {
            let name = &entry.name;
            files.push((format!("{name}/{SKILL_FILE}"), entry.sha256.clone()));
            for file in &entry.files {
                files.push((format!("{name}/{}", file.path), file.sha256.clone()));
            }
        }
        IndexVersion(library_version(files, vocabulary))
    }
}

/// What binds a cursor to one request on one version of the library.
pub(super) struct CursorKey {
    version: String,
    request: String,
}

/// The part of a request that the cursor binds, in this field order. The
/// `facts` of the form are sorted by name, so their order as sent does not count.
#[derive(Serialize)]
struct Bound<'a> {
    form: Value,
    kinds: &'a Option<Vec<String>>,
    categories: &'a Option<Vec<String>>,
}

impl CursorKey {
    pub(super) fn new(version: &IndexVersion, request: &SearchRequest, form: &RequestForm) -> Self {
        let mut form = serde_json::to_value(form).expect("a request form is JSON");
        if let Some(facts) = form.get_mut("facts").and_then(Value::as_object_mut) {
            facts.sort_keys();
        }
        let bound = Bound {
            form,
            kinds: &request.kinds,
            categories: &request.categories,
        };
        let text = serde_json::to_string(&bound).expect("a request form is JSON");
        let digest: String = Sha256::digest(text.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let version = version.0.strip_prefix("sha256:").unwrap_or(&version.0);
        CursorKey {
            version: version.chars().take(BINDING_DIGITS).collect(),
            request: digest[..BINDING_DIGITS].to_owned(),
        }
    }

    /// The cursor of the page that starts at `offset`.
    pub(super) fn cursor(&self, offset: usize) -> String {
        format!("{CURSOR_TAG}.{offset}.{}.{}", self.version, self.request)
    }

    /// The offset that `cursor` continues from. The caller checks it against
    /// the length of the list.
    pub(super) fn read(&self, cursor: &str) -> Result<usize, SearchError> {
        let bad = |reason: &str| SearchError::BadCursor {
            reason: reason.to_owned(),
        };
        let parts: Vec<&str> = cursor.split('.').collect();
        let [tag, offset, version, request] = parts[..] else {
            return Err(bad("it is not four parts joined by `.`"));
        };
        if tag != CURSOR_TAG {
            return Err(bad("its tag is not `c1`"));
        }
        if offset.is_empty() || !offset.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(bad("its offset is not a decimal number"));
        }
        let offset: usize = offset.parse().map_err(|_| bad("its offset is too large"))?;
        if !is_binding(version) || !is_binding(request) {
            return Err(bad("its bindings are not 16 lower-case hex digits"));
        }
        if request != self.request {
            return Err(bad("it was made for another request"));
        }
        if version != self.version {
            return Err(SearchError::StaleCursor);
        }
        Ok(offset)
    }
}

fn is_binding(text: &str) -> bool {
    text.len() == BINDING_DIGITS
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Shrink a page whose estimate is over its budget, one step at a time for
/// every item, until it fits. Then set the cursor of the items after it.
pub(super) fn fit(result: &mut RequestResult, key: &CursorKey, offset: usize) {
    let steps: [fn(&mut Item); 6] = [
        cut_why,
        leave_out_hashes,
        |item| item.words = None,
        |item| item.placement = None,
        |item| item.open_when.clear(),
        |item| item.files.clear(),
    ];
    set_cursor(result, key, offset);
    if result.items.is_empty() {
        return;
    }
    for step in steps {
        if fits(result) {
            return;
        }
        result.items.iter_mut().for_each(step);
    }
    while !fits(result) && result.items.len() > 1 {
        result.items.pop();
        result.more += 1;
        set_cursor(result, key, offset);
    }
}

fn set_cursor(result: &mut RequestResult, key: &CursorKey, offset: usize) {
    let next = offset + result.items.len();
    result.cursor = (result.more > 0).then(|| key.cursor(next));
}

fn fits(result: &RequestResult) -> bool {
    let budget = MAX_TOKENS_PER_ITEM * result.items.len().max(DEFAULT_LIMIT);
    let text = serde_json::to_string(result).expect("a result of strings and numbers is JSON");
    estimated_tokens(&text) <= budget
}

fn cut_why(item: &mut Item) {
    if let Some(head) = item.why.find("; ") {
        item.why.truncate(head);
    }
}

fn leave_out_hashes(item: &mut Item) {
    item.sha256 = None;
    for file in &mut item.files {
        file.sha256 = None;
    }
}

/// Write the `confirm` of each item of a page. A question that an earlier
/// item of the page asked is not asked again; the item points to that item.
pub(super) fn write_confirms(page: &mut [Item], candidates: &[Candidate]) {
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
pub(super) fn item(
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
        open_when.extend(metadata.open_when.iter().cloned());
        open_when.extend(choice.open_when);
    }
    Ok(Item {
        skill: entry.name.clone(),
        library: entry.library.clone(),
        kind: metadata.map(|metadata| metadata.kind),
        category: metadata.map(|metadata| metadata.category.clone()),
        slot: slot(entry, &candidate.needs[0]),
        tier: candidate.tier,
        why: candidate.why.clone(),
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
