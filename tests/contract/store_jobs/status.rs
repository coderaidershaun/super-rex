//! What `status::apply` promises to the job commands and to `rex feedback`:
//! each row of the one transition table, with the history rows it writes.

use rusqlite::params;
use super_rex::store::status::{self, Change, Closed, Event, Status, Verdict};
use super_rex::store::{Accepts, JobId, Store, StoreError, StorePaths};

const SEEDED_AT: &str = "2000-01-01T00:00:00.000Z";

const ALL: [Status; 9] = [
    Status::Created,
    Status::Planned,
    Status::Blocked,
    Status::Failed,
    Status::Delivered,
    Status::Accepted,
    Status::Rejected,
    Status::Analyzed,
    Status::Cancelled,
];

const CLOSES: [Closed; 4] = [
    Closed::Delivered,
    Closed::Blocked,
    Closed::Failed,
    Closed::Cancelled,
];

const ACCEPTED: Event = Event::Feedback {
    verdict: Verdict::Accepted,
    has_analysis: false,
};
const REJECTED: Event = Event::Feedback {
    verdict: Verdict::Rejected,
    has_analysis: false,
};
const ANALYZED: Event = Event::Feedback {
    verdict: Verdict::Rejected,
    has_analysis: true,
};

#[derive(Debug)]
enum Expect {
    /// The statuses the job steps through, one history row each. Empty: the
    /// status stays and nothing is written.
    Steps(Vec<Status>),
    Refused {
        event: &'static str,
        accepts: Accepts,
    },
    NoOutcomeYet,
    AnalysisDue,
}

struct Case {
    before: Status,
    event: Event,
    expect: Expect,
}

fn rows(befores: &[Status], events: &[Event], expect: impl Fn(Event) -> Expect) -> Vec<Case> {
    let mut cases = Vec::new();
    for &before in befores {
        for &event in events {
            cases.push(Case {
                before,
                event,
                expect: expect(event),
            });
        }
    }
    cases
}

fn closed_status(event: Event) -> Status {
    match event {
        Event::Close(closed) => closed.into(),
        other => panic!("not a close: {other:?}"),
    }
}

/// One group of cases for each row of the table, in the table's order.
fn transition_table() -> Vec<Case> {
    use Status::*;
    let closes: Vec<Event> = CLOSES.into_iter().map(Event::Close).collect();
    let feedbacks = [
        ACCEPTED,
        REJECTED,
        ANALYZED,
        Event::Feedback {
            verdict: Verdict::Accepted,
            has_analysis: true,
        },
    ];
    let not_created: Vec<Status> = ALL.into_iter().filter(|s| *s != Created).collect();
    let not_delivered = [
        Event::Close(Closed::Blocked),
        Event::Close(Closed::Failed),
        Event::Close(Closed::Cancelled),
    ];

    let mut cases = Vec::new();
    cases.extend(rows(&[Created], &[Event::Plan], |_| {
        Expect::Steps(vec![Planned])
    }));
    cases.extend(rows(&not_created, &[Event::Plan], |_| Expect::Refused {
        event: "rex job plan",
        accepts: Accepts::CreatedJob,
    }));
    cases.extend(rows(&[Created], &not_delivered, |event| {
        Expect::Steps(vec![closed_status(event)])
    }));
    cases.extend(rows(&[Created], &[Event::Close(Closed::Delivered)], |_| {
        Expect::Refused {
            event: "rex job close --status delivered",
            accepts: Accepts::PlannedOrBlockedJob,
        }
    }));
    cases.extend(rows(&[Planned, Blocked], &closes, |event| {
        Expect::Steps(vec![closed_status(event)])
    }));
    cases.extend(rows(
        &[Delivered, Failed, Cancelled, Accepted, Rejected, Analyzed],
        &closes,
        |_| Expect::Refused {
            event: "rex job close",
            accepts: Accepts::OpenOrBlockedJob,
        },
    ));
    cases.extend(rows(&[Created, Planned], &feedbacks, |_| {
        Expect::NoOutcomeYet
    }));
    cases.extend(rows(&[Delivered], &[ACCEPTED], |_| {
        Expect::Steps(vec![Accepted])
    }));
    cases.extend(rows(&[Accepted], &[ACCEPTED], |_| Expect::Steps(vec![])));
    cases.extend(rows(&[Delivered, Accepted], &[REJECTED], |_| {
        Expect::Steps(vec![Rejected])
    }));
    cases.extend(rows(&[Delivered, Accepted], &[ANALYZED], |_| {
        Expect::Steps(vec![Rejected, Analyzed])
    }));
    cases.extend(rows(&[Rejected], &[ANALYZED], |_| {
        Expect::Steps(vec![Analyzed])
    }));
    cases.extend(rows(&[Rejected], &[REJECTED, ACCEPTED], |_| {
        Expect::AnalysisDue
    }));
    cases.extend(rows(
        &[Blocked, Failed, Cancelled, Analyzed],
        &feedbacks,
        |_| Expect::Steps(vec![]),
    ));
    cases
}

