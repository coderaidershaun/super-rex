//! The answer of a read, built from the store's rows after its write, with the
//! states and the unreadable files of this read.

use rusqlite::Connection;

use super::Read;
use super::files::Standing;
use crate::capture::error::CaptureError;
use crate::capture::rows::{self, StoredRun, StoredTotal};
use crate::capture::runs::{JobRuns, Run, RunState};
use crate::capture::tokens::{TokenCounts, TokenTotal};

impl Read<'_> {
    pub(super) fn result(
        &self,
        conn: &Connection,
        stored_runs: &[StoredRun],
        standing: Standing,
    ) -> Result<JobRuns, CaptureError> {
        let Standing {
            states,
            last_active_ms,
        } = standing;
        let session = rows::session_total(conn, self.job)?;
        let session_tokens = session
            .as_ref()
            .map_or(TokenTotal::Unknown, StoredTotal::total);
        let session_cost = session.as_ref().and_then(|session| session.cost);
        let runs: Vec<Run> = stored_runs
            .iter()
            .map(|run| {
                let state = states.get(&run.agent_id).copied().unwrap_or(RunState::Open);
                run_of(run, state)
            })
            .collect();
        let has_damaged_transcript = stored_runs.iter().any(|run| run.total.has_damaged_lines())
            || session.as_ref().is_some_and(StoredTotal::has_damaged_lines);
        let has_unsure_run = runs.iter().any(|run| {
            run.state == RunState::Unconfirmed
                || (run.state != RunState::Open && run.tokens == TokenTotal::Unknown)
        });
        let incomplete = self.has_unreadable_file
            || has_damaged_transcript
            || has_unsure_run
            || (!runs.is_empty() && session_tokens == TokenTotal::Unknown);
        let tokens = sum(runs.iter().map(|run| run.tokens).chain([session_tokens]));
        let cost = runs
            .iter()
            .map(|run| run.cost)
            .chain([session_cost])
            .flatten()
            .reduce(|mut sum, part| {
                sum += part;
                sum
            });
        let count = |state: RunState| {
            let count = runs.iter().filter(|run| run.state == state).count();
            u32::try_from(count).unwrap_or(u32::MAX)
        };
        Ok(JobRuns {
            open: count(RunState::Open),
            finished: count(RunState::Finished),
            unconfirmed: count(RunState::Unconfirmed),
            tokens,
            session_tokens,
            cost,
            session_cost,
            incomplete,
            last_active_ms,
            runs,
        })
    }
}

fn run_of(run: &StoredRun, state: RunState) -> Run {
    let stop_minus_start = run
        .last_stop_ms()
        .zip(run.started_ms)
        .and_then(|(stopped, started)| stopped.checked_sub(started));
    Run {
        agent_id: run.agent_id.clone(),
        parent_agent_id: run.parent_agent_id.clone(),
        depth: run.depth,
        role: run.role.clone(),
        tag_job: run.tag_job.clone(),
        agent_type: run.agent_type.clone(),
        model_ran: run.model_ran.clone(),
        effort_ran: run.effort_ran.clone(),
        background: run.background,
        state,
        started_at: run.started_at.clone(),
        stopped_at: run.last_stop_at().map(str::to_owned),
        duration_ms: run.duration_ms.or(stop_minus_start),
        tool_uses: run.tool_uses,
        lines_added: run.lines_added,
        lines_removed: run.lines_removed,
        brief: run.brief.clone(),
        final_message: run.final_message.clone(),
        // The empty text is a read transcript that opened no skill.
        skills_opened: run.skills_opened.as_deref().map(|skills| {
            skills
                .split(',')
                .filter(|skill| !skill.is_empty())
                .map(str::to_owned)
                .collect()
        }),
        tokens: run.total.total(),
        cost: run.total.cost,
    }
}

/// `Final` only when every total is final, `Unknown` only when every one is unknown.
fn sum(totals: impl Iterator<Item = TokenTotal>) -> TokenTotal {
    let mut counts = TokenCounts::default();
    let (mut is_any_known, mut is_all_final) = (false, true);
    for total in totals {
        match total {
            TokenTotal::Unknown => is_all_final = false,
            TokenTotal::Provisional(part) => {
                counts += part;
                is_any_known = true;
                is_all_final = false;
            }
            TokenTotal::Final(part) => {
                counts += part;
                is_any_known = true;
            }
        }
    }
    match (is_any_known, is_all_final) {
        (false, _) => TokenTotal::Unknown,
        (true, true) => TokenTotal::Final(counts),
        (true, false) => TokenTotal::Provisional(counts),
    }
}
