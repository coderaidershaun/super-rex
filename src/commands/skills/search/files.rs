//! The second step of a choice: which files of one found skill match the request, topic files by
//! the skill's own router cases, and a language file or code twin only on a language match.

use rusqlite::Connection;

use super::index::{FileEntry, FileRole, OpenWhen, SkillEntry};
use super::keyword::{KEYWORD_SCORE_RATIO, keyword_error, match_query, ranked_rows};
use super::request::FormTerms;
use super::response::{FileChoice, FileItem, SearchError};

/// The most topic files of one skill when its header gives no `max_topics`.
pub const DEFAULT_MAX_TOPICS: usize = 5;

/// The bm25 weights of the router case and the intents of a topic file.
const WHEN_WEIGHT: f64 = 1.0;
const TOPIC_INTENTS_WEIGHT: f64 = 1.0;

/// The files of `entry` for `form`.
///
/// # Errors
/// [`SearchError::KeywordSearch`] when SQLite fails.
pub fn choose_files(entry: &SkillEntry, form: &FormTerms) -> Result<FileChoice, SearchError> {
    let topics: Vec<&FileEntry> = entry
        .files
        .iter()
        .filter(|file| file.role == Some(FileRole::Topic))
        .collect();
    let max_topics = entry
        .metadata
        .as_ref()
        .and_then(|metadata| metadata.max_topics)
        .unwrap_or(DEFAULT_MAX_TOPICS);
    let chosen = if form.needs.is_empty() || topics.is_empty() {
        Vec::new()
    } else {
        ranked_topics(&topics, &form.needs, max_topics)?
    };
    let speaks = |file: &FileEntry| {
        file.language
            .as_ref()
            .is_some_and(|language| form.facts.languages.contains(language))
    };

    let mut choice = FileChoice::default();
    for topic in &chosen {
        choice.files.push(file_item(topic));
        if let Some(twin) = code_twin(entry, topic).filter(|twin| speaks(twin)) {
            choice.files.push(file_item(twin));
        }
    }
    for file in &entry.files {
        let is_language_file = file.role == Some(FileRole::Language)
            || (file.role == Some(FileRole::Code) && twin_topic(entry, file).is_none());
        if is_language_file && speaks(file) {
            choice.files.push(file_item(file));
        }
    }
    if chosen.is_empty() {
        for topic in &topics {
            let when = router_case(topic);
            choice.open_when.push(open_file(&topic.path, &when));
            if let Some(twin) = code_twin(entry, topic).filter(|twin| speaks(twin)) {
                choice.open_when.push(open_file(&twin.path, &when));
            }
        }
    }
    Ok(choice)
}

/// The topic files that match the needs best, best first and then in path
/// order, cut at the score ratio and at `max_topics`.
fn ranked_topics<'a>(
    topics: &[&'a FileEntry],
    needs: &[String],
    max_topics: usize,
) -> Result<Vec<&'a FileEntry>, SearchError> {
    let connection = Connection::open_in_memory().map_err(keyword_error)?;
    connection
        .execute_batch(r#"CREATE VIRTUAL TABLE topics USING fts5("when", intents);"#)
        .map_err(keyword_error)?;
    for (position, topic) in topics.iter().enumerate() {
        let rowid = i64::try_from(position).expect("a skill holds fewer than 2^63 files");
        connection
            .execute(
                r#"INSERT INTO topics (rowid, "when", intents) VALUES (?1, ?2, ?3)"#,
                (
                    rowid,
                    topic.when.clone().unwrap_or_default(),
                    topic.intents.join(". "),
                ),
            )
            .map_err(keyword_error)?;
    }
    let sql = format!(
        "SELECT rowid, bm25(topics, {WHEN_WEIGHT:?}, {TOPIC_INTENTS_WEIGHT:?}) \
         FROM topics WHERE topics MATCH ?1"
    );
    let mut best_of: Vec<Option<f64>> = vec![None; topics.len()];
    for need in needs {
        let Some(query) = match_query(need) else {
            continue;
        };
        for (position, relevance) in ranked_rows(&connection, &sql, &query)? {
            let best = &mut best_of[position];
            *best = Some(best.map_or(relevance, |known| known.max(relevance)));
        }
    }

    let mut scored: Vec<(&FileEntry, f64)> = topics
        .iter()
        .zip(best_of)
        .filter_map(|(topic, relevance)| relevance.map(|relevance| (*topic, relevance)))
        .collect();
    scored.sort_by(|left, right| {
        right
            .1
            .total_cmp(&left.1)
            .then_with(|| left.0.path.cmp(&right.0.path))
    });
    let Some(&(_, best)) = scored.first() else {
        return Ok(Vec::new());
    };
    Ok(scored
        .into_iter()
        .take_while(|(_, relevance)| *relevance >= KEYWORD_SCORE_RATIO * best)
        .take(max_topics)
        .map(|(topic, _)| topic)
        .collect())
}

/// The code file `x-code.md` of the topic `x.md`.
fn code_twin<'a>(entry: &'a SkillEntry, topic: &FileEntry) -> Option<&'a FileEntry> {
    let stem = topic.path.strip_suffix(".md")?;
    let twin_path = format!("{stem}-code.md");
    entry
        .files
        .iter()
        .find(|file| file.role == Some(FileRole::Code) && file.path == twin_path)
}

/// The topic file `x.md` of the code file `x-code.md`.
fn twin_topic<'a>(entry: &'a SkillEntry, code: &FileEntry) -> Option<&'a FileEntry> {
    let stem = code.path.strip_suffix("-code.md")?;
    let topic_path = format!("{stem}.md");
    entry
        .files
        .iter()
        .find(|file| file.role == Some(FileRole::Topic) && file.path == topic_path)
}

/// The case in which to open a topic file: its `when`, else its first
/// intent, else its path.
fn router_case(topic: &FileEntry) -> String {
    topic
        .when
        .clone()
        .or_else(|| topic.intents.first().cloned())
        .unwrap_or_else(|| topic.path.clone())
}

fn open_file(path: &str, when: &str) -> OpenWhen {
    OpenWhen {
        skill: None,
        file: Some(path.to_owned()),
        when: when.to_owned(),
    }
}

fn file_item(file: &FileEntry) -> FileItem {
    FileItem {
        path: file.path.clone(),
        role: file.role.expect("only files with a role are chosen"),
        language: file.language.clone(),
        when: file.when.clone(),
        sha256: Some(file.sha256.clone()),
    }
}
