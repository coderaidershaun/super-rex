//! `rex hook`: called by Claude Code on each hook event. It must never fail or
//! print, because either can stop an agent.

use std::ffi::OsString;
use std::path::Path;

use crate::error::RexError;

/// Arguments of `rex hook`. Every argument is accepted and ignored, because
/// Claude Code treats a hook that exits with code 2 as an order to block the
/// agent, and clap exits with 2 on an argument it does not know.
#[derive(clap::Args)]
#[command(disable_help_flag = true)]
pub struct HookOptions {
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, hide = true)]
    pub arguments: Vec<OsString>,
}

/// Do nothing, and succeed.
///
/// # Errors
/// None yet.
pub fn run(_cwd: &Path, _options: HookOptions) -> Result<(), RexError> {
    Ok(())
}
