//! `get`: the text and its hash, every refusal as not_found, and the start-up refusal of a missing folder.

use std::fs;
use std::process::Command;

use serde_json::json;

use super::{Server, copy_tree, fixture, sha256_hex};

#[test]
fn get_returns_the_text_and_the_sha256_of_that_text() {
    let mut server = Server::start(&[&fixture("lib-a")]);
    server.initialize();
    let (is_error, value) = server.call("get", &json!({ "skill": "alpha", "file": "sub/note.md" }));

    assert!(!is_error);
    assert_eq!(value["skill"], "alpha");
    assert_eq!(value["file"], "sub/note.md");
    let text = value["text"].as_str().unwrap();
    assert_eq!(text, "Note in alpha of library a.\n");
    assert_eq!(value["sha256"], sha256_hex(text.as_bytes()));
}

#[test]
fn get_refuses_every_bad_skill_or_file_as_not_found() {
    let mut server = Server::start(&[&fixture("lib-a")]);
    server.initialize();
    let cases = [
        ("missing", "SKILL.md"),
        ("alpha", "missing.md"),
        ("alpha", "../beta/SKILL.md"),
        ("alpha", "/etc/hosts"),
        ("alpha", "sub/../../beta/SKILL.md"),
        ("alpha", ""),
        ("alpha", "sub"),
        ("../x", "SKILL.md"),
        ("A", "SKILL.md"),
        ("not-a-skill", "README.md"),
    ];
    for (skill, file) in cases {
        let (code, _) = server.get_error_code(skill, file);
        assert_eq!(code, "not_found", "get {skill:?} {file:?}");
    }
}

#[test]
fn a_symlink_that_leaves_the_skill_folder_is_not_found_and_leaks_nothing() {
    let temp = tempfile::tempdir().unwrap();
    let library = temp.path().join("lib");
    copy_tree(&fixture("lib-a"), &library);
    let sentinel = temp.path().join("sentinel.txt");
    fs::write(&sentinel, "SENTINEL-SECRET").unwrap();
    std::os::unix::fs::symlink(&sentinel, library.join("alpha/link.md")).unwrap();
    std::os::unix::fs::symlink(temp.path(), library.join("alpha/dir-link")).unwrap();

    let mut server = Server::start(&[&library]);
    server.initialize();
    for file in ["link.md", "dir-link/sentinel.txt"] {
        let (code, answer) = server.get_error_code("alpha", file);
        assert_eq!(code, "not_found");
        assert!(!answer.contains("SENTINEL-SECRET"));
    }
    assert!(!server.head().to_string().contains("SENTINEL"));
}

#[test]
fn a_library_folder_that_does_not_exist_stops_the_command_with_one_error_line() {
    let temp = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_rex"))
        .args(["skills", "serve", "--library"])
        .arg(temp.path().join("nope"))
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
    assert!(stderr.starts_with("error: "), "{stderr}");
}
