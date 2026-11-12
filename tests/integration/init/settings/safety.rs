//! Which safety settings `rex init` writes: what `--strict` and `--no-hooks`
//! change, and when the folder above the repo is opened to agents.

use std::path::Path;

use serde_json::{Value, json};

use super::expected::{as_written, hooked, parent_of, read_settings, safety_settings};
use crate::init::support::{Repo, repo, rex_init_command, rex_init_failed, rex_init_ok};

// A program that opens its own network connection cannot run in the sandbox,
// so an agent may run a command again outside it. Some systems block the
// sandbox itself, so Claude Code still starts there, with no sandbox.
// `--strict` takes both away, and a file that is stricter than the run asks
// for is never made weaker. `--no-hooks` leaves out the hook entries.
#[test]
fn strict_and_no_hooks_change_only_the_settings_they_name() {
    let sandbox_key = |repo: &Repo, name: &str| {
        let written: Value = serde_json::from_str(&read_settings(repo)).expect("settings JSON");
        written["sandbox"][name].clone()
    };
    let allow_key = |repo: &Repo| sandbox_key(repo, "allowUnsandboxedCommands");
    let fail_key = |repo: &Repo| sandbox_key(repo, "failIfUnavailable");

    let strict = repo();

    let printed = rex_init_ok(&strict, &["--strict"]);

    assert_eq!(printed.stderr, "");
    assert_eq!(allow_key(&strict), json!(false));
    assert_eq!(fail_key(&strict), json!(true));
    let mut expected = hooked(safety_settings(&strict));
    expected["sandbox"]["failIfUnavailable"] = json!(true);
    expected["sandbox"]["allowUnsandboxedCommands"] = json!(false);
    // With --strict no command may run outside the sandbox, so none is excluded.
    expected["sandbox"]
        .as_object_mut()
        .expect("sandbox object")
        .remove("excludedCommands");
    assert_eq!(read_settings(&strict), as_written(&expected));

    // A later run without the flag keeps the stricter values, and says how to
    // change each. A file that an earlier version of rex wrote is this case.
    let before = read_settings(&strict);

    let printed = rex_init_ok(&strict, &[]);

    assert_eq!(read_settings(&strict), before);
    assert_eq!(
        printed.stderr,
        "rex init: warning: kept sandbox.failIfUnavailable = true in .claude/settings.local.json, so Claude Code refuses to start when its sandbox cannot; set it to false to let it start and run commands with no sandbox.\n\
         rex init: warning: kept sandbox.allowUnsandboxedCommands = false in .claude/settings.local.json, so no agent can run a command outside the sandbox, such as a test that opens its own network connection; set it to true to let agents do that.\n"
    );

    // The other way round, the kept values are the weaker ones, so the run fails.
    let open = repo();
    rex_init_ok(&open, &[]);
    assert_eq!(allow_key(&open), json!(true));
    assert_eq!(fail_key(&open), json!(false));

    let printed = rex_init_failed(&open, &["--strict"]);

    assert_eq!(allow_key(&open), json!(true));
    assert_eq!(fail_key(&open), json!(false));
    assert_eq!(
        printed.stderr,
        "rex init: failed: kept sandbox.failIfUnavailable = false in .claude/settings.local.json, so the protection it gives is off; set it to true to turn it on\n\
         rex init: failed: kept sandbox.allowUnsandboxedCommands = true in .claude/settings.local.json, so the protection it gives is off; set it to false to turn it on\n\
         error: rex init did not finish 2 step(s); each is named above\n"
    );

    // Leaving the hooks out turns telemetry off for a new repo, but it never
    // deletes entries an earlier run wrote: the person is told to remove them.
    let fresh = repo();

    let printed = rex_init_ok(&fresh, &["--no-hooks"]);

    assert_eq!(printed.stderr, "");
    assert_eq!(read_settings(&fresh), as_written(&safety_settings(&fresh)));

    let hooked = repo();
    rex_init_ok(&hooked, &[]);
    let before = read_settings(&hooked);

    let printed = rex_init_ok(&hooked, &["--no-hooks"]);

    assert_eq!(read_settings(&hooked), before, "the hook entries stay");
    assert_eq!(
        printed.stderr,
        "rex init: warning: hooks left out, but hook entries from an earlier run are still in .claude/settings.local.json. Remove the entries that run rex hook, rex gate or rex test-hook to turn telemetry, the spend gate and the test output filter off.\n"
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
                "rex init: warning: the parent folder {} holds your home folder, so it was not added as a working directory. Agents can write only inside this repo. Move the repo into a folder of its own to let agents edit sibling repos.\n",
                parent_of(&repo)
            )
        );
        let written: Value = serde_json::from_str(&read_settings(&repo)).expect("settings JSON");
        assert_eq!(written["permissions"].get("additionalDirectories"), None);
        assert_eq!(
            written["permissions"]["blockWritesOutsideWorkingDirectories"],
            json!(true)
        );
    }
}