fn event_name(event: Event) -> &'static str {
    match event {
        Event::Plan => "job plan",
        Event::Close(_) => "job close",
        Event::Feedback { .. } => "feedback",
    }
}

fn seed_job(store: &mut Store, id: &str, status: Status) {
    store
        .write(|tx| {
            tx.execute(
                "INSERT INTO job (id, request, asked_by, class_version, work_type, size, consequence,
                                  languages, merge_agreed, session_id, status, created_at, updated_at)
                 VALUES (?1, 'fix the parser', 'human', 1, 'fix', 'small', 'low', 'rust', 1,
                         'session-' || ?1, ?2, ?3, ?3)",
                params![id, status.to_string(), SEEDED_AT],
            )?;
            Ok::<_, StoreError>(())
        })
        .unwrap();
}

type Transition = (Option<String>, String, String, String);

fn stored(store: &Store, id: &str) -> (String, String, Vec<Transition>) {
    let (status, updated_at) = store
        .conn()
        .query_row(
            "SELECT status, updated_at FROM job WHERE id = ?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    let mut select = store
        .conn()
        .prepare("SELECT from_status, to_status, event, at FROM job_transition WHERE job_id = ?1 ORDER BY id")
        .unwrap();
    let transitions = select
        .query_map([id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    (status, updated_at, transitions)
}

fn check(case: &Case, id: &str, result: Result<Change, StoreError>, store: &Store) {
    let what = format!("{:?} on {}", case.event, case.before);
    let (status, updated_at, transitions) = stored(store, id);
    let before = case.before.to_string();

    let Expect::Steps(steps) = &case.expect else {
        let error = result.expect_err(&what);
        let matched = match (&case.expect, &error) {
            (
                Expect::Refused { event, accepts },
                StoreError::Refused {
                    job,
                    status,
                    event: refused_event,
                    accepts: refused_accepts,
                },
            ) => {
                job == id
                    && *status == before
                    && refused_event == event
                    && refused_accepts == accepts
            }
            (Expect::NoOutcomeYet, StoreError::NoOutcomeYet { job, status }) => {
                job == id && *status == before
            }
            (Expect::AnalysisDue, StoreError::AnalysisDue { job }) => job == id,
            _ => false,
        };
        assert!(matched, "{what}: expected {:?}, got {error:?}", case.expect);
        assert_eq!(status, before, "{what}: a refusal keeps the status");
        assert_eq!(updated_at, SEEDED_AT, "{what}: a refusal writes nothing");
        assert!(transitions.is_empty(), "{what}: a refusal writes no row");
        return;
    };

    let after = steps.last().copied().unwrap_or(case.before);
    let change = result.unwrap_or_else(|error| panic!("{what}: {error}"));
    assert_eq!(
        change,
        Change {
            previous: case.before,
            current: after
        },
        "{what}"
    );
    assert_eq!(status, after.to_string(), "{what}");

    let mut from = before.clone();
    let expected: Vec<(Option<String>, String, String)> = steps
        .iter()
        .map(|step| {
            let row = (
                Some(from.clone()),
                step.to_string(),
                event_name(case.event).to_owned(),
            );
            from = step.to_string();
            row
        })
        .collect();
    let written: Vec<(Option<String>, String, String)> = transitions
        .iter()
        .map(|(from, to, event, _)| (from.clone(), to.clone(), event.clone()))
        .collect();
    assert_eq!(written, expected, "{what}: history rows");

    match transitions.first() {
        None => assert_eq!(updated_at, SEEDED_AT, "{what}: a stay writes nothing"),
        Some((_, _, _, at)) => {
            assert!(
                transitions.iter().all(|row| &row.3 == at),
                "{what}: the rows of one call share one time"
            );
            assert_eq!(
                &updated_at, at,
                "{what}: updated_at is the time of the change"
            );
        }
    }
}

#[test]
fn every_row_of_the_transition_table() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("repo").join(".rex");
    let paths = StorePaths {
        root: temp.path().join("repo"),
        db: dir.join("rex.db"),
        event_log: dir.join("events.jsonl"),
        orchestrators: dir.join("orchestrators"),
        dir,
    };
    let mut store = Store::create(&paths).unwrap();

    let cases = transition_table();
    for (number, case) in cases.iter().enumerate() {
        let id = format!("job{number:07}");
        seed_job(&mut store, &id, case.before);
        let job: JobId = id.parse().unwrap();
        // The closure keeps whatever `apply` wrote even when it refuses, so a
        // refusal that wrote something would show below.
        let result = store
            .write(|tx| Ok::<_, StoreError>(status::apply(tx, &job, case.event)))
            .unwrap();
        check(case, &id, result, &store);
    }

    let missing: JobId = "0g4cbf7k2q".parse().unwrap();
    let result = store.write(|tx| status::apply(tx, &missing, Event::Plan));
    match result {
        Err(StoreError::JobNotFound { job, store: dir }) => {
            assert_eq!(job, "0g4cbf7k2q");
            assert_eq!(dir.file_name().unwrap(), ".rex");
        }
        other => panic!("expected JobNotFound, got {other:?}"),
    }
}
