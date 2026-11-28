//! `rex skills check` with and without `--pool`: each intent phrase must find
//! its own skill in the top five among the library and the pool.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::skills_serve::copy_tree;

const LIBRARY: &str = "tests/fixtures/skills-intent/library";
const POOL: &str = "tests/fixtures/skills-intent/pool";
const FOUND_SKILL: &str = "tests/fixtures/skills-intent/library/ix-diagram-colours";

const SKIP_LINE: &str = "warning: intent check skipped; give --pool <folder> of other skills \
    to check that each intent finds its skill\n";

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The warning that the check prints when it searched fewer than 1,000 skills.
fn small_pool_line(searched: usize) -> String {
    format!(
        "warning: the intent check searched {searched} skills; a pool of 1000 or more is \
         needed for a real check\n"
    )
}

/// Runs `rex skills check` from the repo root with an empty home folder, so
/// the user's own skills are never read.
fn skills_check(arguments: &[&str]) -> Output {
    let home = tempfile::tempdir().expect("temp home");
    Command::new(env!("CARGO_BIN_EXE_rex"))
        .current_dir(repo_root())
        .env("HOME", home.path())
        .args(["skills", "check"])
        .args(arguments)
        .output()
        .expect("run rex skills check")
}

fn text_of(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("utf8 output")
}

/// Stdout, stderr and exit code of one run, to compare as one value.
fn trace(output: &Output) -> (String, String, Option<i32>) {
    (
        text_of(&output.stdout),
        text_of(&output.stderr),
        output.status.code(),
    )
}

#[test]
fn intent_that_misses_its_skill_is_a_finding() {
    let output = skills_check(&[LIBRARY, "--pool", POOL]);

    assert_eq!(
        trace(&output),
        (
            "ok tests/fixtures/skills-intent/library/ix-diagram-colours\n\
             tests/fixtures/skills-intent/library/ix-sorting-notes/SKILL.md: M14: intent \
             \"sort a list of records quickly\" does not find this skill in the top 5 of 10 \
             skills; it ranks 7\n"
                .to_owned(),
            format!(
                "{}error: 1 findings in 1 of 2 skills\n",
                small_pool_line(10)
            ),
            Some(1),
        )
    );
}

#[test]
fn intents_that_find_their_skill_pass() {
    let output = skills_check(&[FOUND_SKILL, "--pool", POOL]);

    assert_eq!(
        trace(&output),
        (format!("ok {FOUND_SKILL}\n"), small_pool_line(9), Some(0),)
    );
}

#[test]
fn no_pool_skips_the_intent_check_with_one_line() {
    let output = skills_check(&[LIBRARY]);

    assert_eq!(
        trace(&output),
        (
            "ok tests/fixtures/skills-intent/library/ix-diagram-colours\n\
             ok tests/fixtures/skills-intent/library/ix-sorting-notes\n"
                .to_owned(),
            SKIP_LINE.to_owned(),
            Some(0),
        )
    );
}

#[test]
fn unreadable_pool_skill_is_left_out() {
    let temp = tempfile::tempdir().expect("temp folder");
    let pool = temp.path().join("pool");
    copy_tree(&repo_root().join(POOL), &pool);
    let locked = pool.join("px-locked/SKILL.md");
    fs::create_dir_all(locked.parent().expect("a skill folder")).expect("make a pool skill");
    fs::write(
        &locked,
        "---\nname: px-locked\ndescription: Colours of a diagram.\n---\n# Locked\n",
    )
    .expect("write a pool skill");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).expect("chmod 000");
    if fs::read(&locked).is_ok() {
        eprintln!("this user can read a file of mode 000, so the test asserts nothing");
        return;
    }

    let output = skills_check(&[FOUND_SKILL, "--pool", path_text(&pool)]);

    fs::set_permissions(&locked, fs::Permissions::from_mode(0o644)).expect("chmod back");
    let without_it = skills_check(&[FOUND_SKILL, "--pool", POOL]);
    assert_eq!(trace(&output), trace(&without_it));
    assert_eq!(
        trace(&output),
        (format!("ok {FOUND_SKILL}\n"), small_pool_line(9), Some(0))
    );
}

#[test]
fn fixture_library_passes_its_intent_check() {
    let output = skills_check(&[
        "tests/fixtures/skills-search",
        "--pool",
        "tests/fixtures/skills-search/unrelated",
    ]);

    let ok_lines: String = [
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
    ]
    .iter()
    .map(|skill| format!("ok tests/fixtures/skills-search/{skill}\n"))
    .collect();
    assert_eq!(trace(&output), (ok_lines, small_pool_line(18), Some(0)));
}

fn path_text(path: &Path) -> &str {
    path.to_str().expect("a temp path in UTF-8")
}
