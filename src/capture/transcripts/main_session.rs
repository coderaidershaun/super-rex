//! Reads the main session's transcript for one job: the tokens spent inside the
//! job's time, and the model and effort of the builder that opened the job.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde_json::Value;

use super::{blocks_of, content_text, read_file, records, text_at, usage_of};
use crate::capture::error::CaptureError;
use crate::capture::price::{Cost, cost_of};
use crate::capture::runs::MainFacts;
use crate::capture::tokens::TokenCounts;
use crate::store::JobId;

/// The form of every time the store holds. Each `0` stands for one digit.
const STORE_TIME_FORM: &[u8; 24] = b"0000-00-00T00:00:00.000Z";

/// Read the main transcript at `path`. A request is counted when the first record
/// of its message ID has a time in `[from, to)`; `to` is `None` while the job is
/// open. Times are compared as text, which is right because every time has the
/// store's one form; a record whose time has another form is counted as damaged.
pub(crate) fn read_main_transcript(
    path: &Path,
    job: &JobId,
    from: &str,
    to: Option<&str>,
) -> Result<Option<MainFacts>, CaptureError> {
    let Some(contents) = read_file(path)? else {
        return Ok(None);
    };
    let mut reader = MainReader {
        job_line: format!("job={}", job.as_str()),
        from,
        to,
        facts: MainFacts {
            tokens: TokenCounts::default(),
            cost: Cost::default(),
            damaged_lines: 0,
            bytes: contents.len() as u64,
            builder_model: None,
            builder_effort: None,
        },
        message_ids: HashSet::new(),
        bash_calls: HashMap::new(),
    };
    for record in records(&contents) {
        match record {
            Some(record) => reader.add(&record),
            None => reader.facts.damaged_lines += 1,
        }
    }
    Ok(Some(reader.facts))
}

/// Facts gathered one record at a time. The builder is known only when the result
/// of its `Bash` call comes, in a later record.
struct MainReader<'a> {
    /// The first line that `rex job create` printed for this job.
    job_line: String,
    from: &'a str,
    to: Option<&'a str>,
    facts: MainFacts,
    message_ids: HashSet<String>,
    /// The model and effort of the record that made each `Bash` call, by call ID.
    bash_calls: HashMap<String, (Option<String>, Option<String>)>,
}

impl MainReader<'_> {
    fn add(&mut self, record: &Value) {
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
        if let Some(id) = text_at(message, "id") {
            let Some(at) = text_at(record, "timestamp").filter(|at| is_store_time(at)) else {
                self.facts.damaged_lines += 1;
                return;
            };
            if self.message_ids.insert(id.to_owned()) && self.is_inside(at) {
                self.facts.tokens += usage_of(message);
                self.facts.cost += cost_of(message);
            }
        }
        for block in blocks_of(message) {
            if text_at(block, "type") == Some("tool_use")
                && text_at(block, "name") == Some("Bash")
                && let Some(call_id) = text_at(block, "id")
            {
                let model = text_at(message, "model").map(str::to_owned);
                let effort = text_at(record, "effort").map(str::to_owned);
                self.bash_calls.insert(call_id.to_owned(), (model, effort));
            }
        }
    }

    fn add_user(&mut self, message: &Value) {
        if self.facts.builder_model.is_some() || self.facts.builder_effort.is_some() {
            return;
        }
        for block in blocks_of(message) {
            if text_at(block, "type") != Some("tool_result") {
                continue;
            }
            let Some(call) = text_at(block, "tool_use_id").and_then(|id| self.bash_calls.get(id))
            else {
                continue;
            };
            let output = block.get("content").and_then(content_text);
            if output.is_some_and(|output| self.opened_the_job(&output)) {
                (self.facts.builder_model, self.facts.builder_effort) = call.clone();
                return;
            }
        }
    }

    fn is_inside(&self, at: &str) -> bool {
        at >= self.from && self.to.is_none_or(|to| at < to)
    }

    /// True for the output of the `rex job create` that made this job. A create that
    /// found the session's job already open prints `existing=yes`; it opened nothing.
    fn opened_the_job(&self, output: &str) -> bool {
        let mut lines = output.lines().map(str::trim);
        lines.next() == Some(self.job_line.as_str()) && !lines.any(|line| line == "existing=yes")
    }
}

fn is_store_time(text: &str) -> bool {
    text.len() == STORE_TIME_FORM.len()
        && text
            .bytes()
            .zip(STORE_TIME_FORM)
            .all(|(byte, form)| match form {
                b'0' => byte.is_ascii_digit(),
                _ => byte == *form,
            })
}
