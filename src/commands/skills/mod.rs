//! `rex skills`: lists the skills an agent can choose from, and checks skills
//! against the rules for their header and text.

mod check;
mod header;
mod list;
mod references;
mod rules;

pub use references::KnownSkills;
pub(crate) use rules::is_skill_name;
pub use rules::{Finding, Rule, Skill, SkillFile, check_skill};

use std::io;
use std::path::{Path, PathBuf};

use crate::error::{CommandError, RexError};

use header::Header;

/// The file that makes a folder a skill.
const SKILL_FILE: &str = "SKILL.md";

/// The `description` of the `SKILL.md` at `skill_file`, as one line (the text
/// `rex skills list` prints). `None` when the file cannot be read, has no
/// header, the header is not valid YAML, or `description` is missing or not a
/// string. Fits the `describe` argument of `store::orchestrator::index_all`.
pub fn describe(skill_file: &Path) -> Option<String> {
    Header::of_skill_path(skill_file)?.text("description")
}

/// Arguments of `rex skills`.
#[derive(clap::Args)]
pub struct SkillsOptions {
    #[command(subcommand)]
    pub command: SkillsCommand,
}

/// The commands of `rex skills`.
#[derive(clap::Subcommand)]
pub enum SkillsCommand {
    /// Print each skill's name, description and when to use it.
    #[command(
        after_help = "Looks in the repo's .claude/skills (source=repo), the repo's skill library in .rex/skills (source=library), and $HOME/.claude/skills (source=user). shadowed=yes marks a repo skill that a user skill of the same name replaces; a library skill is never shadowed."
    )]
    List {
        /// Print the list as one JSON line.
        #[arg(long)]
        json: bool,
    },

    /// Check skills against the rules for their header and text.
    #[command(
        after_help = "Prints `ok <path>` for each skill that passes, and `<path>: <rule>: <what is wrong>` for each finding. Exits 1 when there is at least one finding."
    )]
    Check {
        /// A `SKILL.md` file or a skill folder.
        #[arg(required = true)]
        paths: Vec<PathBuf>,
    },
}

/// Failure of `rex skills`.
#[derive(thiserror::Error, Debug)]
pub enum SkillsError {
    #[error("cannot read {}", path.display())]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("{} does not exist", path.display())]
    Missing { path: PathBuf },

    #[error("{} is a folder with no SKILL.md", path.display())]
    NoSkillFile { path: PathBuf },

    #[error("{} is not named SKILL.md; give a SKILL.md file or a skill folder", path.display())]
    NotSkillFile { path: PathBuf },

    #[error("{findings} findings in {skills_with_findings} of {skills} skills")]
    Findings {
        findings: usize,
        skills_with_findings: usize,
        skills: usize,
    },

    #[error("failed to write the skills as JSON")]
    Json {
        #[source]
        source: serde_json::Error,
    },

    #[error(transparent)]
    Rex(#[from] RexError),
}

impl SkillsError {
    fn read(path: &Path) -> impl FnOnce(io::Error) -> SkillsError {
        let path = path.to_path_buf();
        move |source| SkillsError::Read { path, source }
    }
}

impl CommandError for SkillsError {
    fn is_usage(&self) -> bool {
        matches!(self, SkillsError::NotSkillFile { .. })
    }
}

/// Run `rex skills list` or `rex skills check`.
///
/// # Errors
/// - [`SkillsError::Read`] if a skills folder or a skill file exists but
///   cannot be read.
/// - [`SkillsError::Missing`], [`SkillsError::NoSkillFile`] or
///   [`SkillsError::NotSkillFile`] if a path given to `check` is not a skill.
/// - [`SkillsError::Findings`] if `check` finds a broken rule.
/// - [`SkillsError::Json`] or [`SkillsError::Rex`] if the output cannot be
///   written.
pub fn run(cwd: &Path, options: SkillsOptions) -> Result<(), SkillsError> {
    match options.command {
        SkillsCommand::List { json } => list::run(cwd, json),
        SkillsCommand::Check { paths } => check::run(cwd, &paths),
    }
}
