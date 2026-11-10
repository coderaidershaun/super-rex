//! Integration tests of `rex commands` and `rex help`.

use assert_cmd::Command;

#[test]
fn commands_lists_every_subcommand() {
    let assert = Command::cargo_bin("rex")
        .expect("find rex binary")
        .arg("commands")
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8 stdout");
    assert!(stdout.contains("skills"));
    assert!(stdout.contains("codebase"));
    assert!(stdout.contains("smells"));
    assert!(stdout.contains("commands"));
}

#[test]
fn help_lists_every_subcommand() {
    let assert = Command::cargo_bin("rex")
        .expect("find rex binary")
        .arg("help")
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8 stdout");
    assert!(stdout.contains("skills"));
    assert!(stdout.contains("codebase"));
    assert!(stdout.contains("smells"));
    assert!(stdout.contains("commands"));
}
