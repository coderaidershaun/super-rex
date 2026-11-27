//! Two libraries in priority order: a skill name in the first hides the whole skill in the second.

use serde_json::json;

use super::{Server, fixture};

#[test]
fn the_first_library_hides_every_file_of_a_skill_with_the_same_name() {
    let mut server = Server::start(&[&fixture("lib-a"), &fixture("lib-b")]);
    server.initialize();

    let (is_error, value) = server.call("get", &json!({ "skill": "alpha", "file": "SKILL.md" }));
    assert!(!is_error);
    assert!(value["text"].as_str().unwrap().contains("# Alpha A"));

    let (code, _) = server.get_error_code("alpha", "only-in-b.md");
    assert_eq!(code, "not_found");

    let (is_error, value) = server.call("get", &json!({ "skill": "gamma", "file": "SKILL.md" }));
    assert!(!is_error, "{value}");
}

#[test]
fn head_counts_each_skill_name_once_and_names_the_first_library() {
    let mut server = Server::start(&[&fixture("lib-a"), &fixture("lib-b")]);
    server.initialize();
    let head = server.head();

    assert_eq!(head["skills"], 3);
    assert_eq!(head["library"], "lib-a");
}
