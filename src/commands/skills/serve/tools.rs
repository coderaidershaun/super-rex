//! The three library tools: `head`, `search` (stubbed) and `get`, behind one dispatch.

use serde_json::{Value, json};

use super::library::{Libraries, ReadError, sha256_hex};

const PROTOCOL: &str = "skills/1";

/// A failed call, as the `{ code, message }` text of an `isError` result.
struct ToolError {
    code: &'static str,
    message: String,
}

impl ToolError {
    fn new(code: &'static str, message: impl Into<String>) -> ToolError {
        ToolError {
            code,
            message: message.into(),
        }
    }
}

pub(super) fn list() -> Value {
    json!([
        {
            "name": "head",
            "description": "The library's name, version and size. Cheap; call it when a job opens.",
            "inputSchema": { "type": "object", "properties": {
                "include": { "type": "array", "items": { "type": "string", "enum": ["vocabulary"] } }
            } }
        },
        {
            "name": "search",
            "description": "Find skills for one or more request forms. Returns cards, never skill text.",
            "inputSchema": { "type": "object", "properties": {
                "requests": { "type": "array", "items": { "type": "object" } },
                "limit": { "type": "integer", "minimum": 1 },
                "cursor": { "type": ["string", "null"] },
                "kinds": { "type": ["array", "null"], "items": { "type": "string" } },
                "categories": { "type": ["array", "null"], "items": { "type": "string" } }
            }, "required": ["requests"] }
        },
        {
            "name": "get",
            "description": "One skill file and its SHA-256.",
            "inputSchema": { "type": "object", "properties": {
                "skill": { "type": "string" },
                "file": { "type": "string" }
            }, "required": ["skill", "file"] }
        }
    ])
}

/// Run the tool that a `tools/call` names and wrap its answer as a tool result.
pub(super) fn call(libraries: &Libraries, params: &Value) -> Value {
    let name = params.get("name").and_then(Value::as_str).unwrap_or("");
    let arguments = params.get("arguments").unwrap_or(&Value::Null);
    let outcome = match name {
        "head" => head(libraries),
        "search" => search(arguments),
        "get" => get(libraries, arguments),
        other => Err(ToolError::new("unknown_tool", format!("no tool {other}"))),
    };
    match outcome {
        Ok(value) => result(&value, false),
        Err(error) => result(
            &json!({ "code": error.code, "message": error.message }),
            true,
        ),
    }
}

fn result(value: &Value, is_error: bool) -> Value {
    json!({ "content": [{ "type": "text", "text": value.to_string() }], "isError": is_error })
}

/// The one function that the search job replaces.
fn search(_arguments: &Value) -> Result<Value, ToolError> {
    Err(ToolError::new(
        "not_implemented",
        "search is not implemented yet",
    ))
}

/// `version` is `sha256:` and the SHA-256 of this text, in UTF-8: one line
/// `<skill>/<file>`, a tab, the file's lower-case hex SHA-256 and a newline for
/// each served file, sorted by that path as bytes; then the line `vocabulary`,
/// a tab, the vocabulary version (empty when there is none) and a newline.
fn head(libraries: &Libraries) -> Result<Value, ToolError> {
    let unreadable =
        |error: &dyn std::fmt::Display| ToolError::new("unreadable", error.to_string());
    let files = libraries
        .served_files()
        .map_err(|error| unreadable(&error))?;
    let vocabulary = libraries
        .vocabulary_version()
        .map_err(|error| unreadable(&error))?;

    let mut preimage = String::new();
    let mut skills: Vec<&str> = Vec::new();
    for (path, sha) in &files {
        preimage.push_str(&format!("{path}\t{sha}\n"));
        let skill = path.split('/').next().unwrap_or("");
        if skills.last() != Some(&skill) {
            skills.push(skill);
        }
    }
    preimage.push_str(&format!(
        "vocabulary\t{}\n",
        vocabulary.as_deref().unwrap_or("")
    ));

    Ok(json!({
        "protocol": PROTOCOL,
        "library": libraries.name(),
        "version": format!("sha256:{}", sha256_hex(preimage.as_bytes())),
        "skills": skills.len(),
        "vocabulary": vocabulary,
    }))
}

fn get(libraries: &Libraries, arguments: &Value) -> Result<Value, ToolError> {
    let text_argument = |key: &str| {
        arguments
            .get(key)
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::new("bad_arguments", format!("`{key}` must be a string")))
    };
    let skill = text_argument("skill")?;
    let file = text_argument("file")?;
    let not_found = || ToolError::new("not_found", format!("no file {file:?} in skill {skill:?}"));

    let bytes = libraries.read(skill, file).map_err(|error| match error {
        ReadError::NotFound => not_found(),
        ReadError::Unreadable(message) => ToolError::new("unreadable", message),
    })?;
    let text = String::from_utf8(bytes)
        .map_err(|_| ToolError::new("unreadable", format!("{skill}/{file} is not UTF-8 text")))?;
    Ok(json!({ "skill": skill, "file": file, "sha256": sha256_hex(text.as_bytes()), "text": text }))
}
