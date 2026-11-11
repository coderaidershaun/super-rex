//! `rex init` never writes through a symbolic link, because the write would
//! land on whatever the link points at, outside the repo.

use std::fs;
use std::os::unix::fs::symlink;

use tempfile::tempdir;

use super::support::{
    AGENT_MARKER, ONE_STEP_FAILED, SKILL_MARKER, installed, repo, rex_init_failed, rex_init_ok,
    settings_path,
};

// This test must never be deleted. A project can hold a symlink where a skill
// file belongs. Writing through it would replace a file outside the project,
// such as a shell profile.
#[test]
fn force_never_overwrites_the_file_a_symlink_points_at() {
    let first = repo();
    let elsewhere = tempdir().expect("create tempdir");
    let marker = installed(&first, SKILL_MARKER);
    fs::create_dir_all(marker.parent().expect("marker has a folder")).expect("create skill folder");
    let outside = elsewhere.path().join("outside.txt");
    fs::write(&outside, "precious").expect("write outside file");
    symlink(&outside, &marker).expect("create symlink");
    // An agent file is guarded the same way.
    let agent = installed(&first, AGENT_MARKER);
    fs::create_dir_all(agent.parent().expect("agent has a folder")).expect("create agents folder");
    let outside_agent = elsewhere.path().join("outside-agent.md");
    fs::write(&outside_agent, "precious").expect("write outside agent file");
    symlink(&outside_agent, &agent).expect("create agent symlink");

    let stderr = rex_init_ok(&first, &["--force"]).stderr;

    assert_eq!(
        fs::read_to_string(&outside).expect("read outside file"),
        "precious"
    );
    assert!(
        fs::symlink_metadata(&marker)
            .expect("stat marker skill")
            .file_type()
            .is_symlink(),
        "the symlink must still be a symlink"
    );
    assert!(
        stderr.contains("rex init: skipped ./.rex/skills/rex-skill-language/SKILL.md: it or a folder above it is a symbolic link"),
        "got: {stderr}"
    );
    assert_eq!(
        fs::read_to_string(&outside_agent).expect("read outside agent file"),
        "precious"
    );
    assert!(
        stderr.contains("rex init: skipped ./.claude/agents/model-medium-effort-medium.md: it or a folder above it is a symbolic link"),
        "got: {stderr}"
    );

    // A link that points at nothing must not make rex create a file there,
    // and a linked skill folder must receive nothing.
    let second = repo();
    let dangling_target = elsewhere.path().join("created-outside.txt");
    let agent = installed(&second, AGENT_MARKER);
    fs::create_dir_all(agent.parent().expect("agent has a folder")).expect("create agents folder");
    symlink(&dangling_target, &agent).expect("create dangling symlink");
    let skills = second.path().join(".rex/skills");
    fs::create_dir_all(&skills).expect("create skills dir");
    let outside_dir = elsewhere.path().join("outside-dir");
    fs::create_dir_all(&outside_dir).expect("create outside dir");
    symlink(&outside_dir, skills.join("rex-skill-language")).expect("create folder symlink");

    let stderr = rex_init_ok(&second, &["--force"]).stderr;

    assert!(
        !dangling_target.exists(),
        "rex must not create a file where the symlink points"
    );
    assert_eq!(
        fs::read_dir(&outside_dir)
            .expect("read outside dir")
            .count(),
        0
    );
    assert!(
        stderr.contains("rex init: skipped ./.rex/skills/rex-skill-language/SKILL.md: it or a folder above it is a symbolic link"),
        "got: {stderr}"
    );
    assert!(
        stderr.contains("rex init: skipped ./.claude/agents/model-medium-effort-medium.md: it or a folder above it is a symbolic link"),
        "got: {stderr}"
    );
    assert!(
        second.path().join("AGENTS.md").is_file(),
        "the rest of the install must still happen"
    );
}

// This test must never be deleted. The same hazard as a linked skill file:
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
        installed(&repo, SKILL_MARKER).is_file(),
        "the skill bundle must still be installed"
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
    assert!(installed(&linked_file, SKILL_MARKER).is_file());
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
