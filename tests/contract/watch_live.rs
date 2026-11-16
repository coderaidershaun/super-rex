//! What the mod's live snapshot promises to `rex watch`: the shared fixture, in
//! exactly the form the mod writes, loads through the binary's own loader.

use super_rex::commands::watch::data::Origin;
use super_rex::commands::watch::live::{Snapshot, epoch_ms};

const FIXTURE: &str = include_str!("../../fixtures/shared/live/snapshot.json");

#[test]
fn shared_snapshot_fixture_loads_through_the_watch_loader() {
    let snapshot =
        Snapshot::parse(FIXTURE).expect("the fixture is no longer the form rex watch reads");
    let written = epoch_ms(&snapshot.written_at)
        .expect("written_at is not of the form YYYY-MM-DDTHH:MM:SS.mmmZ");

    let part = snapshot.part(written);

    assert_eq!(part.trees.len(), 1);
    assert_eq!(part.jobs.len(), 2);
    let runs = part.jobs.iter().map(|job| job.runs.len()).sum::<usize>()
        + part
            .trees
            .iter()
            .filter(|tree| tree.master_run.is_some())
            .count();
    assert_eq!(runs, snapshot.runs.len());
    let questions = part
        .jobs
        .iter()
        .map(|job| job.questions.len())
        .sum::<usize>()
        + part
            .trees
            .iter()
            .map(|tree| tree.questions.len())
            .sum::<usize>();
    assert_eq!(questions, snapshot.questions.len());
    assert!(part.jobs.iter().all(|job| job.origin == Origin::Live));
    for time in snapshot.runs.iter().map(|run| &run.started_at) {
        assert!(
            epoch_ms(time).is_some(),
            "started_at {time} is not of the snapshot form"
        );
    }
}
