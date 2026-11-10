//! A typed view of one logged hook event, holding only the fields that intake uses.
//! Claude Code adds events and fields over time, so an unknown one never fails here:
//! it gives `Other`, or `None` for the field.

use serde_json::Value;

use super::transcript::{content_text, text_at};

/// One hook event as intake sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Event {
    pub(crate) session_id: Option<String>,
    /// The main session's transcript. Every event names it, also one from a sub-agent.
    pub(crate) transcript_path: Option<String>,
    pub(crate) kind: EventKind,
}

/// What happened, for the events that intake uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EventKind {
    Start(Start),
    Stop(Stop),
    /// The end of a call of the sub-agent tool, `Agent` or `Task`.
    Call(Call),
    /// Any other event or tool, or an event without the agent it is about.
    Other,
}

/// A sub-agent started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Start {
    pub(crate) agent_id: String,
    pub(crate) agent_type: Option<String>,
}

/// A sub-agent stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Stop {
    pub(crate) agent_id: String,
    pub(crate) agent_type: Option<String>,
    /// The effort level the sub-agent ran at.
    pub(crate) effort: Option<String>,
    pub(crate) last_message: Option<String>,
    /// The sub-agent's own transcript.
    pub(crate) agent_transcript_path: Option<String>,
}

/// A call of the sub-agent tool returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Call {
    /// The agent that made the call. `None` means the main session.
    pub(crate) caller_agent_id: Option<String>,
    /// The sub-agent that the call started.
    pub(crate) agent_id: Option<String>,
    /// The prompt given to the sub-agent.
    pub(crate) brief: Option<String>,
    pub(crate) outcome: CallOutcome,
}

/// How the call of the sub-agent tool ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CallOutcome {
    /// A foreground sub-agent finished.
    Completed(Completion),
    /// A background sub-agent was launched and is still working.
    Launched { model: Option<String> },
    /// Any other status, or none.
    OtherStatus,
}

/// What a foreground call reports when its sub-agent finished.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Completion {
    pub(crate) model: Option<String>,
    pub(crate) duration_ms: Option<u64>,
    pub(crate) tool_uses: Option<u64>,
    pub(crate) lines_added: u64,
    pub(crate) lines_removed: u64,
    /// The text blocks of the reply, joined.
    pub(crate) final_text: Option<String>,
}

impl Event {
    /// The view of `event`, the object that Claude Code gave the hook.
    pub(crate) fn of(event: &Value) -> Event {
        let kind = match text_at(event, "hook_event_name") {
            Some("SubagentStart") => Start::of(event).map(EventKind::Start),
            Some("SubagentStop") => Stop::of(event).map(EventKind::Stop),
            Some("PostToolUse") => Call::of(event).map(EventKind::Call),
            _ => None,
        };
        Event {
            session_id: owned_text(event, "session_id"),
            transcript_path: owned_text(event, "transcript_path"),
            kind: kind.unwrap_or(EventKind::Other),
        }
    }
}

impl Start {
    fn of(event: &Value) -> Option<Start> {
        Some(Start {
            agent_id: owned_text(event, "agent_id")?,
            agent_type: owned_text(event, "agent_type"),
        })
    }
}

impl Stop {
    fn of(event: &Value) -> Option<Stop> {
        Some(Stop {
            agent_id: owned_text(event, "agent_id")?,
            agent_type: owned_text(event, "agent_type"),
            effort: event
                .get("effort")
                .and_then(|effort| owned_text(effort, "level")),
            last_message: owned_text(event, "last_assistant_message"),
            agent_transcript_path: owned_text(event, "agent_transcript_path"),
        })
    }
}

impl Call {
    /// `None` unless the tool is exactly `Agent` or `Task`: the hook's matcher is a
    /// pattern, and this is the one exact check.
    fn of(event: &Value) -> Option<Call> {
        if !matches!(text_at(event, "tool_name"), Some("Agent" | "Task")) {
            return None;
        }
        let input = event.get("tool_input").unwrap_or(&Value::Null);
        let response = event.get("tool_response").unwrap_or(&Value::Null);
        let outcome = match text_at(response, "status") {
            Some("completed") => CallOutcome::Completed(Completion::of(response)),
            Some("async_launched") => CallOutcome::Launched {
                model: owned_text(response, "resolvedModel"),
            },
            _ => CallOutcome::OtherStatus,
        };
        Some(Call {
            caller_agent_id: owned_text(event, "agent_id"),
            agent_id: owned_text(response, "agentId"),
            brief: owned_text(input, "prompt"),
            outcome,
        })
    }
}

impl Completion {
    fn of(response: &Value) -> Completion {
        // Claude Code leaves `toolStats` out when the sub-agent called no tool.
        let line_count = |key: &str| {
            response
                .get("toolStats")
                .and_then(|stats| stats.get(key))
                .and_then(Value::as_u64)
                .unwrap_or(0)
        };
        Completion {
            model: owned_text(response, "resolvedModel"),
            duration_ms: response.get("totalDurationMs").and_then(Value::as_u64),
            tool_uses: response.get("totalToolUseCount").and_then(Value::as_u64),
            lines_added: line_count("linesAdded"),
            lines_removed: line_count("linesRemoved"),
            final_text: response.get("content").and_then(content_text),
        }
    }
}

