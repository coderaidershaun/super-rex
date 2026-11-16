//! `rex worktree add` on a branch that exists already: a builder restarted on
//! resume finds its job branch with no worktree, and gets it back on that branch.

use std::fs;

use serde_json::json;

use super::{Repo, git, git_out, run, stderr};

// The worktree is made on the existing branch, and only at the commit of its
// tip, so the commit argument still says where the work starts.
#[test]
fn add_on_an_existing_branch_checks_it_out_at_its_tip_and_refuses_another_commit() {
    let repo = Repo::new();
    let first = repo.head();
    git(&repo.path(), &["branch", "rex/0gjob1"]);
    fs::write(repo.path().join("b"), "b\n").expect("write b");
    git(&repo.path(), &["add", "b"]);
    git(&repo.path(), &["commit", "-q", "-m", "second"]);
    let dir = repo.root().join("0gjob1");
    let add = |commit: &str| {
        run(&[
            &"add",
            &repo.path(),
            &dir,
            &commit,
            &"--branch",
            &"rex/0gjob1",
        ])
    };

    let elsewhere = add(&repo.head());
    let output = add(&first);

    let refusal = format!(
        "branch rex/0gjob1 exists at {first}, not at {}",
        repo.head()
    );
    assert_eq!(elsewhere.status.code(), Some(2), "{}", stderr(&elsewhere));
    assert!(
        stderr(&elsewhere).contains(&refusal),
        "{}",
        stderr(&elsewhere)
    );
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(git_out(&dir, &["branch", "--show-current"]), "rex/0gjob1");
    assert_eq!(git_out(&dir, &["rev-parse", "HEAD"]), first);
    assert_eq!(repo.registered(), json!([Repo::canonical(&dir)]));
}
