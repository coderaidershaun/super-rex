//! `rex hook`: called by Claude Code on each hook event. It must never fail or
//! print, because either can stop an agent.

use std::ffi::OsString;
use std::io::{self, Read};
use std::panic::{self, UnwindSafe};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::capture::append_event;
use crate::error::RexError;
use crate::store::StorePaths;

/// Called by Claude Code, not by people. Append an event to the log.
#[derive(clap::Args)]
pub struct HookOptions {
    #[command(flatten)]
    pub ignored: IgnoredArguments,
}

// Every argument of a command that Claude Code runs as a hook is accepted and
// ignored, because Claude Code treats a hook that exits with code 2 as an order
// to block the agent, and clap exits with 2 on an argument it does not know.
// This is not a doc comment, because clap would show one as the description of
// a command whose options struct has none.
#[derive(clap::Args)]
#[command(disable_help_flag = true)]
pub struct IgnoredArguments {
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, hide = true)]
    pub arguments: Vec<OsString>,
}

/// Append the event on standard input to the event log of the store found from
/// `cwd`. Nothing is written when the input is empty or no store is found, and no
/// folder is ever made. It opens no database, because a write lock that another
/// `rex` holds would make the agent wait. It appends any event without looking
/// at its name, so a new registration needs no change here.
///
/// # Errors
/// None. Every failure, a panic included, ends in silence with success, because a
/// hook that prints or fails can stop the agent that fired it.
pub fn run(cwd: &Path, _options: HookOptions) -> Result<(), RexError> {
    // Whether the event was logged is not reported: there is no one to tell.
    let _logged = silently(|| log_event(cwd));
    Ok(())
}

/// Runs `work` with panics caught and printing nothing. `None` when it panicked or
/// gave `None`.
pub(super) fn silently<T>(work: impl FnOnce() -> Option<T> + UnwindSafe) -> Option<T> {
    // The default panic hook prints to standard error, which Claude Code can hand
    // to the model, so it is replaced by one that prints nothing.
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));
    let result = panic::catch_unwind(work).ok().flatten();
    panic::set_hook(default_hook);
    result
}

/// `None` when the event was not logged: unreadable input, no store, or a log that
/// cannot be written.
fn log_event(cwd: &Path) -> Option<()> {
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input).ok()?;
    let paths = StorePaths::discover(cwd).ok()?;
    let since_epoch = SystemTime::now().duration_since(UNIX_EPOCH).ok()?;
    let at_ms = u64::try_from(since_epoch.as_millis()).ok()?;
    append_event(&paths.event_log, &input, at_ms).ok()
}
