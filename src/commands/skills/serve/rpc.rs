//! JSON-RPC 2.0 framing: turns one input line into at most one answer line.

use serde_json::{Value, json};

use super::library::Libraries;
use super::tools;

const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;

const SERVER_NAME: &str = "super-rex-skills";
const DEFAULT_PROTOCOL: &str = "2025-06-18";

/// The answer to one line, or `None` for a blank line or a notification.
pub(super) fn answer(libraries: &Libraries, line: &str) -> Option<Value> {
    if line.trim().is_empty() {
        return None;
    }
    let Ok(message) = serde_json::from_str::<Value>(line) else {
        return Some(error(&Value::Null, PARSE_ERROR, "the line is not JSON"));
    };
    let Some(method) = message.get("method").and_then(Value::as_str) else {
        return Some(error(
            &Value::Null,
            INVALID_REQUEST,
            "no method in the request",
        ));
    };
    // A message with no id is a notification and gets no answer.
    let id = message.get("id")?;
    if !(id.is_string() || id.is_number()) {
        return Some(error(
            &Value::Null,
            INVALID_REQUEST,
            "the id must be a string or a number",
        ));
    }
    let params = message.get("params").cloned().unwrap_or(Value::Null);
    Some(match method {
        "initialize" => success(id, &initialize(&params)),
        "ping" => success(id, &json!({})),
        "tools/list" => success(id, &json!({ "tools": tools::list() })),
        "tools/call" => success(id, &tools::call(libraries, &params)),
        other => error(id, METHOD_NOT_FOUND, &format!("no method {other}")),
    })
}

fn initialize(params: &Value) -> Value {
    let protocol = params
        .get("protocolVersion")
        .and_then(Value::as_str)
        .unwrap_or(DEFAULT_PROTOCOL);
    json!({
        "protocolVersion": protocol,
        "capabilities": { "tools": {} },
        "serverInfo": { "name": SERVER_NAME, "version": env!("CARGO_PKG_VERSION") },
    })
}

fn success(id: &Value, result: &Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error(id: &Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}
