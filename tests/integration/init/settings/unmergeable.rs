//! What `rex init` does with a settings file it cannot merge into: the file
//! stays as the person left it and the run fails with a message that names it.

use std::fs;

use super::expected::{read_settings, write_settings};
use crate::init::support::{ONE_STEP_FAILED, repo, rex_init_failed, settings_path};

// A file rex cannot read as a JSON object is still the person's: writing it
// would lose whatever they meant it to hold.
#[test]
fn a_settings_file_that_is_not_json_is_left_alone_and_the_run_fails() {
    let not_json = repo();
    let broken = "{ \"sandbox\": { \"enabled\": true, }\n";
    write_settings(&not_json, broken);

    let printed = rex_init_failed(&not_json, &[]);

    assert_eq!(read_settings(&not_json), broken);
    assert!(
        not_json.path().join(".rex/config.json").is_file()
            && not_json.path().join("AGENTS.md").is_file(),
        "the rest of the install must still happen"
    );
    assert!(
        !printed.stdout.contains("settings.local.json"),
        "got: {}",
        printed.stdout
    );
    let failed_not_json = "rex init: failed: .claude/settings.local.json is not valid JSON and was left as it is, so the settings SuperRex needs are not set; correct it, then run rex init again: ";
    assert!(
        printed.stderr.starts_with(failed_not_json),
        "got: {}",
        printed.stderr
    );
    assert!(
        printed.stderr.contains(" line 1 column "),
        "got: {}",
        printed.stderr
    );
    assert!(
        printed.stderr.ends_with(&format!("\n{ONE_STEP_FAILED}")),
        "got: {}",
        printed.stderr
    );
    assert_eq!(printed.stderr.lines().count(), 2, "got: {}", printed.stderr);

    let not_an_object = repo();
    write_settings(&not_an_object, "[]\n");

    let printed = rex_init_failed(&not_an_object, &[]);

    assert_eq!(read_settings(&not_an_object), "[]\n");
    assert_eq!(
        printed.stderr,
        format!(
            "rex init: failed: .claude/settings.local.json is not a JSON object and was left as it is, so the settings SuperRex needs are not set; make its top level an object, then run rex init again\n{ONE_STEP_FAILED}"
        )
    );

    // A folder where the file goes cannot be read, on any system and for
    // any user.
    let unreadable = repo();
    fs::create_dir_all(settings_path(&unreadable)).expect("create folder at the settings path");

    let printed = rex_init_failed(&unreadable, &[]);

    assert!(settings_path(&unreadable).is_dir());
    let failed = "rex init: failed: .claude/settings.local.json could not be read and was left as it is, so the settings SuperRex needs are not set; make it readable, then run rex init again: ";
    assert!(
        printed.stderr.starts_with(failed) && printed.stderr.ends_with(ONE_STEP_FAILED),
        "got: {}",
        printed.stderr
    );

    // A file rex cannot read later on stops the install at that step, and
    // the failure kept before it is still named, first.
    let stopped = repo();
    write_settings(&stopped, broken);
    fs::create_dir(stopped.path().join("AGENTS.md")).expect("create folder at AGENTS.md");

    let printed = rex_init_failed(&stopped, &[]);

    assert!(
        !stopped.path().join(".gitignore").exists(),
        "the install stops"
    );
    let lines: Vec<&str> = printed.stderr.lines().collect();
    assert_eq!(lines.len(), 3, "got: {}", printed.stderr);
    assert!(lines[0].starts_with(failed_not_json), "got: {}", lines[0]);
    assert!(
        lines[1].starts_with("rex init: failed: io error at ./AGENTS.md: "),
        "got: {}",
        lines[1]
    );
    assert_eq!(
        lines[2],
        "error: rex init did not finish 2 step(s); each is named above"
    );
}
