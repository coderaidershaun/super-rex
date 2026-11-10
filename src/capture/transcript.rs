//! Reads one Claude Code transcript file into the facts that hook events do not give:
//! tokens, and the counts of a background run. It knows nothing of the store or of jobs.

use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use serde_json::Value;

use super::error::CaptureError;
use super::tokens::TokenCounts;

/// The folders whose sub-folders are skills, as the two last parts of their paths: the
/// skills folders of Claude Code, and the orchestrator library of rex.
const SKILL_ROOTS: [(&str, &str); 2] = [(".claude", "skills"), (".rex", "orchestrators")];

/// What one transcript says about its run.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TranscriptFacts {
    /// Each message ID counted one time, all models added up.
    pub tokens: TokenCounts,
    /// Count of distinct message IDs.
    pub requests: u64,
    pub tool_uses: u64,
    /// Claude Code's own count: the pieces of the new text split on newlines, not a diff.
    pub lines_added: u64,
    pub lines_removed: u64,
    /// The `timestamp` text of the first and last record that has one, as written.
    pub first_at: Option<String>,
    pub last_at: Option<String>,
    /// The brief, for a sub-agent transcript.
    pub first_user_text: Option<String>,
    pub last_assistant_text: Option<String>,
    /// Of the last assistant record that names one.
    pub model: Option<String>,
    pub effort: Option<String>,
    /// Sorted, each name one time.
    pub skills_opened: Vec<String>,
    /// Lines that are not valid JSON, a cut last line included. They are skipped.
    pub damaged_lines: u64,
    /// Size of the file that was read.
    pub bytes: u64,
}

/// Read the transcript at `path`. Returns `Ok(None)` when there is no file there: a
/// transcript that is gone, or not written yet, is a normal state.
///
/// Records of unknown types and unknown fields are ignored, so a newer Claude Code
/// still reads.
///
/// # Errors
/// [`CaptureError::ReadTranscript`] if the file exists and cannot be read.
pub fn read_transcript(path: &Path) -> Result<Option<TranscriptFacts>, CaptureError> {
    let Some(contents) = read_file(path)? else {
        return Ok(None);
    };
    let mut tally = Tally::default();
    for record in records(&contents) {
        match record {
            Some(record) => tally.add(&record),
            None => tally.facts.damaged_lines += 1,
        }
    }
    Ok(Some(tally.finish(contents.len() as u64)))
}

/// The bytes of the transcript at `path`, or `None` when there is no file there.
pub(super) fn read_file(path: &Path) -> Result<Option<Vec<u8>>, CaptureError> {
    match fs::read(path) {
        Ok(contents) => Ok(Some(contents)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(source) => Err(CaptureError::ReadTranscript {
            path: path.to_path_buf(),
            source,
        }),
    }
}

/// Each record of a transcript, in order. `None` is a line that is not valid JSON,
/// a cut last line included. Blank lines are not records.
pub(super) fn records(contents: &[u8]) -> impl Iterator<Item = Option<Value>> {
    contents
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.trim_ascii().is_empty())
        .map(|line| serde_json::from_slice::<Value>(line).ok())
}

/// Facts gathered one record at a time. Some depend on later records: a request is
/// split over several records, and a read fails only when a later result says so.
#[derive(Default)]
struct Tally {
    facts: TranscriptFacts,
    has_first_user: bool,
    message_ids: HashSet<String>,
    tool_use_ids: HashSet<String>,
    failed_tool_use_ids: HashSet<String>,
    skills: BTreeSet<String>,
    skill_reads: Vec<SkillRead>,
}

struct SkillRead {
    tool_use_id: String,
    skill: String,
}

impl Tally {
    fn add(&mut self, record: &Value) {
        if let Some(at) = text_at(record, "timestamp") {
            if self.facts.first_at.is_none() {
                self.facts.first_at = Some(at.to_owned());
            }
            self.facts.last_at = Some(at.to_owned());
        }
        let Some(message) = record.get("message") else {
            return;
        };
        match text_at(record, "type") {
            Some("assistant") => self.add_assistant(record, message),
            Some("user") => self.add_user(message),
            _ => {}
        }
    }

    fn add_assistant(&mut self, record: &Value, message: &Value) {
        if let Some(model) = text_at(message, "model") {
            self.facts.model = Some(model.to_owned());
        }
        if let Some(effort) = text_at(record, "effort") {
            self.facts.effort = Some(effort.to_owned());
        }
        // One request is written as one record for each content block, and every one
        // of them repeats the request's usage.
        if let Some(id) = text_at(message, "id")
            && self.message_ids.insert(id.to_owned())
        {
            self.facts.tokens += usage_of(message);
        }
        for block in blocks_of(message) {
            match text_at(block, "type") {
                Some("text") => {
                    if let Some(text) = text_at(block, "text") {
                        self.facts.last_assistant_text = Some(text.to_owned());
                    }
                }
                Some("tool_use") => self.add_tool_use(block),
                _ => {}
            }
        }
    }

