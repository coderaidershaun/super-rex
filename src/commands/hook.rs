//! `rex hook`: called by Claude Code on each hook event. It must never fail or
//! print, because either can stop an agent.

use std::ffi::OsString;
use std::io::{self, Read};
use std::panic;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::capture::append_event;
use crate::error::RexError;
use crate::store::StorePaths;

/// Arguments of `rex hook`. Every argument is accepted and ignored, because
/// Claude Code treats a hook that exits with code 2 as an order to block the
/// agent, and clap exits with 2 on an argument it does not know.
#[derive(clap::Args)]
#[command(disable_help_flag = true)]
pub struct HookOptions {
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, hide = true)]
    pub arguments: Vec<OsString>,
}

/// Append the event on standard input to the event log of the store found from
/// `cwd`. Nothing is written when the input is empty or no store is found, and no
/// folder is ever made.
///
/// # Errors
/// None. Every failure, a panic included, ends in silence with success, because a
/// hook that prints or fails can stop the agent that fired it.
pub fn run(cwd: &Path, _options: HookOptions) -> Result<(), RexError> {
    // The default panic hook prints to standard error, which Claude Code can hand
    // to the model, so it is replaced by one that prints nothing.
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));
    // Whether the event was logged is not reported: there is no one to tell.
    let _logged = panic::catch_unwind(|| log_event(cwd));
    panic::set_hook(default_hook);
    Ok(())
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
