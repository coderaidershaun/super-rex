//! Whether a run is open, finished or unconfirmed at the moment of a read. It is
//! worked out at each read and never stored, so a quiet run that becomes active
//! again is shown as open again.

use std::collections::HashMap;
use std::time::Duration;

use crate::capture::runs::RunState;

/// How long a run that has not finished may stay silent and still count as open.
/// It is longer than the longest shell command an agent can run in the foreground
/// (10 minutes).
pub(super) const QUIET: Duration = Duration::from_secs(15 * 60);

/// What the state of one run is worked out from.
pub(super) struct Activity<'a> {
    pub(super) agent_id: &'a str,
    pub(super) parent_agent_id: Option<&'a str>,
    pub(super) is_finished: bool,
    /// The newest of the run's start and the modification time of its transcript.
    pub(super) last_active_ms: Option<u64>,
}

/// The state of each run at `now_ms`, in the order of `runs`, from the last
/// activity that [`last_activities`] gives for it.
pub(super) fn states(
    runs: &[Activity<'_>],
    last_active_ms: &HashMap<String, u64>,
    now_ms: u64,
) -> Vec<RunState> {
    runs.iter()
        .map(|run| {
            if run.is_finished {
                return RunState::Finished;
            }
            match last_active_ms.get(run.agent_id) {
                Some(at) if Duration::from_millis(now_ms.saturating_sub(*at)) <= QUIET => {
                    RunState::Open
                }
                _ => RunState::Unconfirmed,
            }
        })
        .collect()
}

/// The newest activity of each run and of the runs below it, by agent ID. A run is
/// active when any run below it in the tree is, because a lead waits while its
/// workers run. A run with no known activity is left out.
pub(super) fn last_activities(runs: &[Activity<'_>]) -> HashMap<String, u64> {
    runs.iter()
        .filter_map(|run| {
            let at = latest_activity(runs, run, runs.len())?;
            Some((run.agent_id.to_owned(), at))
        })
        .collect()
}

/// The newest activity of `run` and of every run below it. `levels_left` ends the
/// walk in a tree whose parent links form a loop.
fn latest_activity(runs: &[Activity<'_>], run: &Activity<'_>, levels_left: usize) -> Option<u64> {
    let mut latest = run.last_active_ms;
    if levels_left == 0 {
        return latest;
    }
    for child in runs
        .iter()
        .filter(|child| child.parent_agent_id == Some(run.agent_id))
    {
        latest = latest.max(latest_activity(runs, child, levels_left - 1));
    }
    latest
}
