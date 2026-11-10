//! A job's life through the `rex` binary: opened, planned, closed and shown,
//! with the hooks absent, from the main checkout and from worktrees.

use std::fs;
use std::path::Path;

use super_rex::store::StoreError;

use super::fixture::{new_repo, new_store, new_worktree, rex, stderr_of, stdout_of, value};

const SESSION: &str = "4f9c2e1a-7b3d-4e8f-9a6c-1d2e3f4a5b6c";

const COMMIT: &str = "3f2a9c1e8b7d6a5f4e3d2c1b0a9f8e7d6c5b4a39";

const PLAN: &str = r#"{
  "orchestrator": "pipeline",
  "skills": ["rex-code-tdd", "rex-code-philosophy", "rex-code-tdd"],
  "breakdown": "1. Reject zero.\n2. Test it.",
  "roles": [
    {"role": "engineer", "intelligence": "high", "reliability": "high", "consequence": "high", "verification": "low"},
    {"role": "reviewer", "intelligence": "medium", "reliability": "medium", "consequence": "medium", "verification": "high"},
    {"role": "skill-writer", "session": true}
  ]
}"#;

fn create_args<'a>(extra: &[&'a str]) -> Vec<&'a str> {
    let mut args = vec![
        "job",
        "create",
        "--request-file",
        "-",
        "--asked-by",
        "human",
        "--size",
        "medium",
        "--consequence",
        "high",
    ];
    args.extend_from_slice(extra);
    args
}

