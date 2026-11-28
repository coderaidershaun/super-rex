//! Pages of `rex skills search`: `limit`, `more`, and a cursor that gives
//! exactly the rest, or a typed error when it cannot.

use std::fs::OpenOptions;
use std::io::Write;

use serde_json::{Value, json};

use super::answer::{main_library, result, run_search, rust_loop, skills};
use crate::skills_serve::{Server, copy_tree};

fn first_page_cursor(result: &Value) -> String {
    result["cursor"].as_str().expect("a cursor").to_owned()
}

#[test]
fn cursor_returns_exactly_the_items_the_limit_held_back() {
    let libraries = [main_library()];

    let first = result(
        &libraries,
        &json!({ "requests": [rust_loop()], "limit": 2, "token_budget": 10_000 }),
    );
    let cursor = first_page_cursor(&first);
    let second = result(
        &libraries,
        &json!({ "requests": [rust_loop()], "limit": 2, "token_budget": 10_000, "cursor": cursor }),
    );
    let whole = result(
        &libraries,
        &json!({ "requests": [rust_loop()], "limit": 10, "token_budget": 10_000 }),
    );

    assert_eq!(skills(&first), ["fx-coding-rules", "fx-latency"]);
    assert_eq!(first["more"], 1);
    let parts: Vec<&str> = cursor.split('.').collect();
    assert_eq!(parts[..2], ["c1", "2"]);
    assert_eq!(parts[2].len(), 16);
    assert_eq!(parts[3].len(), 16);
    assert_eq!(skills(&second), ["fx-test-first"]);
    assert_eq!(second["more"], 0);
    assert_eq!(second["cursor"], Value::Null);
    let mut both_pages = skills(&first);
    both_pages.extend(skills(&second));
    assert_eq!(both_pages, skills(&whole));
}

#[test]
fn cursor_from_before_a_change_is_stale() {
    let copy = tempfile::tempdir().expect("temp folder");
    let library = copy.path().join("skills-search");
    copy_tree(&main_library(), &library);
    let first = result(
        std::slice::from_ref(&library),
        &json!({ "requests": [rust_loop()], "limit": 2, "token_budget": 10_000 }),
    );
    let next = json!({ "requests": [rust_loop()], "limit": 2, "token_budget": 10_000, "cursor": first_page_cursor(&first) });
    let mut skill_file = OpenOptions::new()
        .append(true)
        .open(library.join("fx-test-first/SKILL.md"))
        .expect("open SKILL.md");
    writeln!(skill_file, "One more line.").expect("append a line");

    let output = run_search(std::slice::from_ref(&library), &next.to_string());
    let mut server = Server::start(&[&library]);
    server.initialize();
    let (is_error, tool) = server.call("search", &next);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr.trim_end(),
        "error: the library changed since the cursor was made; search again without it"
    );
    assert!(is_error);
    assert_eq!(tool["code"], "stale_cursor");
}

#[test]
fn garbled_cursor_is_bad() {
    let arguments = json!({ "requests": [rust_loop()], "cursor": "eyJvIjoxMH0" });

    let output = run_search(&[main_library()], &arguments.to_string());
    let mut server = Server::start(&[&main_library()]);
    server.initialize();
    let (is_error, tool) = server.call("search", &arguments);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty(), "no item is printed");
    assert!(
        stderr.starts_with("error: the cursor cannot be read"),
        "{stderr}"
    );
    assert!(is_error);
    assert_eq!(tool["code"], "bad_cursor");
}

#[test]
fn a_huge_limit_after_a_cursor_gives_the_rest() {
    let libraries = [main_library()];
    let first = result(
        &libraries,
        &json!({ "requests": [rust_loop()], "limit": 2, "token_budget": 10_000 }),
    );
    let rest = json!({ "requests": [rust_loop()], "limit": u64::MAX, "token_budget": 10_000,
                       "cursor": first_page_cursor(&first) });

    let second = result(&libraries, &rest);
    let mut server = Server::start(&[&main_library()]);
    server.initialize();
    let (is_error, tool) = server.call("search", &rest);
    let head = server.head();

    assert_eq!(skills(&second), ["fx-test-first"]);
    assert!(!is_error, "{tool}");
    assert_eq!(skills(&tool["results"][0]), ["fx-test-first"]);
    assert!(head["version"].is_string(), "the server still answers");
}

#[test]
fn cursor_accepts_the_same_facts_in_another_order() {
    let libraries = [main_library()];
    let reordered = json!({ "id": "T1",
            "facts": { "latency_scale": "ns_us", "languages": ["rust"], "writes_code": true },
            "activity": "build", "touches": ["threads"], "must_have": ["latency"] });
    let first = result(
        &libraries,
        &json!({ "requests": [rust_loop()], "limit": 2, "token_budget": 10_000 }),
    );

    let second = result(
        &libraries,
        &json!({ "requests": [reordered], "limit": 2, "token_budget": 10_000, "cursor": first_page_cursor(&first) }),
    );

    assert_eq!(skills(&second), ["fx-test-first"]);
}
