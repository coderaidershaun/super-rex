//! `rex skills check` passes a skill whose header nests maps and lists of maps
//! under `metadata`, in a plugin and in a project, and still refuses broken headers.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use assert_cmd::Command;
use yaml_rust2::{Yaml, YamlLoader};

/// Every field of the proposed `metadata` map, in the order the fixture writes them.
const METADATA_FIELDS: [&str; 12] = [
    "kind",
    "category",
    "claims",
    "placement",
    "scope",
    "not_when",
    "intents",
    "files",
    "open_when",
    "requires",
    "replaces",
    "max_topics",
];

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixtures() -> PathBuf {
    repo_root().join("tests/fixtures/wp0-metadata")
}

fn metadata_skill() -> PathBuf {
    fixtures().join("metadata-plugin/skills/wp0-metadata")
}

/// Runs `rex skills check` from `cwd` with an empty home folder, so the
/// user's own skills are never read.
fn skills_check(cwd: &Path, paths: &[&str]) -> Output {
    let home = tempfile::tempdir().expect("temp home");
    Command::cargo_bin("rex")
        .expect("find rex binary")
        .current_dir(cwd)
        .env("HOME", home.path())
        .args(["skills", "check"])
        .args(paths)
        .output()
        .expect("run rex skills check")
}

fn text_of(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("utf8 output")
}

fn metadata_of_fixture() -> Yaml {
    let text = fs::read_to_string(metadata_skill().join("SKILL.md")).expect("read fixture skill");
    let header = text
        .strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---\n"))
        .map(|(header, _)| header)
        .expect("fixture skill has a header");
    let documents = YamlLoader::load_from_str(header).expect("fixture header is YAML");
    documents[0]["metadata"].clone()
}

#[test]
fn skills_check_passes_the_metadata_fixture_and_its_baseline_in_a_plugin() {
    let output = skills_check(
        &fixtures(),
        &[
            "metadata-plugin/skills/wp0-metadata",
            "baseline-plugin/skills/wp0-metadata",
        ],
    );

    assert_eq!(text_of(&output.stderr), "");
    assert_eq!(
        text_of(&output.stdout),
        "ok metadata-plugin/skills/wp0-metadata\nok baseline-plugin/skills/wp0-metadata\n"
    );
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn skills_check_passes_the_metadata_fixture_in_a_project_claude_skills_folder() {
    let temp = tempfile::tempdir().expect("temp dir");
    let repo = temp.path();
    fs::create_dir_all(repo.join(".git")).expect("make .git");
    let copy = repo.join(".claude/skills/wp0-metadata");
    fs::create_dir_all(&copy).expect("make project skill folder");
    for entry in fs::read_dir(metadata_skill()).expect("read fixture skill folder") {
        let entry = entry.expect("read fixture entry");
        fs::copy(entry.path(), copy.join(entry.file_name())).expect("copy fixture file");
    }

    let output = skills_check(repo, &[".claude/skills/wp0-metadata"]);

    assert_eq!(text_of(&output.stderr), "");
    assert_eq!(text_of(&output.stdout), "ok .claude/skills/wp0-metadata\n");
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn skills_check_refuses_each_broken_header_of_the_control_fixture() {
    let output = skills_check(&fixtures(), &["broken-plugin/skills"]);

    let stdout = text_of(&output.stdout);
    let findings: Vec<&str> = stdout
        .lines()
        .map(|line| line.split(": the header").next().unwrap_or(line))
        .collect();
    assert_eq!(
        findings,
        [
            "broken-plugin/skills/wp0-broken/SKILL.md: H2",
            "broken-plugin/skills/wp0-unclosed/SKILL.md: H2",
            "broken-plugin/skills/wp0-unterminated/SKILL.md: H2",
        ],
        "{stdout}"
    );
    assert_eq!(
        text_of(&output.stderr),
        "error: 3 findings in 3 of 3 skills\n"
    );
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn metadata_fixture_nests_every_field_and_names_only_real_files_and_skills() {
    let metadata = metadata_of_fixture();
    let fields: Vec<&str> = metadata
        .as_hash()
        .expect("metadata is a map")
        .keys()
        .map(|key| key.as_str().expect("metadata key is text"))
        .collect();
    assert_eq!(fields, METADATA_FIELDS);
    assert!(
        metadata["claims"]["facts"].as_hash().is_some(),
        "claims.facts must be a nested map"
    );
    assert!(
        metadata["scope"].as_hash().is_some(),
        "scope must be a nested map"
    );

    let files = metadata["files"].as_vec().expect("files is a list");
    for file in files {
        let path = file["path"]
            .as_str()
            .expect("each file is a map with a path");
        assert!(
            metadata_skill().join(path).is_file(),
            "files names {path}, which is not in the fixture"
        );
    }
    for target in metadata["open_when"].as_vec().expect("open_when is a list") {
        let exists = match (target["file"].as_str(), target["skill"].as_str()) {
            (Some(file), None) => metadata_skill().join(file).is_file(),
            (None, Some(skill)) => repo_root()
                .join("plugin/library")
                .join(skill)
                .join("SKILL.md")
                .is_file(),
            _ => false,
        };
        assert!(
            exists,
            "open_when target {target:?} is no fixture file and no library skill"
        );
    }
}
