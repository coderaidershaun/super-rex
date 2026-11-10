//! `rex commands`: lists every command with its description, so that an agent
//! learns the whole command line in one call.

use clap::Command;

use crate::error::RexError;

/// Arguments of `rex commands`.
#[derive(clap::Args)]
pub struct CommandsOptions {}

/// # Errors
/// [`RexError::NotBuilt`] until the command is built.
pub fn run(_cli: &Command, _options: CommandsOptions) -> Result<(), RexError> {
    Err(RexError::NotBuilt {
        command: "commands",
    })
}
