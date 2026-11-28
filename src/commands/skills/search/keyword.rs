//! Lookup C: keyword search with SQLite FTS5 and bm25 over each skill's name, card and intents,
//! one query per need, widened by aliases; it only proposes candidates, each with a question.

use std::cmp::Ordering;

use rusqlite::Connection;

use super::index::Index;
use super::request::{FormTerms, Need, TextQuery};
use super::response::{Candidate, Found, KeywordScore, Question, SearchError, Tier};
use super::vocabulary::{TermGroup, Vocabulary};

/// The bm25 weight of each column of the keyword table.
pub const NAME_WEIGHT: f64 = 4.0;
pub const CARD_WEIGHT: f64 = 2.0;
pub const INTENTS_WEIGHT: f64 = 3.0;

/// The most candidates one keyword query keeps.
pub const MAX_KEYWORD_CANDIDATES: usize = 5;

/// A candidate is kept only when its relevance is at least this share of the
/// best one of its query. The results of gate A set the real value.
pub const KEYWORD_SCORE_RATIO: f64 = 0.5;

/// How lookup C cuts its candidates, and whether an answer shows their
/// scores. Only `rex skills search` changes them, to measure the keyword
/// search; the default is the built-in ratio with no scores.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeywordSettings {
    ratio: f64,
    shows_scores: bool,
}

impl Default for KeywordSettings {
    fn default() -> Self {
        KeywordSettings {
            ratio: KEYWORD_SCORE_RATIO,
            shows_scores: false,
        }
    }
}

impl KeywordSettings {
    /// Settings with `ratio` in place of [`KEYWORD_SCORE_RATIO`] at the cut
    /// of each need's candidates, when it is given.
    ///
    /// # Errors
    /// [`SearchError::BadKeywordRatio`] when `ratio` is not above 0 and at
    /// most 1.
    pub fn new(ratio: Option<f64>, shows_scores: bool) -> Result<KeywordSettings, SearchError> {
        let ratio = ratio.unwrap_or(KEYWORD_SCORE_RATIO);
        let is_share = ratio > 0.0 && ratio <= 1.0;
        if !is_share {
            return Err(SearchError::BadKeywordRatio { ratio });
        }
        Ok(KeywordSettings {
            ratio,
            shows_scores,
        })
    }

    #[must_use]
    pub fn ratio(&self) -> f64 {
        self.ratio
    }

    #[must_use]
    pub fn shows_scores(&self) -> bool {
        self.shows_scores
    }

    /// The ratio when it is not the built-in one. A cursor binds it, so that
    /// a cursor never continues a list that another ratio cut.
    #[must_use]
    pub fn changed_ratio(&self) -> Option<f64> {
        let is_built_in = self.ratio == KEYWORD_SCORE_RATIO;
        (!is_built_in).then_some(self.ratio)
    }
}

/// The groups whose terms widen a keyword query, in the order of the
/// vocabulary.
const WIDENING_GROUPS: [TermGroup; 4] = [
    TermGroup::Activities,
    TermGroup::Touches,
    TermGroup::Qualities,
    TermGroup::Artefacts,
];

/// One skill that a keyword query matches.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ranked {
    /// The position of the skill in `Index::skills`.
    pub skill: usize,
    /// Minus the bm25 score: higher is better.
    pub relevance: f64,
}

/// The keyword table of one index, built in memory once and queried many
/// times.
#[derive(Debug)]
pub struct KeywordTable {
    connection: Connection,
    /// The name and library priority of each entry, by position, to break
    /// ties of relevance.
    tie_breaks: Vec<(String, usize)>,
}

impl KeywordTable {
    /// # Errors
    /// [`SearchError::KeywordSearch`] when SQLite cannot build the table.
    pub fn build(index: &Index) -> Result<KeywordTable, SearchError> {
        let connection = Connection::open_in_memory().map_err(keyword_error)?;
        connection
            .execute_batch("CREATE VIRTUAL TABLE skills USING fts5(name, card, intents);")
            .map_err(keyword_error)?;
        let mut insert = connection
            .prepare("INSERT INTO skills (rowid, name, card, intents) VALUES (?1, ?2, ?3, ?4)")
            .map_err(keyword_error)?;
        for (position, entry) in index.skills.iter().enumerate() {
            let intents = match &entry.metadata {
                Some(metadata) => metadata.intents.join(". "),
                None => entry.when_to_use.clone(),
            };
            let rowid = i64::try_from(position).expect("an index holds fewer than 2^63 skills");
            insert
                .execute((rowid, &entry.name, &entry.card, intents))
                .map_err(keyword_error)?;
        }
        drop(insert);
        let tie_breaks = index
            .skills
            .iter()
            .map(|entry| (entry.name.clone(), entry.library_priority))
            .collect();
        Ok(KeywordTable {
            connection,
            tie_breaks,
        })
    }

