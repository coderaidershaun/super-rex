//! `rex skills`: lists the skills an agent can choose from, and checks a skill
//! file.

use std::path::Path;

use crate::error::RexError;

/// Arguments of `rex skills`.
#[derive(clap::Args)]
pub struct SkillsOptions {
    #[command(subcommand)]
    pub command: SkillsCommand,
}

/// The commands of `rex skills`.
#[derive(clap::Subcommand)]
pub enum SkillsCommand {
    /// Print each skill's name, description and "use when".
    List,

    /// Validate a skill file.
    Check,
}

/// # Errors
/// [`RexError::NotBuilt`] until the command is built.
pub fn run(_cwd: &Path, _options: SkillsOptions) -> Result<(), RexError> {
    Err(RexError::NotBuilt { command: "skills" })
}
