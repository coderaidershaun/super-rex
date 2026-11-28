//! `head`: its five fields, the version rules, the vocabulary file and the embedded library.

use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::json;

use super::{Server, copy_tree, fixture};

fn head_of(library: &Path) -> serde_json::Value {
    let mut server = Server::start(&[library]);
    server.initialize();
    server.head()
}

#[test]
fn head_answers_its_five_fields() {
    let head = head_of(&fixture("lib-a"));

    assert_eq!(head["protocol"], "skills/1");
    assert_eq!(head["library"], "lib-a");
    assert_eq!(head["skills"], 2);
    assert_eq!(head["vocabulary"], json!(null));
    let version = head["version"].as_str().unwrap();
    let digits = version.strip_prefix("sha256:").expect("sha256: prefix");
    assert_eq!(digits.len(), 64);
    assert!(
        digits
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    );
}

#[test]
fn the_version_is_stable_and_changes_with_one_byte_of_one_file() {
    let temp = tempfile::tempdir().unwrap();
    let library = temp.path().join("lib");
    copy_tree(&fixture("lib-a"), &library);

    let first = head_of(&library)["version"].clone();
    assert_eq!(head_of(&library)["version"], first);

    fs::write(
        library.join("alpha/sub/note.md"),
        "Note in alpha of library b.\n",
    )
    .unwrap();
    assert_ne!(head_of(&library)["version"], first);
}

#[test]
fn the_version_is_known_for_a_fixed_folder() {
    // sha256 of "alpha/SKILL.md\t<hash>\nalpha/sub/note.md\t<hash>\nbeta/SKILL.md\t<hash>\nvocabulary\t\n"
    let mut preimage = String::new();
    for path in ["alpha/SKILL.md", "alpha/sub/note.md", "beta/SKILL.md"] {
        let bytes = fs::read(fixture("lib-a").join(path)).unwrap();
        preimage += &format!("{path}\t{}\n", super::sha256_hex(&bytes));
    }
    preimage += "vocabulary\t\n";

    assert_eq!(
        head_of(&fixture("lib-a"))["version"],
        format!("sha256:{}", super::sha256_hex(preimage.as_bytes()))
    );
}

#[test]
fn with_no_library_the_embedded_one_is_served() {
    let mut server = Server::start(&[]);
    server.initialize();
    let head = server.head();

    assert_eq!(head["library"], "super-rex");
    assert_eq!(head["skills"], 25);
    assert_eq!(head["vocabulary"], "2");
}

#[test]
fn a_folder_copy_of_the_plugin_library_has_the_embedded_version() {
    let temp = tempfile::tempdir().unwrap();
    let library = temp.path().join("library");
    let plugin = Path::new(env!("CARGO_MANIFEST_DIR")).join("plugin");
    copy_tree(&plugin.join("library"), &library);
    copy_tree(&plugin.join("vocab"), &temp.path().join("vocab"));
    fs::write(library.join("rex-code-tdd/.DS_Store"), "junk").unwrap();
    fs::create_dir_all(library.join(".hidden-skill")).unwrap();
    fs::write(library.join(".hidden-skill/SKILL.md"), "hidden").unwrap();

    let embedded = head_of_embedded();
    let copy = head_of(&library);
    assert_eq!(copy["version"], embedded["version"]);
    assert_eq!(copy["skills"], embedded["skills"]);
}

#[test]
fn a_vocabulary_file_beside_the_library_sets_vocabulary_and_changes_the_version() {
    let temp = tempfile::tempdir().unwrap();
    let library = temp.path().join("lib");
    copy_tree(&fixture("lib-a"), &library);
    let without = head_of(&library);

    fs::create_dir_all(temp.path().join("vocab")).unwrap();
    fs::write(
        temp.path().join("vocab/terms.json"),
        r#"{"version":"1","terms":[]}"#,
    )
    .unwrap();
    let with = head_of(&library);

    assert_eq!(with["vocabulary"], "1");
    assert_ne!(with["version"], without["version"]);
}

#[test]
fn an_invalid_vocabulary_file_stops_the_server_with_one_error_line() {
    for bad in ["{ not json", r#"{"terms":[]}"#, r#"{"version":1}"#] {
        let temp = tempfile::tempdir().unwrap();
        let library = temp.path().join("lib");
        copy_tree(&fixture("lib-a"), &library);
        fs::create_dir_all(temp.path().join("vocab")).unwrap();
        fs::write(temp.path().join("vocab/terms.json"), bad).unwrap();

        let output = Command::new(env!("CARGO_BIN_EXE_rex"))
            .args(["skills", "serve", "--library"])
            .arg(&library)
            .output()
            .unwrap();

        assert!(!output.status.success(), "{bad}");
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert_eq!(stderr.lines().count(), 1, "{stderr}");
        assert!(stderr.starts_with("error: "), "{stderr}");
        assert!(stderr.contains("terms.json"), "{stderr}");
    }
}

fn head_of_embedded() -> serde_json::Value {
    let mut server = Server::start(&[]);
    server.initialize();
    server.head()
}