    /// Every skill that `text` matches, best first, equal relevance in name
    /// order. Every lookup C query and the intent check of `rex skills
    /// check` rank with this.
    ///
    /// # Errors
    /// [`SearchError::KeywordSearch`] when SQLite fails.
    pub fn rank(&self, text: &str) -> Result<Vec<Ranked>, SearchError> {
        let Some(query) = match_query(text) else {
            return Ok(Vec::new());
        };
        let sql = format!(
            "SELECT rowid, bm25(skills, {NAME_WEIGHT:?}, {CARD_WEIGHT:?}, {INTENTS_WEIGHT:?}) \
             FROM skills WHERE skills MATCH ?1"
        );
        let mut ranked: Vec<Ranked> = ranked_rows(&self.connection, &sql, &query)?
            .into_iter()
            .map(|(skill, relevance)| Ranked { skill, relevance })
            .collect();
        ranked.sort_by(|left, right| self.compare(left, right));
        Ok(ranked)
    }

    fn compare(&self, left: &Ranked, right: &Ranked) -> Ordering {
        let (left_name, left_priority) = &self.tie_breaks[left.skill];
        let (right_name, right_priority) = &self.tie_breaks[right.skill];
        right
            .relevance
            .total_cmp(&left.relevance)
            .then_with(|| left_name.cmp(right_name))
            .then_with(|| left_priority.cmp(right_priority))
            .then_with(|| left.skill.cmp(&right.skill))
    }
}

/// Lookup C: for each keyword query of the form, the best few skills.
///
/// # Errors
/// [`SearchError::KeywordSearch`] when SQLite fails.
pub fn lookup_c(index: &Index, form: &FormTerms) -> Result<Found, SearchError> {
    lookup_c_at_ratio(index, form, KEYWORD_SCORE_RATIO)
}

/// Lookup C with `ratio` in place of [`KEYWORD_SCORE_RATIO`].
///
/// # Errors
/// [`SearchError::KeywordSearch`] when SQLite fails.
pub fn lookup_c_at_ratio(
    index: &Index,
    form: &FormTerms,
    ratio: f64,
) -> Result<Found, SearchError> {
    let mut found = Found::default();
    if form.keyword_queries.is_empty() {
        return Ok(found);
    }
    let table = KeywordTable::build(index)?;
    for query in &form.keyword_queries {
        let ranked = table.rank(&query.text)?;
        let Some((best, kept)) = cut(index, &ranked, ratio) else {
            continue;
        };
        for kept in kept {
            let score = KeywordScore {
                score: kept.relevance,
                best,
            };
            found
                .candidates
                .push(keyword_candidate(index, query, kept, score));
        }
    }
    Ok(found)
}

/// The best relevance of one query and the head of its ranked list that it
/// keeps, or `None` when nothing matched. Only served entries count toward
/// the limit and set the best relevance; the others met on the way are kept,
/// so the answer can report them as dropped.
fn cut(index: &Index, ranked: &[Ranked], ratio: f64) -> Option<(f64, Vec<Ranked>)> {
    let best = ranked
        .iter()
        .find(|entry| index.is_served(entry.skill))
        .or(ranked.first())
        .map(|entry| entry.relevance)?;
    let mut kept = Vec::new();
    let mut served = 0;
    for entry in ranked {
        if entry.relevance < ratio * best || served == MAX_KEYWORD_CANDIDATES {
            break;
        }
        if index.is_served(entry.skill) {
            served += 1;
        }
        kept.push(*entry);
    }
    Some((best, kept))
}

