//! The `.gitignore` block of an older `rex init`, which ignored `.rex/config.json`,
//! `.rex/skills/`, `.claude/skills/` and `.claude/agents/`: `rex init` replaces it
//! when it is as that release wrote it, and says what to change when it is not.

use std::fs;

use super::support::{repo, rex_init_ok};

/// The block of rex 0.4.0 to 0.7.0, byte for byte.
const BLOCK_0_4: &str = "# rex:ignore superrex\n.claude/skills/\n.claude/agents/\n.claude/settings.local.json\n/.rex/*\n/.claude/worktrees/\n";

/// The block of rex 0.1.0 to 0.3.0, byte for byte.
const BLOCK_0_1: &str = "# rex:ignore superrex\n.claude/skills/\n.claude/agents/\n.claude/settings.local.json\n/.rex/\n/.claude/worktrees/\n";

/// The block this version writes.
const BLOCK: &str = "# rex:ignore superrex\n.claude/settings.local.json\n/.claude/worktrees/\n/.rex/rex.db*\n/.rex/events.jsonl\n/.rex/orchestrators/\n/.rex/plans/\n/.rex/worktrees/\n/.rex/live/\n";

#[test]
fn an_old_block_is_replaced_in_place_and_the_lines_around_it_stay() {
    for old in [BLOCK_0_4, BLOCK_0_1] {
        let repo = repo();
        let path = repo.path().join(".gitignore");
        fs::write(&path, format!("/target\n\n{old}*.log\n")).expect("write .gitignore");

        let printed = rex_init_ok(&repo, &[]);

        assert_eq!(
            fs::read_to_string(&path).expect("read .gitignore"),
            format!("/target\n\n{BLOCK}*.log\n")
        );
        assert!(
            printed.stdout.contains(
                "rex init: replaced the rex block in .gitignore with this version's: the old one also ignored .rex/config.json, .rex/skills/, .claude/skills/ and .claude/agents/\n"
            ),
            "got: {}",
            printed.stdout
        );
        assert!(printed.stdout.contains("rex init: .gitignore updated\n"));
        assert!(
            printed
                .stdout
                .contains(".gitignore and .pre-commit-config.yaml (git add + git commit)"),
            "the commit line names .gitignore: {}",
            printed.stdout
        );
        assert_eq!(printed.stderr, "");

        let again = rex_init_ok(&repo, &[]);

        assert!(
            !again.stdout.contains(".gitignore"),
            "got: {}",
            again.stdout
        );
    }
}

#[test]
fn a_changed_block_is_kept_and_the_lines_to_change_are_named() {
    let repo = repo();
    let path = repo.path().join(".gitignore");
    let theirs = "# rex:ignore superrex\n.claude/skills/\n.claude/settings.local.json\n/.rex/*\n!/.rex/skills/\n/.claude/worktrees/\n\n/target\n";
    fs::write(&path, theirs).expect("write .gitignore");

    let printed = rex_init_ok(&repo, &[]);

    assert_eq!(fs::read_to_string(&path).expect("read .gitignore"), theirs);
    assert!(
        !printed.stdout.contains(".gitignore updated"),
        "got: {}",
        printed.stdout
    );
    assert_eq!(
        printed.stderr,
        "rex init: warning: kept the rex block in .gitignore, which differs from the block any rex wrote; to finish the upgrade, change it by hand:\n\
         rex init:   delete the line .claude/skills/\n\
         rex init:   delete the line /.rex/*\n\
         rex init:   add the line /.rex/rex.db*\n\
         rex init:   add the line /.rex/events.jsonl\n\
         rex init:   add the line /.rex/orchestrators/\n\
         rex init:   add the line /.rex/plans/\n\
         rex init:   add the line /.rex/worktrees/\n\
         rex init:   add the line /.rex/live/\n"
    );
}

#[test]
fn a_changed_block_that_already_ignores_what_this_version_does_gets_no_warning() {
    let repo = repo();
    let path = repo.path().join(".gitignore");
    // The same lines in another order, and one more of the person's.
    let theirs = "# rex:ignore superrex\n/.rex/live/\n.claude/settings.local.json\n/.claude/worktrees/\n/.rex/rex.db*\n/.rex/events.jsonl\n/.rex/orchestrators/\n/.rex/plans/\n/.rex/worktrees/\n/.rex/cache/\n";
    fs::write(&path, theirs).expect("write .gitignore");

    let printed = rex_init_ok(&repo, &[]);

    assert_eq!(fs::read_to_string(&path).expect("read .gitignore"), theirs);
    assert_eq!(printed.stderr, "");
}

// This repo keeps its own copy of the block, so a `rex init` here must find
// nothing to change in it and nothing to warn about. The packaged crate has no
// `.gitignore`, so the test from inside the package has nothing to check.
#[test]
fn this_repos_own_gitignore_needs_no_change_and_no_warning() {
    let own = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".gitignore");
    let Ok(text) = fs::read_to_string(&own) else {
        return;
    };
    let repo = repo();
    let path = repo.path().join(".gitignore");
    fs::write(&path, &text).expect("write .gitignore");

    let printed = rex_init_ok(&repo, &[]);

    assert_eq!(fs::read_to_string(&path).expect("read .gitignore"), text);
    assert!(
        !printed.stdout.contains(".gitignore"),
        "got: {}",
        printed.stdout
    );
    assert!(
        !printed.stderr.contains(".gitignore"),
        "got: {}",
        printed.stderr
    );
}