    fn add_tool_use(&mut self, block: &Value) {
        let tool_use_id = text_at(block, "id").unwrap_or_default();
        if !self.tool_use_ids.insert(tool_use_id.to_owned()) {
            return;
        }
        self.facts.tool_uses += 1;
        let Some(input) = block.get("input") else {
            return;
        };
        match text_at(block, "name") {
            Some("Edit") => {
                self.facts.lines_added += piece_count(input, "new_string");
                self.facts.lines_removed += piece_count(input, "old_string");
            }
            Some("Write") => self.facts.lines_added += piece_count(input, "content"),
            Some("Skill") => {
                if let Some(skill) = text_at(input, "skill") {
                    self.skills.insert(skill.to_owned());
                }
            }
            Some("Read") => {
                if let Some(skill) = text_at(input, "file_path").and_then(skill_of_path) {
                    self.skill_reads.push(SkillRead {
                        tool_use_id: tool_use_id.to_owned(),
                        skill: skill.to_owned(),
                    });
                }
            }
            _ => {}
        }
    }

    fn add_user(&mut self, message: &Value) {
        if !self.has_first_user {
            self.has_first_user = true;
            self.facts.first_user_text = user_text(message);
        }
        for block in blocks_of(message) {
            let is_failed_result = text_at(block, "type") == Some("tool_result")
                && block.get("is_error") == Some(&Value::Bool(true));
            if is_failed_result && let Some(tool_use_id) = text_at(block, "tool_use_id") {
                self.failed_tool_use_ids.insert(tool_use_id.to_owned());
            }
        }
    }

    fn finish(mut self, bytes: u64) -> TranscriptFacts {
        for read in self.skill_reads {
            if !self.failed_tool_use_ids.contains(&read.tool_use_id) {
                self.skills.insert(read.skill);
            }
        }
        self.facts.requests = self.message_ids.len() as u64;
        self.facts.skills_opened = self.skills.into_iter().collect();
        self.facts.bytes = bytes;
        self.facts
    }
}

pub(super) fn text_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

pub(super) fn blocks_of(message: &Value) -> &[Value] {
    message
        .get("content")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

pub(super) fn usage_of(message: &Value) -> TokenCounts {
    let count = |key: &str| {
        message
            .get("usage")
            .and_then(|usage| usage.get(key))
            .and_then(Value::as_u64)
            .unwrap_or(0)
    };
    TokenCounts {
        input: count("input_tokens"),
        output: count("output_tokens"),
        cache_read: count("cache_read_input_tokens"),
        cache_creation: count("cache_creation_input_tokens"),
    }
}

/// Claude Code counts the pieces after a split on newlines, so a final newline adds one.
fn piece_count(input: &Value, key: &str) -> u64 {
    text_at(input, key).map_or(0, |text| text.split('\n').count() as u64)
}

/// The skill a read file belongs to. A `SKILL.md` belongs to the skill that the folder
/// holding it names. Any other Markdown file belongs to `<name>` when it is at or below
/// `<root>/<name>/` for one of [`SKILL_ROOTS`], and to no skill when it is anywhere else.
fn skill_of_path(path: &str) -> Option<&str> {
    if let Some(folder) = path.strip_suffix("/SKILL.md") {
        return folder.rsplit('/').next().filter(|name| !name.is_empty());
    }
    if !path.ends_with(".md") {
        return None;
    }
    let parts: Vec<&str> = path.split('/').collect();
    parts.windows(4).rev().find_map(|window| match *window {
        [parent, root, name, _]
            if SKILL_ROOTS.contains(&(parent, root))
                && !name.is_empty()
                && !name.starts_with('.') =>
        {
            Some(name)
        }
        _ => None,
    })
}

fn user_text(message: &Value) -> Option<String> {
    content_text(message.get("content")?)
}

/// The text of a message's or a tool result's `content`: plain text, or a list of
/// blocks of which some are text.
pub(super) fn content_text(content: &Value) -> Option<String> {
    match content {
        Value::String(text) => Some(text.clone()),
        Value::Array(blocks) => {
            let texts: Vec<&str> = blocks
                .iter()
                .filter(|block| text_at(block, "type") == Some("text"))
                .filter_map(|block| text_at(block, "text"))
                .collect();
            (!texts.is_empty()).then(|| texts.join("\n"))
        }
        _ => None,
    }
}