fn keyword_candidate(
    index: &Index,
    query: &TextQuery,
    kept: Ranked,
    score: KeywordScore,
) -> Candidate {
    let why = match &query.need {
        Need::Text(position) => format!("keyword: need {}", position + 1),
        Need::Word(word) => format!("keyword: {word}"),
        other => unreachable!("a keyword query serves a need or a word, not {other:?}"),
    };
    Candidate {
        skill: kept.skill,
        tier: Tier::C,
        needs: vec![query.need.clone()],
        why,
        questions: vec![Question::Keyword {
            skill: index.skills[kept.skill].name.clone(),
            need: query.need.clone(),
        }],
        through_alias: false,
        relevance: Some(kept.relevance),
        keyword_score: Some(score),
    }
}

/// The FTS5 query of `text`: its own words, then each term the words name
/// with that term's alias words and broader term, each as a quoted phrase,
/// joined by `OR`. `None` when the text holds no word.
///
/// Every phrase holds only lower-case letters and digits between its quotes,
/// so no caller text can change the syntax of the query.
pub(super) fn match_query(text: &str) -> Option<String> {
    let words = words_of(text);
    if words.is_empty() {
        return None;
    }
    let mut phrases: Vec<Vec<String>> = Vec::new();
    for word in &words {
        push_phrase(&mut phrases, vec![word.clone()]);
    }
    let vocabulary = Vocabulary::embedded();
    for (group, term) in matched_terms(vocabulary, &words) {
        push_phrase(&mut phrases, words_of(&term));
        for alias in vocabulary.aliases() {
            if alias.group == group && alias.term == term {
                push_phrase(&mut phrases, words_of(&alias.word));
            }
        }
        if let Some(broader) = vocabulary.broader_of(group, &term) {
            push_phrase(&mut phrases, words_of(&broader.broader_term));
        }
    }
    let quoted: Vec<String> = phrases
        .iter()
        .map(|phrase| format!("\"{}\"", phrase.join(" ")))
        .collect();
    Some(quoted.join(" OR "))
}

/// The terms of the widening groups whose words occur as a run of `words`,
/// directly or through an alias word, in group order and with no repeats.
fn matched_terms(vocabulary: &Vocabulary, words: &[String]) -> Vec<(TermGroup, String)> {
    let mut terms: Vec<(TermGroup, String)> = Vec::new();
    for group in WIDENING_GROUPS {
        for start in 0..words.len() {
            for end in start + 1..=words.len() {
                let candidate = words[start..end].join("-");
                if vocabulary.has_term(group, &candidate) {
                    push_term(&mut terms, group, candidate);
                }
            }
        }
        for alias in vocabulary.aliases() {
            if alias.group == group && holds_run(words, &words_of(&alias.word)) {
                push_term(&mut terms, group, alias.term.clone());
            }
        }
    }
    terms
}

fn push_term(terms: &mut Vec<(TermGroup, String)>, group: TermGroup, term: String) {
    let pair = (group, term);
    if !terms.contains(&pair) {
        terms.push(pair);
    }
}

fn push_phrase(phrases: &mut Vec<Vec<String>>, phrase: Vec<String>) {
    if !phrase.is_empty() && !phrases.contains(&phrase) {
        phrases.push(phrase);
    }
}

fn holds_run(words: &[String], run: &[String]) -> bool {
    !run.is_empty() && words.windows(run.len()).any(|window| window == run)
}

/// The lower-case runs of letters and digits of `text`.
pub(super) fn words_of(text: &str) -> Vec<String> {
    text.split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// Runs `sql`, which selects a rowid and a bm25 score and takes the match
/// query as `?1`, and gives each rowid with its relevance.
pub(super) fn ranked_rows(
    connection: &Connection,
    sql: &str,
    query: &str,
) -> Result<Vec<(usize, f64)>, SearchError> {
    let mut statement = connection.prepare(sql).map_err(keyword_error)?;
    let rows = statement
        .query_map([query], |row| {
            let rowid: i64 = row.get(0)?;
            let score: f64 = row.get(1)?;
            Ok((rowid, score))
        })
        .map_err(keyword_error)?;
    let mut ranked = Vec::new();
    for row in rows {
        let (rowid, score) = row.map_err(keyword_error)?;
        let position =
            usize::try_from(rowid).expect("every rowid is a position, so never negative");
        ranked.push((position, -score));
    }
    Ok(ranked)
}

pub(super) fn keyword_error(source: rusqlite::Error) -> SearchError {
    SearchError::KeywordSearch { source }
}
