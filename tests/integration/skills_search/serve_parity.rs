//! The `search` tool of `rex skills serve` and `rex skills search` give the
//! same text for the same arguments, errors included.

use std::fs;
use std::os::unix::fs::PermissionsExt;

use serde_json::{Value, json};

use super::answer::{go_service, main_library, run_search, rust_loop};
use crate::skills_serve::{Server, copy_tree};

/// The raw text of a `search` tool result and its `isError` flag.
fn tool_text(server: &mut Server, arguments: &Value) -> (bool, String) {
    let answer = server.request(
        "tools/call",
        &json!({ "name": "search", "arguments": arguments }),
    );
    let result = &answer["result"];
    let text = result["content"][0]["text"].as_str().expect("text content");
    (result["isError"] == true, text.to_owned())
}

#[test]
fn cli_and_serve_tool_give_the_same_answer() {
    let arguments = json!({ "requests": [rust_loop(), go_service()] });
    let garbled = json!({ "requests": [rust_loop()], "cursor": "eyJvIjoxMH0" });
    let library = main_library();

    let output = run_search(std::slice::from_ref(&library), &arguments.to_string());
    let mut server = Server::start(&[&library]);
    server.initialize();
    let (is_error, text) = tool_text(&mut server, &arguments);
    let (garbled_is_error, garbled_text) = tool_text(&mut server, &garbled);

    assert!(output.status.success());
    let line = String::from_utf8(output.stdout).expect("UTF-8");
    assert!(!is_error);
    assert_eq!(format!("{text}\n"), line);
    assert!(garbled_is_error);
    let garbled: Value = serde_json::from_str(&garbled_text).expect("JSON");
    assert_eq!(garbled["code"], "bad_cursor");
    let not_a_form = json!({ "requests": [{ "id": ["T1"] }] });
    let (bad_is_error, bad_text) = tool_text(&mut server, &not_a_form);
    let bad_output = run_search(std::slice::from_ref(&library), &not_a_form.to_string());
    let bad: Value = serde_json::from_str(&bad_text).expect("JSON");
    assert!(bad_is_error);
    assert_eq!(bad["code"], "bad_arguments");
    let message = bad["message"].as_str().expect("message");
    assert_eq!(
        String::from_utf8_lossy(&bad_output.stderr),
        format!("error: {message}\n")
    );
    let head = server.head();
    let answer: Value = serde_json::from_str(&text).expect("JSON");
    assert_eq!(answer["index_version"], head["version"]);
}

#[test]
fn unreadable_skill_file_gives_unreadable_through_the_serve_tool() {
    let temp = tempfile::tempdir().expect("temp folder");
    let copy = temp.path().join("skills-search");
    copy_tree(&main_library(), &copy);
    let skill_file = copy.join("fx-test-first/SKILL.md");
    fs::set_permissions(&skill_file, fs::Permissions::from_mode(0o000)).expect("chmod 000");
    if fs::read(&skill_file).is_ok() {
        eprintln!("this user can read a file of mode 000, so the test asserts nothing");
        return;
    }
    let mut server = Server::start(&[&copy]);
    server.initialize();

    let (is_error, tool) = server.call("search", &json!({ "requests": [rust_loop()] }));

    fs::set_permissions(&skill_file, fs::Permissions::from_mode(0o644)).expect("chmod back");
    assert!(is_error);
    assert_eq!(tool["code"], "unreadable");
    let message = tool["message"].as_str().expect("message");
    assert!(message.contains("fx-test-first"), "{message}");
}
