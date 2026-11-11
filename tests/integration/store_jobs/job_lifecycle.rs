//! A job's life through the `rex` binary: opened, planned, closed and shown,
//! with the hooks absent, from the main checkout and from worktrees, the
//! delivery outcomes `merged`, `blocked` and `kept-local`, the skills a role
//! was assigned and never opened, the skills a run opened beyond them, and the
//! runs whose final message is long.

use std::fs;

use serde_json::{Value, json};

use super_rex::store::StoreError;

use super::fixture::{
    COMMIT, PLAN, SESSION, close, create_args, new_repo, new_store, new_worktree, open_job, plan,
    rex, show_json, stderr_of, stdout_of, value,
};

/// The plan of the fixture, with skills for each role.
const ROLE_SKILLS_PLAN: &str = r#"{
  "orchestrator": "pipeline",
  "skills": ["rex-code-tdd", "rex-code-philosophy", "rex-code-tdd"],
  "breakdown": "1. Reject zero.\n2. Test it.",
  "roles": [
    {"role": "engineer", "intelligence": "high", "reliability": "high", "consequence": "high", "verification": "low", "skills": ["rex-code-tdd", "rex-code-philosophy"]},
    {"role": "reviewer", "intelligence": "medium", "reliability": "medium", "consequence": "medium", "verification": "high", "skills": ["rex-code-philosophy"]}
  ]
}"#;