#[test]
fn job_is_opened_planned_closed_and_shown_from_a_worktree() {
    let temp = tempfile::tempdir().unwrap();
    let main = new_repo(temp.path(), "shop");
    new_store(&main, "pipeline");

    let created = stdout_of(
        rex(
            &main,
            &create_args(&[
                "--merge",
                "yes",
                "--type",
                "fix",
                "--lang",
                "Rust",
                "--lang",
                "typescript",
                "--lang",
                "rust",
                "--budget",
                "low",
                "--autonomy",
                "high",
            ]),
        )
        .env("CLAUDE_CODE_SESSION_ID", SESSION)
        .write_stdin("Reject a quantity of zero.\n"),
    );
    let job = value(&created, "job").to_owned();
    assert_eq!(created, format!("job={job}\nstatus=created\nexisting=no\n"));

    let worktree = new_worktree(&main, temp.path(), "agent-a1b8");
    let plan_file = temp.path().join("plan.json");
    fs::write(&plan_file, PLAN).unwrap();
    let planned = stdout_of(&mut rex(
        &worktree,
        &[
            "job",
            "plan",
            &job,
            "--plan-file",
            plan_file.to_str().unwrap(),
        ],
    ));
    assert_eq!(
        planned,
        format!(
            "job={job}\nstatus=planned\norchestrator=pipeline\norchestrator_version=1\n\
             role.engineer=model-medium-effort-high\nconflict.engineer=budget\n\
             role.reviewer=model-medium-effort-medium\nrole.skill-writer=session\n"
        )
    );

    let shown = stdout_of(rex(&worktree, &["job", "show"]).env("CLAUDE_CODE_SESSION_ID", SESSION));
    assert_eq!(value(&shown, "job"), job);
    assert_eq!(value(&shown, "status"), "planned");
    assert_eq!(value(&shown, "stored_status"), "planned");

    let plan_path = plan_file.to_str().unwrap();
    let refused = stderr_of(
        &mut rex(&worktree, &["job", "plan", &job, "--plan-file", plan_path]),
        1,
    );
    assert_eq!(
        refused,
        format!(
            "error: job {job} is planned, and rex job plan accepts only a created job; a job is \
             planned once, and a builder that got existing=yes records no plan of its own\n"
        )
    );
    let usage = stderr_of(
        rex(
            &main,
            &create_args(&[
                "--merge", "no", "--type", "chore", "--lang", "none", "--lang", "rust",
            ]),
        )
        .env("CLAUDE_CODE_SESSION_ID", "another-session")
        .write_stdin("A second request.\n"),
        2,
    );
    assert!(usage.starts_with("error: --lang none"), "{usage}");

    let deliveries = format!(
        r#"[{{"repo": ".", "branch": "agent-a1b8", "merge_commit": "{COMMIT}", "outcome": "merged"}}]"#
    );
    let closed = stdout_of(
        rex(
            &worktree,
            &[
                "job",
                "close",
                &job,
                "--status",
                "delivered",
                "--outcome",
                "Zero is rejected",
                "--deliveries-file",
                "-",
            ],
        )
        .write_stdin(deliveries),
    );
    assert_eq!(
        closed,
        format!(
            "job={job}\nstatus=delivered\ndeliveries=1\ndelivery.1=merged {}\n",
            main.display()
        )
    );

    let shown = stdout_of(&mut rex(&worktree, &["job", "show", &job]));
    let created_at = value(&shown, "created_at");
    let updated_at = value(&shown, "updated_at");
    assert_eq!(
        shown,
        format!(
            "job={job}\nstatus=delivered\nstored_status=delivered\ntype=fix\nsize=medium\n\
             consequence=high\nlanguages=rust,typescript\nduration=none\nbudget=low\n\
             autonomy=high\nmerge_agreed=yes\nsession={SESSION}\nasked_by=human\n\
             builder_model=none\nbuilder_effort=none\norchestrator=pipeline\n\
             orchestrator_version=1\nskills=rex-code-philosophy,rex-code-tdd\n\
             created_at={created_at}\nupdated_at={updated_at}\ntokens=none\n\
             tokens_provisional=yes\nincomplete=no\noutcome=Zero is rejected\n\
             role.engineer=model-medium-effort-high inputs=-LHHHHL demand=model-xhigh-effort-max \
             rules=T1,T2,E1,E2,E4,C1,C2,F1 conflict=budget oms_version=1\n\
             role.reviewer=model-medium-effort-medium inputs=-LMHMMH \
             demand=model-medium-effort-medium rules=T1,E1,E3,E4 conflict=none oms_version=1\n\
             role.skill-writer=session\n\
             delivery.1=merged branch=agent-a1b8 pr=none merge_agreed=yes merge_commit={COMMIT} \
             repo={}\n\
             request:\n  Reject a quantity of zero.\nbreakdown:\n  1. Reject zero.\n  2. Test it.\n\
             experience_recalled:\n",
            main.display()
        )
    );

    let json = stdout_of(&mut rex(&worktree, &["job", "show", &job, "--json"]));
    let record: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(record["schema"], 1);
    assert_eq!(record["job"]["status"], "delivered");
    assert_eq!(
        record["job"]["languages"],
        serde_json::json!(["rust", "typescript"])
    );
    let events: Vec<&str> = record["transitions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["event"].as_str().unwrap())
        .collect();
    assert_eq!(events, ["job create", "job plan", "job close"]);
    assert_eq!(record["roles"][0]["inputs"]["budget"], "low");
    assert_eq!(record["deliveries"][0]["merge_agreed"], true);
    assert_eq!(record["runs"], serde_json::json!([]));
}

/// Opens a job in `session` and gives its ID.
fn open_job(cwd: &Path, session: &str, extra: &[&str]) -> String {
    let created = stdout_of(
        rex(cwd, &create_args(extra))
            .env("CLAUDE_CODE_SESSION_ID", session)
            .write_stdin("A request.\n"),
    );
    value(&created, "job").to_owned()
}

/// Records `plan` for `job`, given on standard input.
fn plan(cwd: &Path, job: &str, plan: &str) -> String {
    stdout_of(rex(cwd, &["job", "plan", job, "--plan-file", "-"]).write_stdin(plan.to_owned()))
}

#[test]
fn lifecycle_cases_of_challenge_6() {
    let temp = tempfile::tempdir().unwrap();
    let main = new_repo(temp.path(), "notes");
    let mut store = new_store(&main, "pipeline");
    let research_args = ["--merge", "no", "--type", "research", "--lang", "none"];

    // A sub-agent that starts before the plan, such as one writing a skill, is
    // tied to the job but does not move it.
    let research = open_job(&main, "session-1", &research_args);
    store
        .write(|tx| {
            tx.execute(
                "INSERT INTO sub_agent_run (job_id, session_id, agent_id, first_seen_at)
                 VALUES (?1, 'session-1', 'a1b8ec258e0b550ba', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
                [&research],
            )
            .map_err(StoreError::from)
        })
        .unwrap();
    let shown = stdout_of(&mut rex(&main, &["job", "show", &research]));
    assert_eq!(value(&shown, "status"), "created");
    assert_eq!(value(&plan(&main, &research, PLAN), "status"), "planned");

    let closed = stdout_of(&mut rex(
        &main,
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

    // No remote, and the requester declined a merge: the work stays on its branch.
    let local = open_job(
        &main,
        "session-2",
        &["--merge", "no", "--type", "fix", "--lang", "go"],
    );
    plan(&main, &local, PLAN);
    let note = "branch fix-zero keeps the change; the repository has no remote";
    let deliveries = format!(
        r#"[{{"repo": "{}", "branch": "fix-zero", "outcome": "kept-local", "note": "{note}"}}]"#,
        main.display()
    );
    let closed = stdout_of(
        rex(
            &main,
            &[
                "job",
                "close",
                &local,
                "--status",
                "delivered",
                "--outcome",
                "Fixed on a branch",
                "--deliveries-file",
                "-",
            ],
        )
        .write_stdin(deliveries),
    );
    assert_eq!(
        closed,
        format!(
            "job={local}\nstatus=delivered\ndeliveries=1\ndelivery.1=kept-local {}\n",
            main.display()
        )
    );
    let shown = stdout_of(&mut rex(&main, &["job", "show", &local]));
    assert_eq!(
        value(&shown, "delivery.1"),
        format!(
            "kept-local branch=fix-zero pr=none merge_agreed=no merge_commit=none repo={}",
            main.display()
        )
    );
    assert!(
        shown.ends_with(&format!("delivery.1.note:\n  {note}\n")),
        "{shown}"
    );
}

/// `rex job close <job> ...` with the deliveries on standard input.
fn close(cwd: &Path, job: &str, status: &str, deliveries: &str) -> assert_cmd::Command {
    let mut command = rex(
        cwd,
        &[
            "job",
            "close",
            job,
            "--status",
            status,
            "--outcome",
            "Shop merged",
            "--deliveries-file",
            "-",
        ],
    );
    command.write_stdin(deliveries.to_owned());
    command
}

fn show_json(cwd: &Path, job: &str) -> serde_json::Value {
    serde_json::from_str(&stdout_of(&mut rex(cwd, &["job", "show", job, "--json"]))).unwrap()
}

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
}

#[test]
fn create_in_session_with_open_job_returns_it() {
    let temp = tempfile::tempdir().unwrap();
    let main = new_repo(temp.path(), "shop");
    new_store(&main, "pipeline");
    let job = open_job(
        &main,
        SESSION,
        &["--merge", "yes", "--type", "fix", "--lang", "rust"],
    );
    plan(&main, &job, PLAN);

    // A sub-agent's builder in the same session asks for a job of its own.
    let nested = rex(
        &main,
        &create_args(&["--merge", "no", "--type", "chore", "--lang", "none"]),
    )
    .env("CLAUDE_CODE_SESSION_ID", SESSION)
    .write_stdin("A sub-task.\n")
    .output()
    .unwrap();
    assert!(nested.status.success());
    assert_eq!(
        String::from_utf8(nested.stdout).unwrap(),
        format!("job={job}\nstatus=planned\nexisting=yes\n")
    );
    assert_eq!(
        String::from_utf8(nested.stderr).unwrap(),
        format!(
            "note: session {SESSION} already has open job {job}; no job was opened; this work \
             is recorded under it\n"
        )
    );

    let refused = stderr_of(
        rex(&main, &["job", "plan", &job, "--plan-file", "-"]).write_stdin(PLAN),
        1,
    );
    assert!(
        refused.contains("rex job plan accepts only a created job"),
        "{refused}"
    );
    let shown = show_json(&main, &job);
    assert_eq!(shown["transitions"].as_array().unwrap().len(), 2);
    assert_eq!(shown["roles"].as_array().unwrap().len(), 3);
}
