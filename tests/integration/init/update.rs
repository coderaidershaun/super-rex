//! `rex init --force` replaces only the bundle files that differ, and leaves
//! the project files a person may have edited.

use std::fs;

use super::support::{
    Repo, SKILL_MARKER, bundle_file, bundle_file_count, installed, repo, rex_init_ok,
};

const HOOKS_LINE: &str = "rex init: pre-commit hooks installed\n";

fn rex_init_stdout(repo: &Repo, args: &[&str]) -> String {
    rex_init_ok(repo, args).stdout
}

#[test]
fn force_overwrites_only_the_files_that_differ() {
    let repo = repo();
    let n = bundle_file_count();
    rex_init_stdout(&repo, &[]);
    let marker = installed(&repo, SKILL_MARKER);
    fs::write(&marker, b"my own edit").expect("edit marker skill");

    let stdout = rex_init_stdout(&repo, &["--force"]);

    assert_eq!(
        stdout,
        format!(
            "rex init: 0 written, 1 overwritten, {} skipped\n{HOOKS_LINE}",
            n - 1
        )
    );
    assert_eq!(
        fs::read(marker).expect("read marker skill"),
        bundle_file(SKILL_MARKER)
    );

    let retired = repo.path().join(".claude/skills/orchestration-selection");
    fs::create_dir_all(&retired).expect("make retired skill folder");

    let printed = rex_init_ok(&repo, &["--force"]);

    assert_eq!(
        printed.stdout,
        format!("rex init: 0 written, {n} skipped\n{HOOKS_LINE}"),
        "--force on an up-to-date install must rewrite nothing"
    );
    assert!(
        printed
            .stderr
            .contains("orchestration-selection is a skill this version no longer uses"),
        "a retired skill folder must be named: {}",
        printed.stderr
    );
    assert!(
        retired.is_dir(),
        "rex must never delete a retired skill folder"
    );
}

// This test must never be deleted. --force is about the skill files only. A
// project file that rex also manages, but that the user has taken over, must
// stay exactly as the user left it.
#[test]
fn force_never_replaces_the_project_files_a_user_may_have_edited() {
    let repo = repo();
    rex_init_stdout(&repo, &[]);
    let edited = [
        ("AGENTS.md", "my own agent notes\n"),
        (".gitignore", "/my-own-ignore\n"),
        (".pre-commit-config.yaml", "my own hooks\n"),
    ];
    for (name, contents) in edited {
        fs::write(repo.path().join(name), contents).expect("write project file");
    }

    rex_init_stdout(&repo, &["--force"]);

    for (name, contents) in edited {
        let after = fs::read_to_string(repo.path().join(name)).expect("read project file");
        assert!(
            after.contains(contents),
            "{name} lost the user's text: {after:?}"
        );
    }
    let config = fs::read_to_string(repo.path().join(".pre-commit-config.yaml")).expect("read");
    assert_eq!(config, "my own hooks\n");
}
