//! The skill library that rex 0.4 to 0.7 copied into `.rex/skills/`: `rex init`
//! removes a copy the person never changed, so it no longer hides the library
//! the plugin ships, and keeps every other folder there.

use std::fs;
use std::path::{Path, PathBuf};

use super::support::{Repo, repo, rex_init_ok};

/// Two skills exactly as rex 0.7.0 wrote them, from `git show 7deda22:.rex/skills/…`.
fn fixture(skill: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/old-library")
        .join(skill)
}

/// Copy the fixture of `skill` into the repo's `.rex/skills/<skill>/`.
fn install_old_copy(repo: &Repo, skill: &str) -> PathBuf {
    let dest = repo.path().join(".rex/skills").join(skill);
    fs::create_dir_all(&dest).expect("create skill folder");
    for entry in fs::read_dir(fixture(skill)).expect("list fixture") {
        let entry = entry.expect("fixture entry");
        fs::copy(entry.path(), dest.join(entry.file_name())).expect("copy fixture file");
    }
    dest
}

#[test]
fn an_unchanged_old_copy_is_removed_with_one_notice_each() {
    let repo = repo();
    let spike = install_old_copy(&repo, "rex-code-spike");
    let commenting = install_old_copy(&repo, "rex-code-commenting");
    // Finder's own file is not a change of the person's.
    fs::write(commenting.join(".DS_Store"), b"\0\0").expect("write .DS_Store");

    let printed = rex_init_ok(&repo, &[]);

    assert!(
        !spike.exists(),
        "the 0.7 copy of rex-code-spike is still there"
    );
    assert!(
        !commenting.exists(),
        "the 0.7 copy of rex-code-commenting is still there"
    );
    for skill in ["rex-code-commenting", "rex-code-spike"] {
        let line = format!(
            "rex init: removed .rex/skills/{skill}/: it was an unchanged copy from an older rex, and the plugin ships {skill} now\n"
        );
        assert_eq!(
            printed.stdout.matches(&line).count(),
            1,
            "got: {}",
            printed.stdout
        );
    }
    assert_eq!(printed.stderr, "");

    // Nothing is left to remove the second time.
    let printed = rex_init_ok(&repo, &[]);

    assert!(
        !printed.stdout.contains("removed"),
        "got: {}",
        printed.stdout
    );
}

#[test]
fn a_changed_copy_is_kept_with_a_notice_that_it_shadows_the_shipped_skill() {
    let repo = repo();
    let changed = install_old_copy(&repo, "rex-code-commenting");
    let mut text = fs::read_to_string(changed.join("examples.md")).expect("read example");
    text.push_str("\nOne more example of ours.\n");
    fs::write(changed.join("examples.md"), &text).expect("change example");
    let added = install_old_copy(&repo, "rex-code-spike");
    fs::write(added.join("notes.md"), "Our notes.\n").expect("add a file");

    let printed = rex_init_ok(&repo, &[]);

    assert_eq!(
        fs::read_to_string(changed.join("examples.md")).expect("read"),
        text
    );
    assert!(added.join("notes.md").exists());
    assert!(added.join("SKILL.md").exists());
    assert!(
        !printed.stdout.contains("removed"),
        "got: {}",
        printed.stdout
    );
    assert_eq!(
        printed.stderr,
        "rex init: warning: kept .rex/skills/rex-code-commenting/: it differs from the copy an older rex installed, so it hides the rex-code-commenting skill that the plugin ships; to use the plugin's, move your changes out and delete that folder\n\
         rex init: warning: kept .rex/skills/rex-code-spike/: it differs from the copy an older rex installed, so it hides the rex-code-spike skill that the plugin ships; to use the plugin's, move your changes out and delete that folder\n"
    );
}

#[test]
fn a_skill_of_the_person_or_of_a_job_is_kept_with_no_notice() {
    let repo = repo();
    let skills = repo.path().join(".rex/skills");
    // A skill a job wrote, and a feedback fork of a shipped skill under its own name.
    for (name, text) in [
        (
            "shop-csv-export",
            "---\nname: shop-csv-export\n---\nOurs.\n",
        ),
        (
            "rex-code-tdd-shop",
            "---\nname: rex-code-tdd-shop\n---\nFork.\n",
        ),
    ] {
        fs::create_dir_all(skills.join(name)).expect("create skill");
        fs::write(skills.join(name).join("SKILL.md"), text).expect("write skill");
    }
    // A file of the person's, not a skill folder, with a shipped name.
    fs::write(skills.join("rex-code-tdd"), "a file\n").expect("write file");

    let printed = rex_init_ok(&repo, &[]);

    for name in ["shop-csv-export", "rex-code-tdd-shop"] {
        assert!(
            skills.join(name).join("SKILL.md").exists(),
            "{name} was removed"
        );
    }
    assert!(skills.join("rex-code-tdd").is_file());
    assert!(
        !printed.stdout.contains("removed"),
        "got: {}",
        printed.stdout
    );
    assert_eq!(printed.stderr, "");
}

#[cfg(unix)]
#[test]
fn a_linked_library_folder_is_never_looked_into() {
    let repo = repo();
    let shared = tempfile::tempdir().expect("shared library");
    let shared_copy = shared.path().join("rex-code-spike");
    fs::create_dir_all(&shared_copy).expect("create shared skill");
    fs::copy(
        fixture("rex-code-spike").join("SKILL.md"),
        shared_copy.join("SKILL.md"),
    )
    .expect("copy fixture");
    fs::create_dir_all(repo.path().join(".rex")).expect("create .rex");
    std::os::unix::fs::symlink(shared.path(), repo.path().join(".rex/skills"))
        .expect("link the library folder");

    let printed = rex_init_ok(&repo, &[]);

    assert!(
        shared_copy.join("SKILL.md").exists(),
        "a shared copy was removed"
    );
    assert_eq!(
        printed.stderr,
        "rex init: warning: .rex/skills is a symbolic link, so rex did not remove the copies an older rex installed there; for a copy there, it hides the skill of the same name that the plugin ships\n"
    );
}
