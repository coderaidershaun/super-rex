//! Integration tests of `rex skills list` and `rex skills check`. Every run
//! sets `HOME` to a temporary folder, so the user's own skills are never read.

mod check;
mod list;

use std::fs;
use std::path::Path;
use std::process::Output;

use assert_cmd::Command;

pub(super) fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().expect("file has a folder")).expect("make folder");
    fs::write(path, text).expect("write file");
}

pub(super) fn rex_skills(cwd: &Path, home: &Path, arguments: &[&str]) -> Output {
    Command::cargo_bin("rex")
        .expect("find rex binary")
        .current_dir(cwd)
        .env("HOME", home)
        .arg("skills")
        .args(arguments)
        .output()
        .expect("run rex skills")
}

pub(super) fn text_of(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("utf8 output")
}

pub(super) fn stdout_of_success(cwd: &Path, home: &Path, arguments: &[&str]) -> String {
    let output = rex_skills(cwd, home, arguments);
    assert_eq!(
        output.status.code(),
        Some(0),
        "rex skills {arguments:?}: {}",
        text_of(&output.stderr)
    );
    text_of(&output.stdout)
}
