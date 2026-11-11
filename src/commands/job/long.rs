//! The runs whose final message is long, so that `rex job close` shows which
//! agents handed their whole output back instead of a path and a summary.

use serde_json::{Value, json};

use crate::capture::JobRuns;

/// A final message longer than this, in characters, is flagged. Each line a
/// sub-agent returns stays in the session's context and is paid for on every
/// later call, so a report belongs in a file.
const MAX_FINAL_CHARS: usize = 2_000;

/// The role name printed for a run that carries no `Rex-Role` tag.
const NO_ROLE: &str = "-";

/// What the close found about the length of the runs' final messages.
pub(super) struct LongMessages {
    /// False when no run has a final message, and the list is then empty.
    has_messages: bool,
    /// In run order.
    long: Vec<LongMessage>,
}

struct LongMessage {
    /// `None` for a run with no `Rex-Role` tag.
    role: Option<String>,
    agent_id: String,
    chars: usize,
}

impl LongMessages {
    pub(super) fn read(view: &JobRuns) -> Self {
        let mut has_messages = false;
        let mut long = Vec::new();
        for run in &view.runs {
            let Some(message) = &run.final_message else {
                continue;
            };
            has_messages = true;
            let chars = message.chars().count();
            if chars > MAX_FINAL_CHARS {
                long.push(LongMessage {
                    role: run.role.clone(),
                    agent_id: run.agent_id.clone(),
                    chars,
                });
            }
        }
        LongMessages { has_messages, long }
    }

    /// The `long_messages=` lines of the text answer.
    pub(super) fn push_text(&self, text: &mut String) {
        if !self.has_messages {
            return;
        }
        text.push_str(&format!("long_messages={}\n", self.long.len()));
        for (number, long) in self.long.iter().enumerate() {
            text.push_str(&format!(
                "long_message.{}={} {} chars={}\n",
                number + 1,
                long.role.as_deref().unwrap_or(NO_ROLE),
                long.agent_id,
                long.chars,
            ));
        }
    }

    pub(super) fn json(&self) -> Value {
        self.long
            .iter()
            .map(|long| json!({"role": long.role, "agent": long.agent_id, "chars": long.chars}))
            .collect()
    }
}
