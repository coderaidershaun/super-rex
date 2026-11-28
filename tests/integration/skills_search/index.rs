//! The index that the loader builds from the fixture library, seen through
//! `rex skills index`: one entry per skill and per sibling file.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

use crate::skills_serve::{copy_tree, sha256_hex};

fn main_library() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/skills-search")
}

fn run_index(libraries: &[&Path]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rex"));
    command.args(["skills", "index"]);
    for library in libraries {
        command.arg("--library").arg(library);
    }
    command.output().expect("run rex skills index")
}

fn index_document(libraries: &[&Path]) -> Value {
    let output = run_index(libraries);
    assert!(
        output.status.success(),
        "rex skills index failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("index.json is JSON")
}

fn skills(document: &Value) -> &Vec<Value> {
    document["skills"].as_array().expect("skills is a list")
}

fn skill<'a>(document: &'a Value, name: &str) -> &'a Value {
    skills(document)
        .iter()
        .find(|skill| skill["name"] == name)
        .unwrap_or_else(|| panic!("{name} is not in the index"))
}

fn names(document: &Value) -> Vec<&str> {
    skills(document)
        .iter()
        .map(|skill| skill["name"].as_str().expect("name is text"))
        .collect()
}

const FIXTURE_SKILLS: [&str; 15] = [
    "fx-coding-rules",
    "fx-debugging",
    "fx-design-sketch",
    "fx-diagrams",
    "fx-failure-policy",
    "fx-gap-ts",
    "fx-latency",
    "fx-old-diagrams",
    "fx-proptest",
    "fx-release-notes",
    "fx-test-doubles",
    "fx-test-first",
    "fx-ts-errors",
    "fx-unit-tests",
    "fx-validation-plan",
];

#[test]
fn index_has_an_entry_for_each_skill_and_each_file() {
    let library = main_library();

    let document = index_document(&[&library]);

    assert_eq!(names(&document), FIXTURE_SKILLS);
    let file_count = |name: &str| skill(&document, name)["files"].as_array().map(Vec::len);
    assert_eq!(file_count("fx-latency"), Some(5));
    assert_eq!(file_count("fx-coding-rules"), Some(2));
    assert_eq!(file_count("fx-release-notes"), Some(0));
    assert!(
        skill(&document, "fx-release-notes")
            .get("metadata")
            .is_none()
    );
    let libraries: Vec<&Value> = skills(&document)
        .iter()
        .map(|skill| &skill["library"])
        .collect();
    assert!(libraries.iter().all(|library| *library == "skills-search"));
}

#[test]
fn card_is_cut_to_thirty_words() {
    let library = main_library();

    let document = index_document(&[&library]);

    let latency = skill(&document, "fx-latency");
    let description = latency["description"].as_str().expect("description");
    let card = latency["card"].as_str().expect("card");
    assert_eq!(description.split_whitespace().count(), 34);
    let first_thirty: Vec<&str> = description.split_whitespace().take(30).collect();
    assert_eq!(card, first_thirty.join(" "));
    assert!(card.ends_with("tens of microseconds"), "card: {card}");
    for entry in skills(&document) {
        let words = entry["card"].as_str().expect("card").split_whitespace();
        assert!(words.count() <= 30, "card of {} is too long", entry["name"]);
    }
}

#[test]
fn index_file_names_each_skill_with_type_url_and_digest() {
    let library = main_library();

    let document = index_document(&[&library]);

    assert_eq!(document["protocol"], "skills/1");
    assert_eq!(document["library"], "skills-search");
    assert_eq!(document["vocabulary"], Value::Null);
    for entry in skills(&document) {
        let name = entry["name"].as_str().expect("name");
        let bytes = fs::read(library.join(name).join("SKILL.md")).expect("read SKILL.md");
        assert_eq!(entry["type"], "skill-md", "{name}");
        assert!(entry["description"].is_string(), "{name}");
        assert_eq!(entry["url"], format!("{name}/SKILL.md"));
        assert_eq!(entry["digest"], format!("sha256:{}", sha256_hex(&bytes)));
    }
}

#[test]
fn malformed_header_gives_an_entry_with_no_metadata() {
    let temp = tempfile::tempdir().expect("temp folder");
    let copy = temp.path().join("skills-search");
    copy_tree(&main_library(), &copy);
    fs::create_dir(copy.join("fx-broken")).expect("make skill folder");
    fs::write(
        copy.join("fx-broken/SKILL.md"),
        "---\nname: fx-broken\ndescription: A header with a tab.\nmetadata:\n\tkind: rule\n---\nBody.\n",
    )
    .expect("write SKILL.md");

    let output = run_index(&[&copy]);

    assert!(output.status.success(), "exit: {:?}", output.status);
    let document: Value = serde_json::from_slice(&output.stdout).expect("JSON");
    let mut expected_names = FIXTURE_SKILLS.to_vec();
    expected_names.insert(0, "fx-broken");
    assert_eq!(names(&document), expected_names);
    assert!(skill(&document, "fx-broken").get("metadata").is_none());
    let original = index_document(&[&main_library()]);
    for name in FIXTURE_SKILLS {
        assert_eq!(skill(&document, name), skill(&original, name), "{name}");
    }
}

#[test]
fn unreadable_skill_file_stops_the_index_with_one_line() {
    let temp = tempfile::tempdir().expect("temp folder");
    let copy = temp.path().join("skills-search");
    copy_tree(&main_library(), &copy);
    let skill_file = copy.join("fx-test-first/SKILL.md");
    fs::set_permissions(&skill_file, fs::Permissions::from_mode(0o000)).expect("chmod 000");
    if fs::read(&skill_file).is_ok() {
        eprintln!("this user can read a file of mode 000, so the test asserts nothing");
        return;
    }

    let output = run_index(&[&copy]);

    fs::set_permissions(&skill_file, fs::Permissions::from_mode(0o644)).expect("chmod back");
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    let error_lines: Vec<&str> = stderr
        .lines()
        .filter(|line| line.starts_with("error:"))
        .collect();
    assert_eq!(error_lines.len(), 1, "stderr: {stderr}");
    assert!(
        error_lines[0].contains("fx-test-first"),
        "{}",
        error_lines[0]
    );
}