#[test]
fn job_is_opened_planned_closed_and_shown_from_a_worktree() {
    let temp = tempfile::tempdir().unwrap();
    let main = new_repo(temp.path(), "shop");
    let mut store = new_store(&main, "pipeline");

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
    fs::write(&plan_file, ROLE_SKILLS_PLAN).unwrap();
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
             role.reviewer=model-medium-effort-medium\n"
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
    // The plan file is checked before the job's status.
    let refused = |from: &str, to: &str| {
        let plan = ROLE_SKILLS_PLAN.replacen(from, to, 1);
        stderr_of(
            rex(&worktree, &["job", "plan", &job, "--plan-file", "-"]).write_stdin(plan),
            1,
        )
    };
    assert_eq!(
        refused(r#"["rex-code-philosophy"]"#, r#"["rex-code-smells"]"#),
        "error: skill rex-code-smells of role reviewer in plan file - is not in the job's \
         \"skills\"; add it to \"skills\", or remove it from the role\n"
    );
    assert_eq!(
        refused(
            r#"["rex-code-philosophy"]"#,
            r#"["rex-code-tdd/mocking.md"]"#
        ),
        "error: skill name rex-code-tdd/mocking.md in plan file - is not valid; give the skill \
         folder name, lower-case letters and digits in groups joined by single hyphens, such as \
         rex-code-tdd for rex-code-tdd/mocking.md\n"
    );
    assert_eq!(
        refused(
            r#"["rex-code-philosophy"]"#,
            r#"["rex-code-philosophy", "rex-code-philosophy"]"#
        ),
        "error: role reviewer in plan file - names skill rex-code-philosophy twice; give each \
         skill once\n"
    );
    // A role always gets a grid agent: the plan file has no form for a role
    // that the session runs itself.
    let session_role = refused(
        r#"{"role": "reviewer","#,
        r#"{"role": "reviewer", "session": true,"#,
    );
    assert!(
        session_role.contains("unknown field `session`"),
        "{session_role}"
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

    // One engineer run that opened one of its two skills and one skill it was
    // not given, and no reviewer run. With no transcript the run has no
    // tokens, so the record is incomplete. Its final message has 2,001
    // characters in 4,002 bytes, so it is long; a second engineer run's
    // message of 2,000 characters is not. A decision-maker run, which the
    // plan does not hold, read the builder's text and the orchestrator that
    // the builder gives it, so it opened no skill beyond its brief.
    let extra = main.join(".rex/skills/rex-code-smells");
    fs::create_dir_all(&extra).unwrap();
    fs::write(
        extra.join("SKILL.md"),
        "---\nname: rex-code-smells\n---\n\nFind the smells in five words.\n",
    )
    .unwrap();
    store
        .write(|tx| {
            tx.execute(
                "INSERT INTO sub_agent_run (job_id, session_id, agent_id, depth, role,
                                            skills_opened, final_message, first_seen_at,
                                            started_at, stopped_at)
                 VALUES (?1, ?2, 'a1b8', 1, 'engineer', 'rex-code-smells,rex-code-tdd', ?3,
                         strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                         strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                         strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
                        (?1, ?2, 'a1b9', 1, 'engineer', 'rex-code-tdd', ?4,
                         strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                         strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                         strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
                        (?1, ?2, 'a1c0', 1, 'decision-maker', 'orchestration-builder,pipeline',
                         NULL,
                         strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                         strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                         strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
                [&job, SESSION, &"é".repeat(2001), &"e".repeat(2000)],
            )
            .map_err(StoreError::from)
        })
        .unwrap();
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
            "job={job}\nstatus=delivered\ndeliveries=1\ndelivery.1=merged {}\n\
             missed_skills=1\nmissed_skill.1=engineer rex-code-philosophy\n\
             role_not_run.1=reviewer\nextra_skills=1\n\
             extra_skill.1=engineer rex-code-smells skill_md_words=10\nskills_incomplete=yes\n\
             long_messages=1\nlong_message.1=engineer a1b8 chars=2001\n",
            main.display()
        )
    );

    // What `rex recall` stored is printed whole, however long it is.
    let lesson = "Check the quantity at the API edge, not only in the client. ".repeat(40);
    let recalled = format!(
        r#"{{"lessons":[{{"feedback_id":3,"job":"0g30kh5n1w","score":3,"lesson":"{lesson}"}}],"orchestrators":[{{"name":"pipeline","version":1,"best_match":3}}],"unused":[]}}"#
    );
    store
        .write(|tx| {
            tx.execute(
                "UPDATE job SET experience_recalled = ?1,
                                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 WHERE id = ?2",
                [&recalled, &job],
            )
            .map_err(StoreError::from)
        })
        .unwrap();

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
             created_at={created_at}\nupdated_at={updated_at}\ntokens_input=none\n\
             tokens_cache_write=none\ntokens_cache_read=none\ntokens_output=none\n\
             tokens_cache_hit=none\ncost_usd=none\ncost_unpriced_requests=none\ntokens_provisional=no\nincomplete=yes\noutcome=Zero is rejected\n\
             role.engineer=model-medium-effort-high inputs=-LHHHHL demand=model-xhigh-effort-max \
             rules=T1,T2,E1,E2,E4,C1,C2,F1 conflict=budget oms_version=2 \
             skills=rex-code-tdd,rex-code-philosophy\n\
             role.reviewer=model-medium-effort-medium inputs=-LMHMMH \
             demand=model-medium-effort-medium rules=T1,E1,E3,E4 conflict=none oms_version=2 \
             skills=rex-code-philosophy\n\
             delivery.1=merged branch=agent-a1b8 pr=none merge_agreed=yes merge_commit={COMMIT} \
             repo={}\n\
             session input=none cache_write=none cache_read=none output=none cache_hit=none \
             cost_usd=none unpriced_requests=none provisional=no\n\
             run.a1b8=finished parent=main role=engineer agent=none ran=none/none \
             skills=rex-code-smells,rex-code-tdd input=none cache_write=none cache_read=none output=none \
             cache_hit=none cost_usd=none \
             unpriced_requests=none provisional=no\n\
             run.a1b9=finished parent=main role=engineer agent=none ran=none/none \
             skills=rex-code-tdd input=none cache_write=none cache_read=none output=none \
             cache_hit=none cost_usd=none unpriced_requests=none provisional=no\n\
             run.a1c0=finished parent=main role=decision-maker agent=none ran=none/none \
             skills=orchestration-builder,pipeline input=none cache_write=none cache_read=none \
             output=none cache_hit=none cost_usd=none unpriced_requests=none provisional=no\n\
             request:\n  Reject a quantity of zero.\nbreakdown:\n  1. Reject zero.\n  2. Test it.\n\
             experience_recalled:\n  {recalled}\n",
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
    assert_eq!(
        record["roles"][0]["skills"],
        json!(["rex-code-tdd", "rex-code-philosophy"])
    );
    assert_eq!(record["deliveries"][0]["merge_agreed"], true);
    assert_eq!(
        record["runs"][0]["skills_opened"],
        json!(["rex-code-smells", "rex-code-tdd"])
    );
    assert_eq!(record["job"]["experience_recalled"], recalled.as_str());

    // The --json form of create, plan and close, on a second job.
    let created = stdout_of(
        rex(
            &main,
            &create_args(&[
                "--merge", "yes", "--type", "fix", "--lang", "rust", "--json",
            ]),
        )
        .env("CLAUDE_CODE_SESSION_ID", "json-session")
        .write_stdin("Show the error to the user.\n"),
    );
    let created: Value = serde_json::from_str(&created).unwrap();
    assert_eq!(keys(&created), ["schema", "job", "status", "existing"]);
    assert_eq!(
        (&created["status"], &created["existing"]),
        (&json!("created"), &json!(false))
    );
    let second = created["job"].as_str().unwrap();
    let planned = stdout_of(
        rex(
            &worktree,
            &["job", "plan", second, "--plan-file", "-", "--json"],
        )
        .write_stdin(ROLE_SKILLS_PLAN),
    );
    let planned: Value = serde_json::from_str(&planned).unwrap();
    assert_eq!(
        keys(&planned),
        ["schema", "job", "status", "orchestrator", "roles"]
    );
    assert_eq!(
        planned["orchestrator"],
        json!({"name": "pipeline", "version": 1})
    );
    let role_keys = [
        "role",
        "agent",
        "demand",
        "rules",
        "conflict",
        "oms_version",
    ];
    assert_eq!(keys(&planned["roles"][0]), role_keys);
    assert_eq!(planned["roles"][0]["conflict"], json!([]));
    let deliveries =
        format!(r#"[{{"repo": ".", "merge_commit": "{COMMIT}", "outcome": "merged"}}]"#);
    let closed = stdout_of(
        rex(
            &worktree,
            &[
                "job",
                "close",
                second,
                "--status",
                "delivered",
                "--outcome",
                "Shown",
                "--deliveries-file",
                "-",
                "--json",
            ],
        )
        .write_stdin(deliveries),
    );
    let closed: Value = serde_json::from_str(&closed).unwrap();
    assert_eq!(
        keys(&closed),
        [
            "schema",
            "job",
            "status",
            "deliveries",
            "missed_skills",
            "roles_not_run",
            "extra_skills",
            "skills_incomplete",
            "long_messages"
        ]
    );
    assert_eq!(
        closed["deliveries"],
        json!([{"outcome": "merged", "repo": main.to_str().unwrap()}])
    );
    // No run at all: each role is named once, and no skill is listed.
    assert_eq!(closed["missed_skills"], json!([]));
    assert_eq!(closed["roles_not_run"], json!(["engineer", "reviewer"]));
    assert_eq!(closed["extra_skills"], json!([]));
    assert_eq!(closed["skills_incomplete"], false);
    assert_eq!(closed["long_messages"], json!([]));
}

/// The keys of a JSON object, in the order they were printed.
fn keys(object: &Value) -> Vec<&str> {
    object
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect()
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
