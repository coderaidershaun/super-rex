//! Integration tests of `rex recall`, `rex feedback`, `rex jobs` and `rex watch`.

mod support;

use support::Fixture;

const JOB_A: &str = "0g4aaaaaa1";
const JOB_B: &str = "0g4bbbbbb2";
const JOB_C: &str = "0g4cccccc3";
const JOB_D: &str = "0g4dddddd4";

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
            "lesson feedback=1 job={JOB_A} score=4 status=analyzed causes=missing-context text=Ask about the config format."
        ),
        format!(
            "lesson feedback=3 job={JOB_D} score=2 status=analyzed causes=missing-context text=Lesson of the job that shares two fields."
        ),
        "orchestrators=1".to_owned(),
        format!(
            "orchestrator name=lead-and-verifier version=1 best_match=4 runs=1 accepted=0 missed=1 path={library}/lead-and-verifier/SKILL.md"
        ),
        "unused=1".to_owned(),
        format!(
            "unused name=research-panel version=1 path={library}/research-panel/SKILL.md description=Three readers compare options."
        ),
    ];
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(run.stdout_lines(), expected);
}

#[test]
fn recall_records_while_created_and_not_after_that() {
    let mut fixture = Fixture::new();
    fixture.add_job(JOB_A).insert();
    fixture
        .add_job(JOB_B)
        .status("planned")
        .experience_recalled("kept")
        .insert();

    let plain = fixture.rex(&["recall", JOB_A]);
    let json = fixture.rex(&["recall", JOB_A, "--json"]);
    assert_eq!(
        plain.stdout_lines()[..2],
        [format!("job={JOB_A}"), "recorded=yes".to_owned()]
    );
    assert_eq!(
        json.stdout.trim_end(),
        format!(
            r#"{{"schema":1,"job":"{JOB_A}","recorded":true,"lessons":[],"orchestrators":[],"unused":[]}}"#
        )
    );
    assert_eq!(
        fixture.job_column(JOB_A, "experience_recalled").as_deref(),
        Some(json.stdout.trim_end())
    );

    let planned = fixture.rex(&["recall", JOB_B]);
    let planned_json = fixture.rex(&["recall", JOB_B, "--json"]);
    assert_eq!(
        planned.stdout_lines()[..2],
        [format!("job={JOB_B}"), "recorded=no".to_owned()]
    );
    assert!(
        planned_json.stdout.contains(r#""recorded":false"#),
        "{}",
        planned_json.stdout
    );
    assert_eq!(
        fixture.job_column(JOB_B, "experience_recalled").as_deref(),
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
