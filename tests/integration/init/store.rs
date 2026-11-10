//! The store and its library folder that `rex init` sets up in `.rex/`.

use std::fs;

use super::support::{SKILL_MARKER, installed, repo, rex_init, rex_init_ok};

#[test]
fn fresh_install_creates_the_store_and_the_library_folder() {
    let repo = repo();

    let printed = rex_init_ok(&repo, &[]);

    assert!(
        printed.stdout.contains("rex init: .rex/ created\n"),
        "got: {}",
        printed.stdout
    );
    let store = repo.path().join(".rex");
    assert!(store.join("orchestrators").is_dir());
    assert!(store.join("rex.db").is_file());
}

// A store that cannot be set up must not stop the rest of the install, and
// the person must see why it failed.
#[test]
fn a_store_that_cannot_be_set_up_is_reported_after_the_rest_is_done() {
    let repo = repo();
    fs::write(
        repo.path().join(".rex"),
        "a file where the store folder goes\n",
    )
    .expect("write .rex file");

    let output = rex_init(&repo, &[]);

    assert_eq!(output.status.code(), Some(1), "rex init must fail");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(!stdout.contains(".rex/"), "got: {stdout}");
    assert!(stdout.contains("rex init: pre-commit hooks installed\n"));
    assert!(installed(&repo, SKILL_MARKER).is_file());

    let orchestrators = repo
        .path()
        .canonicalize()
        .expect("canonical repo path")
        .join(".rex")
        .join("orchestrators");
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    let failed = format!(
        "rex init: failed: the store could not be set up: cannot use {}: ",
        orchestrators.display()
    );
    assert!(stderr.starts_with(&failed), "got: {stderr}");
    assert!(
        stderr.ends_with("\nerror: rex init did not finish 1 step(s); each is named above\n"),
        "got: {stderr}"
    );
    assert_eq!(stderr.lines().count(), 2, "got: {stderr}");
}
