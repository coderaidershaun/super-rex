//! A store that `rex init` cannot set up in `.rex/` is reported, and the rest
//! of the install still happens.

use std::fs;

use super::support::{ONE_STEP_FAILED, repo, rex_init_failed};

// A store that cannot be set up must not stop the rest of the install, and
// the person must see why it failed.
#[test]
fn a_store_that_cannot_be_set_up_is_reported_after_the_rest_is_done() {
    let repo = repo();
    fs::create_dir_all(repo.path().join(".rex")).expect("make .rex");
    fs::write(
        repo.path().join(".rex/orchestrators"),
        "a file where the orchestrator folder goes\n",
    )
    .expect("write orchestrators file");

    let printed = rex_init_failed(&repo, &["--with-storage"]);

    let stdout = printed.stdout;
    assert!(!stdout.contains(".rex/rex.db"), "got: {stdout}");
    assert!(stdout.contains("rex init: pre-commit hooks installed\n"));
    assert!(repo.path().join("AGENTS.md").is_file());

    let orchestrators = repo
        .path()
        .canonicalize()
        .expect("canonical repo path")
        .join(".rex")
        .join("orchestrators");
    let stderr = printed.stderr;
    let failed = format!(
        "rex init: failed: the store could not be set up: cannot use {}: ",
        orchestrators.display()
    );
    assert!(stderr.starts_with(&failed), "got: {stderr}");
    assert!(
        stderr.ends_with(&format!("\n{ONE_STEP_FAILED}")),
        "got: {stderr}"
    );
    assert_eq!(stderr.lines().count(), 2, "got: {stderr}");
}
