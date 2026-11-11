//! Integration tests of `rex recall`, `rex feedback` and `rex jobs`.

mod support;

use support::Fixture;

const JOB_A: &str = "0g4aaaaaa1";
const JOB_B: &str = "0g4bbbbbb2";
const JOB_C: &str = "0g4cccccc3";
const JOB_D: &str = "0g4dddddd4";
const JOB_E: &str = "0g4eeeeee5";
const JOB_F: &str = "0g4ffffff6";
const JOB_G: &str = "0g4gggggg7";
const JOB_H: &str = "0g4hhhhhh8";

fn reject_with_lesson(fixture: &Fixture, job: &str, lesson: &str) {
    let run = fixture.rex(&[
        "feedback",
        job,
        "--verdict",
        "rejected",
        "--said",
        "Not what I asked for.",
        "--cause",
        "missing-context",
        "--lesson",
        lesson,
    ]);
    assert_eq!(run.code, 0, "{}", run.stderr);
}

#[test]
fn feedback_on_one_job_is_recalled_for_a_similar_job() {
    let mut fixture = Fixture::new();
    fixture.orchestrator_that_ran("lead-and-verifier", "A lead and a verifier.");
    fixture.orchestrator_that_ran("lean-pair", "Two agents, no review.");
    fixture.orchestrator_that_ran("costly-yes", "Five agents, two reviews.");
    fixture.orchestrator_folder("research-panel", "Three readers compare options.");

    // A equals B on all four fields, C on one, D on exactly two.
    fixture
        .add_job(JOB_A)
        .class("feature", "small", "low", "rust")
        .status("delivered")
        .orchestrator("lead-and-verifier", 1)
        .insert();
    fixture
        .add_job(JOB_B)
        .class("feature", "small", "low", "rust")
        .insert();
    fixture
        .add_job(JOB_C)
        .class("feature", "large", "high", "go")
        .status("delivered")
        .insert();
    fixture
        .add_job(JOB_D)
        .class("feature", "small", "high", "go")
        .status("delivered")
        .insert();
    fixture
        .add_job(JOB_E)
        .status("planned")
        .experience_recalled("kept")
        .insert();
    // Accepted beats missed at any cost: H's orchestrator comes first. F has
    // the same results as A and less cost, but with a request on a model with
    // no price, so its cost is only a lower bound and it comes after A, though
    // G gives it more runs. G is still planned: its cost stays out of the mean.
    fixture
        .add_job(JOB_H)
        .class("feature", "small", "low", "rust")
        .status("accepted")
        .orchestrator("costly-yes", 1)
        .insert();
    fixture
        .add_job(JOB_G)
        .class("feature", "small", "low", "rust")
        .status("planned")
        .orchestrator("lean-pair", 1)
        .insert();
    fixture
        .add_job(JOB_F)
        .class("feature", "small", "low", "rust")
        .status("analyzed")
        .orchestrator("lean-pair", 1)
        .insert();
    // Cost and tokens add up over the main session and each run.
    fixture.add_session_tokens(JOB_H, (2_000_000_000, 0), [0, 0, 100]);
    fixture.add_session_tokens(JOB_G, (9_000_000_000, 0), [100, 0, 0]);
    fixture.add_session_tokens(JOB_A, (300_000_000, 0), [10, 20, 70]);
    fixture.add_run_tokens(JOB_A, "a1", 200_000_000, [0, 0, 100]);
    fixture.add_session_tokens(JOB_F, (120_000_000, 1), [50, 0, 50]);
    reject_with_lesson(&fixture, JOB_A, "Ask about the config format.");
    reject_with_lesson(&fixture, JOB_C, "Lesson of the job that shares one field.");
    reject_with_lesson(&fixture, JOB_D, "Lesson of the job that shares two fields.");

    let run = fixture.rex(&["recall", JOB_B]);

    let library = fixture.paths.orchestrators.display();
    let expected = [
        format!("job={JOB_B}"),
        "recorded=yes".to_owned(),
        "lessons=2".to_owned(),
        format!(
            "lesson feedback=1 job={JOB_A} score=4 status=analyzed cost_usd=0.500000 cache_hit=85% causes=missing-context text=Ask about the config format."
        ),
        format!(
            "lesson feedback=3 job={JOB_D} score=2 status=analyzed cost_usd=none cache_hit=none causes=missing-context text=Lesson of the job that shares two fields."
        ),
        "orchestrators=3".to_owned(),
        format!(
            "orchestrator name=costly-yes version=1 best_match=4 runs=1 accepted=1 missed=0 cost_per_job_usd=2.000000 cache_hit=100% path={library}/costly-yes/SKILL.md"
        ),
        format!(
            "orchestrator name=lead-and-verifier version=1 best_match=4 runs=1 accepted=0 missed=1 cost_per_job_usd=0.500000 cache_hit=85% path={library}/lead-and-verifier/SKILL.md"
        ),
        format!(
            "orchestrator name=lean-pair version=1 best_match=4 runs=2 accepted=0 missed=1 cost_per_job_usd=≥0.120000 cache_hit=50% path={library}/lean-pair/SKILL.md"
        ),
        "unused=1".to_owned(),
        format!(
            "unused name=research-panel version=1 path={library}/research-panel/SKILL.md description=Three readers compare options."
        ),
    ];
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(run.stdout_lines(), expected);

    // A job still being created has its recall recorded, each time it is run.
    let json = fixture.rex(&["recall", JOB_B, "--json"]);
    assert_eq!(json.code, 0, "{}", json.stderr);
    assert!(
        json.stdout.contains(r#""recorded":true"#)
            && json
                .stdout
                .contains(r#""cost_per_job_usd":0.12,"unpriced_requests":1,"cache_hit":50"#),
        "{}",
        json.stdout
    );
    assert_eq!(
        fixture.job_column(JOB_B, "experience_recalled").as_deref(),
        Some(json.stdout.trim_end())
    );

    // A planned job keeps the recall it has: it is shown, not overwritten.
    let planned = fixture.rex(&["recall", JOB_E]);
    assert_eq!(
        planned.stdout_lines()[..2],
        [format!("job={JOB_E}"), "recorded=no".to_owned()]
    );
    assert_eq!(
        fixture.job_column(JOB_E, "experience_recalled").as_deref(),
        Some("kept")
    );
}

/// What `rex feedback` printed on success: the four lines as `previous -> status`.
fn moved(run: &support::Run) -> String {
    assert_eq!(run.code, 0, "{}", run.stderr);
    let lines = run.stdout_lines();
    let value = |index: usize| lines[index].split_once('=').unwrap().1.to_owned();
    format!("{} -> {}", value(2), value(3))
}

#[test]
fn feedback_moves_status_by_the_table() {
    let mut fixture = Fixture::new();
    for (job, status) in [
        (JOB_A, "delivered"),
        (JOB_B, "delivered"),
        (JOB_C, "delivered"),
        (JOB_D, "planned"),
    ] {
        fixture.add_job(job).status(status).insert();
    }
    let said = ["--said", "As the user said."];
    let analysis = [
        "--cause",
        "agent-skill",
        "--lesson",
        "Pick the skill first.",
    ];

    let accepted =
        fixture.rex(&[&["feedback", JOB_A, "--verdict", "accepted"], &said[..]].concat());
    assert_eq!(moved(&accepted), "delivered -> accepted");
    assert_eq!(
        accepted.stdout_lines()[..2],
        [format!("job={JOB_A}"), "feedback=1".to_owned()]
    );

    let rejected = ["feedback", JOB_B, "--verdict", "rejected"];
    let with_analysis = fixture.rex(&[&rejected[..], &said, &analysis].concat());
    assert_eq!(moved(&with_analysis), "delivered -> analyzed");

    // The two-call path: a bare verdict, then the analysis. A second bare
    // verdict is refused and leaves no row.
    let rejected = ["feedback", JOB_C, "--verdict", "rejected"];
    assert_eq!(
        moved(&fixture.rex(&[&rejected[..], &said].concat())),
        "delivered -> rejected"
    );
    let due = fixture.rex(&[&rejected[..], &said].concat());
    assert_eq!(due.code, 1);
    assert_eq!(
        due.stderr.trim_end(),
        format!(
            "error: job {JOB_C} is rejected and its analysis is due; give --cause and --lesson with --verdict rejected"
        )
    );
    assert_eq!(fixture.feedback_rows(JOB_C), 1);
    assert_eq!(
        moved(&fixture.rex(&[&rejected[..], &said, &analysis].concat())),
        "rejected -> analyzed"
    );

    // An analyzed job takes more feedback and keeps its status.
    let again = fixture.rex(&[&["feedback", JOB_C, "--verdict", "accepted"], &said[..]].concat());
    assert_eq!(moved(&again), "analyzed -> analyzed");
    assert_eq!(fixture.feedback_rows(JOB_C), 3);

    // A planned job has no outcome yet, so no row stays.
    let planned = fixture.rex(&[&["feedback", JOB_D, "--verdict", "accepted"], &said[..]].concat());
    assert_eq!(planned.code, 1);
    assert_eq!(
        planned.stderr.trim_end(),
        format!(
            "error: job {JOB_D} is planned and has no final outcome yet; close it with rex job close first"
        )
    );
    assert_eq!(fixture.feedback_rows(JOB_D), 0);
    assert_eq!(fixture.status_of(JOB_D), "planned");

    // Usage errors exit 2: a lesson with an accepted verdict, with and
    // without the cause that clap itself asks for.
    let accepted = ["feedback", JOB_A, "--verdict", "accepted"];
    let rule_1 = fixture.rex(&[&accepted[..], &said, &analysis].concat());
    assert_eq!(rule_1.code, 2);
    assert_eq!(
        rule_1.stderr.trim_end(),
        "error: --cause, --lesson, --evidence, --changed-skill and --changed-orchestrator need --verdict rejected"
    );
    let lesson_only = fixture.rex(&[&accepted[..], &said, &["--lesson", "A lesson."]].concat());
    assert_eq!(lesson_only.code, 2);
    assert_eq!(fixture.feedback_rows(JOB_A), 1);
}

#[test]
fn jobs_json_keeps_its_fixed_shape() {
    let mut fixture = Fixture::new();
    let long = format!("Make the validator reject zero. {}", "x".repeat(200));
    fixture
        .add_job(JOB_A)
        .class("fix", "medium", "high", "go,rust")
        .request(&long)
        .created_at("2026-11-09T16:05:41.330Z")
        .insert();
    fixture
        .add_job(JOB_B)
        .class("feature", "small", "low", "rust")
        .request("Add a --json flag\nto rex status.")
        .created_at("2026-11-10T09:14:03.512Z")
        .status("delivered")
        .insert();
    fixture.add_open_pr(
        JOB_B,
        "/work/widgets",
        "feature/status-json",
        "https://example.test/pr/12",
    );

    let run = fixture.rex(&["jobs", "--json"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(run.stdout_lines().len(), 1);
    let document: serde_json::Value = serde_json::from_str(&run.stdout).unwrap();
    assert_eq!(document["schema"], 1);

    // Newest first; the keys of one job object in their fixed order.
    let jobs = document["jobs"].as_array().unwrap();
    let ids: Vec<_> = jobs.iter().map(|job| job["id"].as_str().unwrap()).collect();
    assert_eq!(ids, [JOB_B, JOB_A]);
    let keys: Vec<_> = jobs[0]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        keys,
        [
            "id",
            "created_at",
            "updated_at",
            "status",
            "stored_status",
            "request",
            "request_truncated",
            "asked_by",
            "class_version",
            "work_type",
            "size",
            "consequence",
            "languages",
            "orchestrator",
            "merge_agreed",
            "builder_model",
            "builder_effort",
            "deliveries",
            "runs",
            "tokens",
            "cost",
            "incomplete",
            "has_feedback",
        ]
    );
    let delivered = &jobs[0];
    assert_eq!(delivered["status"], "delivered");
    assert_eq!(delivered["stored_status"], "delivered");
    assert_eq!(delivered["request"], "Add a --json flag\nto rex status.");
    assert_eq!(delivered["languages"], serde_json::json!(["rust"]));
    assert_eq!(delivered["orchestrator"], serde_json::Value::Null);
    assert_eq!(
        delivered["deliveries"],
        serde_json::json!([{
            "repo": "/work/widgets",
            "branch": "feature/status-json",
            "pr": "https://example.test/pr/12",
            "outcome": "pr-open",
        }])
    );
    assert_eq!(
        delivered["runs"],
        serde_json::json!({"open": 0, "finished": 0, "unconfirmed": 0})
    );
    assert_eq!(delivered["tokens"], serde_json::Value::Null);
    assert_eq!(delivered["cost"], serde_json::Value::Null);
    assert_eq!(delivered["has_feedback"], false);

    // A long request is cut at 200 characters and says so.
    assert_eq!(jobs[1]["request"].as_str().unwrap().chars().count(), 200);
    assert_eq!(jobs[1]["request_truncated"], true);
    assert_eq!(jobs[1]["languages"], serde_json::json!(["go", "rust"]));

    // No match: an empty list, exit 0. Search ignores ASCII case and needs every term.
    let none = fixture.rex(&["jobs", "--json", "--search", "nothing-like-this"]);
    assert_eq!(
        (none.code, none.stdout.as_str()),
        (0, "{\"schema\":1,\"jobs\":[]}\n")
    );
    let found = fixture.rex(&["jobs", "--json", "--search", "JSON", "--search", "STATUS"]);
    assert!(found.stdout.contains(JOB_B) && !found.stdout.contains(JOB_A));

    // Dates are inclusive, on the date part of created_at.
    let day = fixture.rex(&[
        "jobs",
        "--json",
        "--since",
        "2026-11-10",
        "--until",
        "2026-11-10",
    ]);
    assert!(day.stdout.contains(JOB_B) && !day.stdout.contains(JOB_A));
    let reversed = fixture.rex(&[
        "jobs",
        "--json",
        "--since",
        "2026-11-11",
        "--until",
        "2026-11-10",
    ]);
    assert_eq!(
        (reversed.code, reversed.stdout.as_str()),
        (0, "{\"schema\":1,\"jobs\":[]}\n")
    );
    let bad = fixture.rex(&["jobs", "--since", "10/11/2026"]);
    assert_eq!(bad.code, 2);
    assert_eq!(
        bad.stderr.trim_end(),
        "error: --since needs a date as YYYY-MM-DD, not 10/11/2026"
    );

    // Plain form: one line per job, line breaks flattened, nothing when no match.
    let plain = fixture.rex(&["jobs", "--search", "status"]);
    assert_eq!(
        plain.stdout_lines(),
        [format!(
            "{JOB_B} delivered feature small low rust 2026-11-10T09:14:03.512Z Add a --json flag to rex status."
        )]
    );
    assert_eq!(
        fixture
            .rex(&["jobs", "--search", "nothing-like-this"])
            .stdout,
        ""
    );
}
