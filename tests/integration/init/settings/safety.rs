//! Which permissions `rex init` writes: the one rule that allows every Bash
//! command, added to a list of the person's, no sandbox, and when the folder
//! above the repo is opened to agents.

use std::path::Path;

use serde_json::{Value, json};

use super::expected::{PLUGIN_DENY_RULES, parent_of, read_settings, write_settings};
use crate::init::support::{repo, rex_init_command, rex_init_ok};

fn written(repo: &crate::init::support::Repo) -> Value {
    serde_json::from_str(&read_settings(repo)).expect("settings JSON")
}

// An agent runs every Bash command with no prompt; the deny rules still hold
// over that rule, and rex turns no sandbox on.
#[test]
fn every_bash_command_is_allowed_and_no_sandbox_is_written() {
    let repo = repo();

    let printed = rex_init_ok(&repo, &[]);

    assert_eq!(printed.stderr, "");
    let settings = written(&repo);
    assert_eq!(settings["permissions"]["allow"], json!(["Bash"]));
    let deny = settings["permissions"]["deny"]
        .as_array()
        .expect("a deny list");
    assert_eq!(deny.len(), 10, "got: {deny:?}");
    for rule in PLUGIN_DENY_RULES {
        assert!(deny.contains(&json!(rule)), "{rule}");
    }
    assert_eq!(settings.get("sandbox"), None);
    assert_eq!(
        settings["permissions"].get("blockWritesOutsideWorkingDirectories"),
        None
    );
}

// The person's own allow rules stay, in their order, and "Bash" is added once
// after them, however often rex init runs.
#[test]
fn bash_is_added_to_an_allow_list_of_the_person_once() {
    let repo = repo();
    write_settings(
        &repo,
        r#"{"permissions": {"allow": ["Bash(npm test)", "WebFetch", "Bash(rex worktree add *)"]}}"#,
    );

    rex_init_ok(&repo, &[]);
    let before = read_settings(&repo);
    let printed = rex_init_ok(&repo, &[]);

    assert_eq!(
        written(&repo)["permissions"]["allow"],
        json!([
            "Bash(npm test)",
            "WebFetch",
            "Bash(rex worktree add *)",
            "Bash"
        ])
    );
    assert_eq!(read_settings(&repo), before);
    assert!(
        !printed.stdout.contains("settings.local.json"),
        "got: {}",
        printed.stdout
    );

    // A list that holds "Bash" already is not written again.
    let has_it = crate::init::support::repo();
    write_settings(&has_it, r#"{"permissions": {"allow": ["Bash", "Read"]}}"#);

    rex_init_ok(&has_it, &[]);

    assert_eq!(
        written(&has_it)["permissions"]["allow"],
        json!(["Bash", "Read"])
    );
}

// Opening the folder above the repo to agents must never open the home
// folder, which holds keys and shell profiles.
#[test]
fn a_parent_that_holds_home_is_not_added() {
    let homes: [fn(&Path) -> &Path; 2] = [
        |repo| repo.parent().expect("the repo has a parent"),
        |repo| repo,
    ];
    for home_of in homes {
        let repo = repo();
        let canonical = repo.path().canonicalize().expect("canonical repo path");

        let output = rex_init_command(&repo, &[])
            .env("HOME", home_of(&canonical))
            .output()
            .expect("run rex init");

        let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
        assert!(output.status.success(), "got: {stderr}");
        assert_eq!(
            stderr,
            format!(
                "rex init: warning: the parent folder {} holds your home folder, so it was not added as a working directory, and the file tools of agents work only inside this repo. Move the repo into a folder of its own to let them edit sibling repos.\n",
                parent_of(&repo)
            )
        );
        let settings = written(&repo);
        assert_eq!(settings["permissions"].get("additionalDirectories"), None);
        assert_eq!(settings["permissions"]["allow"], json!(["Bash"]));
    }
}
