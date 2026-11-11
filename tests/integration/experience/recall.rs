//! `rex recall` gives the lessons that `rex feedback` recorded on similar jobs,
//! with the orchestrators and the cost of those jobs.

use super::support::{Fixture, JOB_A, JOB_B, JOB_C, JOB_D, JOB_E, JOB_F, JOB_G, JOB_H};

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