fn owned_text(value: &Value, key: &str) -> Option<String> {
    text_at(value, key).map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    const SESSION: &str = "9b2e7c41-3d5a-4f8e-a1c6-2f7d0e9b4a13";
    const MAIN_TRANSCRIPT: &str = "/home/user/.claude/projects/-home-user-work-repo/9b2e7c41-3d5a-4f8e-a1c6-2f7d0e9b4a13.jsonl";

    fn some(text: &str) -> Option<String> {
        Some(text.to_owned())
    }

    fn event_of(kind: EventKind) -> Event {
        Event {
            session_id: some(SESSION),
            transcript_path: some(MAIN_TRANSCRIPT),
            kind,
        }
    }

    fn agent_call(tool: &str, response: Value) -> Value {
        json!({
            "session_id": SESSION, "transcript_path": MAIN_TRANSCRIPT, "cwd": "/home/user/work/repo",
            "agent_id": "a0c8cde396de066a3",
            "hook_event_name": "PostToolUse", "tool_name": tool,
            "tool_input": { "description": "lead", "prompt": "Rex-Job: k3x9p2m7qd\nRex-Role: lead\n\nRead notes.txt.", "subagent_type": "model-medium-effort-low" },
            "tool_response": response
        })
    }

    fn call_of(outcome: CallOutcome) -> Event {
        event_of(EventKind::Call(Call {
            caller_agent_id: some("a0c8cde396de066a3"),
            agent_id: some("b111"),
            brief: some("Rex-Job: k3x9p2m7qd\nRex-Role: lead\n\nRead notes.txt."),
            outcome,
        }))
    }

    #[test]
    fn start() {
        let event = json!({"session_id": SESSION, "transcript_path": MAIN_TRANSCRIPT, "cwd": "/home/user/work/repo", "agent_id": "a0c8cde396de066a3", "agent_type": "model-medium-effort-low", "hook_event_name": "SubagentStart", "new_field": 1});
        assert_eq!(
            Event::of(&event),
            event_of(EventKind::Start(Start {
                agent_id: "a0c8cde396de066a3".to_owned(),
                agent_type: some("model-medium-effort-low"),
            }))
        );
    }

    #[test]
    fn stop() {
        let event = json!({"session_id": SESSION, "transcript_path": MAIN_TRANSCRIPT, "cwd": "/home/user/work/repo", "agent_id": "a0c8cde396de066a3", "agent_type": "model-medium-effort-low", "effort": {"level": "low"}, "hook_event_name": "SubagentStop", "agent_transcript_path": "/home/user/sub/agent-a0c8cde396de066a3.jsonl", "last_assistant_message": "LEAD DONE"});
        assert_eq!(
            Event::of(&event),
            event_of(EventKind::Stop(Stop {
                agent_id: "a0c8cde396de066a3".to_owned(),
                agent_type: some("model-medium-effort-low"),
                effort: some("low"),
                last_message: some("LEAD DONE"),
                agent_transcript_path: some("/home/user/sub/agent-a0c8cde396de066a3.jsonl"),
            }))
        );
    }

    #[test]
    fn completed_call() {
        let response = json!({"status": "completed", "agentId": "b111", "resolvedModel": "claude-sonnet-5-5", "totalDurationMs": 12333, "totalToolUseCount": 4, "toolStats": {"readCount": 1, "linesAdded": 4, "linesRemoved": 3}, "content": [{"type": "text", "text": "<tools-used>Edit</tools-used>\n"}, {"type": "image"}, {"type": "text", "text": "LEAD DONE"}]});
        let completion = Completion {
            model: some("claude-sonnet-5-5"),
            duration_ms: Some(12333),
            tool_uses: Some(4),
            lines_added: 4,
            lines_removed: 3,
            final_text: some("<tools-used>Edit</tools-used>\n\nLEAD DONE"),
        };
        assert_eq!(
            Event::of(&agent_call("Agent", response)),
            call_of(CallOutcome::Completed(completion))
        );
    }

    #[test]
    fn completed_call_with_no_tool_has_no_lines() {
        let response = json!({"status": "completed", "agentId": "b111", "totalToolUseCount": 0, "content": []});
        let completion = Completion {
            model: None,
            duration_ms: None,
            tool_uses: Some(0),
            lines_added: 0,
            lines_removed: 0,
            final_text: None,
        };
        assert_eq!(
            Event::of(&agent_call("Task", response)),
            call_of(CallOutcome::Completed(completion))
        );
    }

    #[test]
    fn launched_and_other_status() {
        let launched = json!({"status": "async_launched", "agentId": "b111", "resolvedModel": "claude-haiku-5-5"});
        assert_eq!(
            Event::of(&agent_call("Agent", launched)),
            call_of(CallOutcome::Launched {
                model: some("claude-haiku-5-5")
            })
        );
        let failed = json!({"status": "error", "agentId": "b111"});
        assert_eq!(
            Event::of(&agent_call("Agent", failed)),
            call_of(CallOutcome::OtherStatus)
        );
    }

    #[test]
    fn other_events_and_tools() {
        let response = json!({"status": "completed", "agentId": "b111"});
        assert_eq!(
            Event::of(&agent_call("ListAgents", response.clone())).kind,
            EventKind::Other
        );
        assert_eq!(
            Event::of(&agent_call("agent", response)).kind,
            EventKind::Other
        );
        let start_with_no_agent =
            json!({"session_id": SESSION, "hook_event_name": "SubagentStart"});
        assert_eq!(Event::of(&start_with_no_agent).kind, EventKind::Other);
        let unknown = json!({"session_id": SESSION, "transcript_path": MAIN_TRANSCRIPT, "hook_event_name": "Notification"});
        assert_eq!(Event::of(&unknown), event_of(EventKind::Other));
        assert_eq!(Event::of(&json!({})).session_id, None);
    }
}
