//! Lost and late facts: events that never came, and a transcript record written
//! after the read. Each gives a correct history, or one marked incomplete.

use std::fs;

use super_rex::capture::{JobRuns, TokenTotal};

use super::fixture::*;

// A run whose start or end never reached the store, because the plugin was not
// loaded or its batch was lost, still counts: its transcript is the trusted source.
#[test]
fn lost_events_give_the_same_tokens() {
    let recorded = runs_without(&[]);
    let [start, stop] = [0, 1];
    for lost in [vec![start], vec![stop], vec![start, stop]] {
        let runs = runs_without(&lost);
        assert!(!runs.incomplete, "events {lost:?} of LEAD lost");
        assert!(agent_ids(&runs).contains(&LEAD), "{:?}", agent_ids(&runs));
        assert_eq!(
            run(&runs, LEAD).tokens,
            run(&recorded, LEAD).tokens,
            "events {lost:?} lost"
        );
        assert_eq!(runs.tokens, recorded.tokens, "events {lost:?} lost");
        let mut others = runs.runs.clone();
        others.retain(|run| run.agent_id != LEAD);
        let mut recorded_others = recorded.runs.clone();
        recorded_others.retain(|run| run.agent_id != LEAD);
        assert_eq!(others, recorded_others, "events {lost:?} lost");
    }

    // The metadata files of LEAD and WORKER lost: nothing of what the plugin
    // reported is lost with them.
    let mut fixture = fixture_without(&[]);
    for agent in [LEAD, WORKER] {
        fs::remove_file(fixture.metadata(agent)).unwrap();
    }
    let runs = fixture.read(JOB_A, AFTER_SESSION);
    assert!(!runs.incomplete, "metadata files lost");
    for (now, was) in runs.runs.iter().zip(&recorded.runs) {
        assert_eq!(now.tokens, was.tokens, "{} tokens", now.agent_id);
    }

    // A run whose transcript was never read has no list of skills, which is
    // not the empty list of a read transcript that opened none.
    let mut fixture = fixture_without(&[]);
    let transcript = fixture.transcript(WORKER);
    fs::remove_file(&transcript).unwrap();
    fs::create_dir(&transcript).unwrap();
    let runs = fixture.read(JOB_A, AFTER_SESSION);
    assert_eq!(run(&runs, WORKER).skills_opened, None);
    assert_eq!(run(&recorded, WORKER).skills_opened, Some(vec![]));

    // Both events lost, and the transcript that would recover the run cannot be
    // read: the run is not found, and the history is marked.
    let mut fixture = fixture_without(&[start, stop]);
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

/// The runs of the recorded session read after it, with LEAD's events at `lost`
/// never sent.
fn runs_without(lost: &[usize]) -> JobRuns {
    fixture_without(lost).read(JOB_A, AFTER_SESSION)
}

/// The recorded session sent through `rex ingest`, with LEAD's events at `lost`
/// (0 = start, 1 = stop) left out.
fn fixture_without(lost: &[usize]) -> Fixture {
    let mut fixture = Fixture::new();
    fixture.open_job(JOB_A, BEFORE_SESSION).unwrap();
    let lead_lines = fixture.lines_of(LEAD);
    let lost: Vec<String> = lost
        .iter()
        .map(|index| lead_lines[*index].clone())
        .collect();
    let sent: Vec<String> = fixture
        .lines
        .iter()
        .filter(|line| !lost.contains(line))
        .cloned()
        .collect();
    fixture.append(&sent);
    fixture
}
