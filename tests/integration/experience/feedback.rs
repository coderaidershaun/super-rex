//! `rex feedback` moves a job's status by the one transition table, and prints
//! the status before and after.

use super::support::{Fixture, JOB_A, JOB_B, JOB_C, JOB_D, Run};

/// What `rex feedback` printed on success: the four lines as `previous -> status`.
fn moved(run: &Run) -> String {
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
