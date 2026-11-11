//! `rex gate`: called by Claude Code before each start of a sub-agent, it refuses
//! the start once the job has spent its limit, and lets the start go on otherwise.

mod limit;
mod raise;

pub use limit::{Dollars, DollarsError, LimitSource, SpendLimit};
pub use raise::{LimitError, LimitOptions, run as run_limit};

use std::io::{self, Read};
use std::path::Path;
use std::time::{Duration, SystemTime};

use serde_json::{Value, json};

use super::hook::silently;
use super::stdout::print_line_to_stdout;
use crate::capture::job_runs;
use crate::error::RexError;
use crate::store::{Store, StorePaths};

/// The gate runs before every start of a sub-agent, so it waits only this long for
/// another writer, not the store's usual 10 seconds.
const LOCK_WAIT: Duration = Duration::from_secs(1);

/// Reads one PreToolUse event on standard input. When the session's open job has
/// spent its limit, prints the refusal that Claude Code reads. Otherwise prints
/// nothing, so the normal permission flow decides; it never prints `allow`.
///
/// # Errors
/// None. Every failure, a panic included, ends in silence with success, so the
/// agent starts its sub-agent: a gate that cannot tell must not stop work.
pub fn run(cwd: &Path, _options: super::hook::HookOptions) -> Result<(), RexError> {
    // Whether the refusal was printed is not reported: there is no one to tell.
    let _refused = silently(|| refuse_if_over(cwd));
    Ok(())
}

/// `None` when the call is not refused or the gate cannot tell.
fn refuse_if_over(cwd: &Path) -> Option<()> {
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input).ok()?;
    let event: Value = serde_json::from_slice(&input).ok()?;
    if event.get("hook_event_name")?.as_str()? != "PreToolUse" {
        return None;
    }
    // A matcher widened by hand must not let the gate refuse other tools.
    if !matches!(event.get("tool_name")?.as_str()?, "Agent" | "Task") {
        return None;
    }
    let session = event.get("session_id")?.as_str()?;
    let paths = StorePaths::discover(cwd).ok()?;
    let mut store = Store::open(&paths).ok()?;
    store.conn().busy_timeout(LOCK_WAIT).ok()?;
    let job = store.open_job_for_session(session).ok()??;
    let runs = job_runs(&mut store, &job, SystemTime::now()).ok()?;
    let limit = SpendLimit::of(&store, &job).ok()?;
    let spent = runs.cost.unwrap_or_default();
    if !limit.is_reached(&spent) {
        return None;
    }
    let refusal = json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "deny",
            "permissionDecisionReason": limit.refusal(&job, &spent),
        }
    });
    print_line_to_stdout(&refusal.to_string()).ok()
}
