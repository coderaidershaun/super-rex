//! The deliveries of a job through the `rex` binary: one for each of two
//! repositories, a `blocked` one recorded again once it is `merged`, and a
//! `kept-local` one.

use super::fixture::{
    COMMIT, PLAN, SESSION, close, new_repo, new_store, open_job, plan, rex, show_json, stderr_of,
    stdout_of, value,
};

#[test]
fn two_repos_one_merged_one_blocked_then_recorded_later() {
    let temp = tempfile::tempdir().unwrap();
    let shop = new_repo(temp.path(), "shop");
    let web = new_repo(temp.path(), "web-client");
    new_store(&shop, "pipeline");
    let job = open_job(
        &shop,
        SESSION,
        &["--merge", "yes", "--type", "fix", "--lang", "rust"],
    );
    plan(&shop, &job, PLAN);

    let note = "pull request 17 waits for a human's approval";
    let first = format!(
        r#"[{{"repo": "{}", "pr": "https://example.com/pull/17", "outcome": "blocked", "note": "{note}"}},
            {{"repo": "{}", "merge_commit": "{COMMIT}", "outcome": "merged"}}]"#,
        web.display(),
        shop.display()
    );
    let refused = stderr_of(&mut close(&shop, &job, "delivered", &first), 1);
    assert_eq!(
        refused,
        format!(
            "error: job {job} cannot be delivered while its delivery to {} is blocked; close it \
             as blocked, or record that repository's new outcome in the deliveries file\n",
            web.display()
        )
    );
    assert_eq!(show_json(&shop, &job)["deliveries"], serde_json::json!([]));

    stdout_of(&mut close(&shop, &job, "blocked", &first));
    let shown = stdout_of(&mut rex(&shop, &["job", "show", &job]));
    assert_eq!(value(&shown, "status"), "blocked");
    assert!(
        value(&shown, "delivery.1").starts_with("blocked "),
        "{shown}"
    );
    assert!(
        value(&shown, "delivery.2").starts_with("merged "),
        "{shown}"
    );
    assert!(
        shown.ends_with(&format!("delivery.1.note:\n  {note}\n")),
        "{shown}"
    );
    let kept = show_json(&shop, &job)["deliveries"][1].clone();

    let rest = format!(
        r#"[{{"repo": "{}", "pr": "https://example.com/pull/17", "merge_commit": "{COMMIT}", "outcome": "merged"}}]"#,
        web.display()
    );
    // From the sibling repository, which has no store, the caller names the store.
    let delivered =
        stdout_of(close(&web, &job, "delivered", &rest).env("REX_STORE", shop.join(".rex")));
    assert_eq!(
        delivered,
        format!(
            "job={job}\nstatus=delivered\ndeliveries=2\ndelivery.1=merged {}\n\
             delivery.2=merged {}\n",
            web.display(),
            shop.display()
        )
    );
    let record = show_json(&shop, &job);
    assert_eq!(record["deliveries"][1], kept);
    let closes: Vec<(&str, &str)> = record["transitions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["event"] == "job close")
        .map(|row| (row["from"].as_str().unwrap(), row["to"].as_str().unwrap()))
        .collect();
    assert_eq!(closes, [("planned", "blocked"), ("blocked", "delivered")]);

    // No remote, and the requester declined a merge: the work stays on its branch.
    let local = open_job(
        &shop,
        "session-2",
        &["--merge", "no", "--type", "fix", "--lang", "go"],
    );
    plan(&shop, &local, PLAN);
    let note = "branch fix-zero keeps the change; the repository has no remote";
    let deliveries = format!(
        r#"[{{"repo": "{}", "branch": "fix-zero", "outcome": "kept-local", "note": "{note}"}}]"#,
        shop.display()
    );
    let closed = stdout_of(&mut close(&shop, &local, "delivered", &deliveries));
    assert_eq!(
        closed,
        format!(
            "job={local}\nstatus=delivered\ndeliveries=1\ndelivery.1=kept-local {}\n",
            shop.display()
        )
    );
    let shown = stdout_of(&mut rex(&shop, &["job", "show", &local]));
    assert_eq!(
        value(&shown, "delivery.1"),
        format!(
            "kept-local branch=fix-zero pr=none merge_agreed=no merge_commit=none repo={}",
            shop.display()
        )
    );
    assert!(
        shown.ends_with(&format!("delivery.1.note:\n  {note}\n")),
        "{shown}"
    );

    // A job that changed no repository closes with no deliveries.
    let research = open_job(
        &shop,
        "session-3",
        &["--merge", "no", "--type", "research", "--lang", "none"],
    );
    plan(&shop, &research, PLAN);
    let closed = stdout_of(&mut rex(
        &shop,
        &[
            "job",
            "close",
            &research,
            "--status",
            "delivered",
            "--outcome",
            "Report written",
        ],
    ));
    assert_eq!(
        closed,
        format!("job={research}\nstatus=delivered\ndeliveries=0\n")
    );
}
