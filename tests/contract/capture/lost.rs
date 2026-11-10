//! Lost and late facts: lines that never came, and a transcript record written
//! after the read. Each gives a correct history, or one marked incomplete.

use std::fs;

use super_rex::capture::{JobRuns, Run, TokenTotal};

use super::fixture::*;

#[test]
fn lost_lines_give_the_same_tree() {
    let recorded = runs_without(&[]);
    let lead = run(&recorded, LEAD).clone();
    let first_record = Some("2026-11-10T13:10:18.532Z".to_owned());
    let last_record = Some("2026-11-10T13:10:32.660Z".to_owned());

    // Each case names the fields that come from another source when lines are lost,
    // with the value that source gives. Every other field of every run is unchanged.
    let start_lost = Run {
        // The tie time: the first record of the run's transcript.
        started_at: first_record.clone(),
        ..lead.clone()
    };
    let stop_lost = Run {
        // The completed call is the next source of both.
        stopped_at: Some("2026-11-10T13:10:32.727Z".to_owned()),
        final_message: Some(
            "<tools-used-before-summary>Bash, Edit, Read, Write</tools-used-before-summary>\n\nLEAD DONE"
                .to_owned(),
        ),
        ..lead.clone()
    };
    let call_lost = Run {
        // Stop minus start, as no completed call gave it.
        duration_ms: Some(14_136),
        ..lead.clone()
    };
    let all_lost = Run {
        // Recovered from the metadata file and the transcript alone.
        started_at: first_record,
        stopped_at: last_record,
        duration_ms: Some(14_128),
        ..lead.clone()
    };

    let [start, stop, call] = [0, 1, 2];
    for (lost, expected) in [
        (vec![start], start_lost),
        (vec![stop], stop_lost),
        (vec![call], call_lost),
        (vec![start, stop, call], all_lost),
    ] {
        let mut runs = runs_without(&lost);
        assert!(!runs.incomplete, "lines {lost:?} lost");
        assert_eq!(runs.damaged_log_lines, lost.len() as u64);
        assert_eq!((runs.open, runs.finished, runs.unconfirmed), (0, 6, 0));
        assert_eq!(run(&runs, LEAD), &expected, "lines {lost:?} lost");
        runs.runs.retain(|run| run.agent_id != LEAD);
        let mut others = recorded.clone();
        others.runs.retain(|run| run.agent_id != LEAD);
        assert_eq!(runs.runs, others.runs, "lines {lost:?} lost");
    }

    // The metadata files of LEAD and WORKER lost: each depth is one more than the
    // parent's, and the main session counts as depth 0.
    let mut fixture = fixture_without(&[]);
    for agent in [LEAD, WORKER] {
        fs::remove_file(fixture.metadata(agent)).unwrap();
    }
    let runs = fixture.read(JOB_A, AFTER_SESSION);
    assert!(!runs.incomplete, "metadata files lost");
    assert_eq!(runs.runs, recorded.runs, "metadata files lost");

    // All three lines lost, and the transcript that would recover the run cannot be
    // read: the run is not found, and the history is marked.
    let mut fixture = fixture_without(&[start, stop, call]);
    let transcript = fixture.transcript(LEAD);
    fs::remove_file(&transcript).unwrap();
    fs::create_dir(&transcript).unwrap();
    let runs = fixture.read(JOB_A, AFTER_SESSION);
    assert!(!agent_ids(&runs).contains(&LEAD), "{:?}", agent_ids(&runs));
    assert!(runs.incomplete, "unreadable transcript");
}

