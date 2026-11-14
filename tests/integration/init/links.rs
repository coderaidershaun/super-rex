//! `rex init` never writes through a symbolic link, because the write would
//! land on whatever the link points at, outside the repo.

use std::fs;
use std::os::unix::fs::symlink;

use tempfile::tempdir;

use super::support::{ONE_STEP_FAILED, repo, rex_init_failed, rex_init_ok, settings_path};

// This test must never be deleted. A project can hold a symlink where the
// config belongs. Writing through it would replace a file outside the project,
// such as a shell profile, and a link that points at nothing would make rex
// create a file there.
#[test]
fn a_linked_config_is_never_written_through() {
    let elsewhere = tempdir().expect("create tempdir");
    let linked = repo();
    fs::create_dir_all(linked.path().join(".rex")).expect("create .rex");
    let outside = elsewhere.path().join("outside.json");
    fs::write(&outside, "{\"storage\": \"off\"}\n").expect("write outside file");
    symlink(&outside, linked.path().join(".rex/config.json")).expect("create symlink");

    let stderr = rex_init_ok(&linked, &["--with-storage"]).stderr;

    assert_eq!(
        fs::read_to_string(&outside).expect("read outside file"),
        "{\"storage\": \"off\"}\n"
    );
    assert!(
        stderr.contains("rex init: skipped ")
            && stderr.contains("config.json: it is a symbolic link"),
        "got: {stderr}"
    );

    let dangling = repo();
    fs::create_dir_all(dangling.path().join(".rex")).expect("create .rex");
    let target = elsewhere.path().join("created-outside.json");
    symlink(&target, dangling.path().join(".rex/config.json")).expect("create dangling symlink");

    rex_init_ok(&dangling, &[]);

    assert!(
        !target.exists(),
        "rex must not create a file where the symlink points"
    );
    assert!(
        dangling.path().join("AGENTS.md").is_file(),
        "the rest of the install must still happen"
    );
}

// This test must never be deleted. The same hazard as a linked config:
// a write through a link lands on whatever it points at, outside the project.
// AGENTS.md is the one rex changes in place rather than only creating, so a
// link here would edit a file the person never named. A link that points at
// nothing is worse: the write would create a file outside the project that
// nobody asked for.
#[test]
fn a_symlinked_agents_md_is_left_alone_and_its_target_untouched() {
    let repo = repo();
    let elsewhere = tempdir().expect("create tempdir");
    let outside = elsewhere.path().join("shared-notes.md");
    fs::write(&outside, "precious").expect("write outside file");
    symlink(&outside, repo.path().join("AGENTS.md")).expect("create symlink");

    let dangling = [".gitignore", ".pre-commit-config.yaml"];
    for name in dangling {
        symlink(elsewhere.path().join(name), repo.path().join(name))
            .expect("create dangling symlink");
    }

    let stderr = rex_init_ok(&repo, &[]).stderr;

    assert_eq!(
        fs::read_to_string(&outside).expect("read outside file"),
        "precious",
        "rex must not write through the link"
    );
    for name in dangling {
        assert!(
            !elsewhere.path().join(name).exists(),
            "rex created {name} outside the project"
        );
        assert!(
            stderr.contains(&format!(
                "rex init: skipped ./{name}: it is a symbolic link"
            )),
            "got: {stderr}"
        );
    }
    assert!(
        repo.path().join(".rex/config.json").is_file(),
        "the rest of the install must still happen"
    );
    assert!(
        stderr.contains("rex init: skipped ./AGENTS.md: it is a symbolic link"),
        "got: {stderr}"
    );
}

// A write through a link lands on whatever it points at, a file the person
// never named in this repo, so rex reads through a link but never writes
// through one.
#[test]
fn a_linked_settings_file_is_not_written_through() {
    let elsewhere = tempdir().expect("create tempdir");
    let linked_file = repo();
    let outside = elsewhere.path().join("shared-settings.json");
    fs::write(&outside, "{}\n").expect("write outside file");
    fs::create_dir_all(linked_file.path().join(".claude")).expect("create .claude");
    symlink(&outside, settings_path(&linked_file)).expect("link settings");

    let printed = rex_init_failed(&linked_file, &[]);

    assert_eq!(
        fs::read_to_string(&outside).expect("read outside file"),
        "{}\n"
    );
    assert!(linked_file.path().join(".rex/config.json").is_file());
    assert_eq!(
        printed.stderr,
        format!(
            "rex init: failed: .claude/settings.local.json is a symbolic link, and rex does not write through one, so the settings SuperRex needs are not set; add them to .claude/settings.local.json where the link points, then run rex init again\n{ONE_STEP_FAILED}"
        )
    );

    let linked_folder = repo();
    let outside = elsewhere.path().join("shared-claude");
    fs::create_dir_all(&outside).expect("create outside folder");
    symlink(&outside, linked_folder.path().join(".claude")).expect("link .claude");

    let printed = rex_init_failed(&linked_folder, &[]);

    assert!(!outside.join("settings.local.json").exists());
    assert_eq!(
        printed.stderr,
        format!(
            "rex init: failed: .claude is a symbolic link, and rex does not write through one, so the settings SuperRex needs are not set; add them to .claude/settings.local.json where the link points, then run rex init again\n{ONE_STEP_FAILED}"
        )
    );

    // A person who merged the settings by hand where the link points is not
    // stopped.
    let merged_by_hand = repo();
    rex_init_ok(&merged_by_hand, &[]);
    let outside = elsewhere.path().join("merged-by-hand.json");
    fs::copy(settings_path(&merged_by_hand), &outside).expect("copy settings");
    fs::remove_file(settings_path(&merged_by_hand)).expect("remove settings");
    symlink(&outside, settings_path(&merged_by_hand)).expect("link settings");
    let before = fs::read(&outside).expect("read outside file");

    let printed = rex_init_ok(&merged_by_hand, &[]);

    assert_eq!(printed.stderr, "");
    assert_eq!(fs::read(&outside).expect("read outside file"), before);
}