#[test]
fn late_transcript_record_is_counted_before_final() {
    let recorded = runs_without(&[]);
    let mut fixture = Fixture::new();
    fixture.open_job(JOB_A, BEFORE_SESSION).unwrap();
    fixture.append(&fixture.lines.clone());
    // Claude Code has not yet written the last request of REVIEWER, nor the main
    // session's last request.
    let reviewer_late = cut_last_lines(&fixture.transcript(REVIEWER), 1);
    let main_late = cut_last_lines(&fixture.main_transcript(), 2);

    let inside_close = fixture.read(JOB_A, "2026-11-10T13:10:44.000Z");
    assert!(!inside_close.incomplete);

    // A transcript that cannot be read counts as absent for that read, and marks it.
    let lead_transcript = fixture.transcript(LEAD);
    let lead_text = fs::read(&lead_transcript).unwrap();
    fs::remove_file(&lead_transcript).unwrap();
    fs::create_dir(&lead_transcript).unwrap();
    let unreadable = fixture.read(JOB_A, "2026-11-10T13:10:44.500Z");
    assert_eq!(
        run(&unreadable, LEAD).tokens,
        run(&inside_close, LEAD).tokens
    );
    assert!(unreadable.incomplete, "unreadable transcript");
    fs::remove_dir(&lead_transcript).unwrap();
    fs::write(&lead_transcript, lead_text).unwrap();

    fixture.close_job(JOB_A, "2026-11-10T13:10:45.000Z");
    let reviewer_early = run(&inside_close, REVIEWER).tokens;
    assert!(matches!(reviewer_early, TokenTotal::Provisional(_)));
    assert_ne!(reviewer_early, run(&recorded, REVIEWER).tokens);
    assert_ne!(inside_close.session_tokens, recorded.session_tokens);

    // A total stored before the close never becomes final, however long ago it was read.
    let much_later = fixture.read(JOB_A, "2026-11-10T14:00:00.000Z");
    assert_eq!(run(&much_later, REVIEWER).tokens, reviewer_early);
    assert_eq!(much_later.session_tokens, inside_close.session_tokens);

    append_text(&fixture.transcript(REVIEWER), &reviewer_late);
    append_text(&fixture.main_transcript(), &main_late);
    let first_after = fixture.read(JOB_A, "2026-11-10T14:00:05.000Z");
    assert_eq!(
        run(&first_after, REVIEWER).tokens,
        run(&recorded, REVIEWER).tokens
    );
    assert_eq!(first_after.session_tokens, recorded.session_tokens);
    assert_eq!(first_after.tokens, recorded.tokens);
    let too_soon = fixture.read(JOB_A, "2026-11-10T14:00:14.999Z");
    assert!(matches!(too_soon.tokens, TokenTotal::Provisional(_)));
    // The main session goes on after the job, so its transcript keeps growing.
    let after_the_job = r#"{"type":"assistant","timestamp":"2026-11-10T13:59:00.000Z","message":{"id":"msg_after_the_job","model":"claude-sonnet-5-5","usage":{"input_tokens":5,"output_tokens":5}}}"#;
    append_text(&fixture.main_transcript(), &format!("{after_the_job}\n"));

    let settled = fixture.read(JOB_A, "2026-11-10T14:00:15.000Z");
    let final_of = |total: TokenTotal| match total {
        TokenTotal::Provisional(counts) => TokenTotal::Final(counts),
        other => panic!("the recorded total is provisional, got {other:?}"),
    };
    for (now, was) in settled.runs.iter().zip(&recorded.runs) {
        assert_eq!(now.tokens, final_of(was.tokens), "{} tokens", now.agent_id);
    }
    assert_eq!(settled.session_tokens, final_of(recorded.session_tokens));
    assert_eq!(settled.tokens, final_of(recorded.tokens));
    assert!(!settled.incomplete);
}

/// The runs of the recorded session read after it, with LEAD's lines at `lost` cut short.
fn runs_without(lost: &[usize]) -> JobRuns {
    fixture_without(lost).read(JOB_A, AFTER_SESSION)
}

/// The recorded session in the log, with LEAD's lines at `lost` (0 = start, 1 = stop,
/// 2 = completed call) cut short, as a short write on a full disk leaves them.
fn fixture_without(lost: &[usize]) -> Fixture {
    let mut fixture = Fixture::new();
    fixture.open_job(JOB_A, BEFORE_SESSION).unwrap();
    let lead_lines = fixture.lines_of(LEAD);
    let lost: Vec<&String> = lost.iter().map(|index| &lead_lines[*index]).collect();
    let logged: Vec<&str> = fixture
        .lines
        .iter()
        .map(|line| {
            if lost.contains(&line) {
                &line[..line.len() / 2]
            } else {
                line.as_str()
            }
        })
        .collect();
    fixture.append(&logged);
    fixture
}
