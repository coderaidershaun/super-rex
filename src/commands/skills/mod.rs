//! `rex skills`: lists the skills an agent can choose from, checks skills against the rules for
//! their header and text, and serves and searches a skill library.

mod check;
mod header;
mod intents;
mod list;
mod metadata;
mod references;
mod rules;
pub mod search;
mod serve;

pub use references::KnownSkills;
pub use rules::{Finding, Rule, Skill, SkillFile, check_skill};

use std::io;
use std::path::{Path, PathBuf};

use crate::error::{CommandError, FailureClass, RexError};

use header::Header;

/// The file that makes a folder a skill.
const SKILL_FILE: &str = "SKILL.md";

/// The folders of a repo, and of the user's home for the first, that `rex skills list` reads.
pub(crate) const REPO_SKILLS_FOLDER: &str = ".claude/skills";
pub(crate) const REPO_LIBRARY_FOLDER: &str = ".rex/skills";

/// The `description` of the `SKILL.md` at `skill_file`, as one line (the text
/// `rex skills list` prints). `None` when the file cannot be read, has no
/// header, the header is not valid YAML, or `description` is missing or not a
/// string. Fits the `describe` argument of `store::orchestrator::index_all`.
pub fn describe(skill_file: &Path) -> Option<String> {
    Header::of_skill_path(skill_file)?.text("description")
}

/// True when the `SKILL.md` at `skill_file` has a `metadata` map that holds a
/// key a skill library search reads.
pub fn has_metadata(skill_file: &Path) -> bool {
    Header::of_skill_path(skill_file).is_some_and(|header| header.metadata().is_some())
}

/// List the skills, or check a skill file.
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
        after_help = "Prints `ok <path>` for each skill that passes, and `<path>: <rule>: <what is wrong>` for each finding. Exits 1 when there is at least one finding. A skill file that the text names as `<skill>/<file>.md` must be in plugin/skills or plugin/library of the plugin, in the repo's .rex/skills, or beside a skill that is checked."
    )]
    Check {
        /// A `SKILL.md` file, a skill folder, or a folder of skill folders.
        #[arg(required = true)]
        paths: Vec<PathBuf>,

        /// A folder of other skills to search among when each intent phrase
        /// must find its own skill. Give it more than once for several.
        #[arg(long = "pool")]
        pools: Vec<PathBuf>,
    },

    /// Serve a skill library to an MCP client over stdin and stdout.
    #[command(
        after_help = "Answers the tools head, search and get, one JSON-RPC message per line. Give --library more than once for several folders, first one wins when a skill name repeats. With no --library, serves the library embedded in this binary."
    )]
    Serve {
        /// The name `head` reports; the first folder's base name, or super-rex, when left out.
        #[arg(long)]
        name: Option<String>,
        /// A folder of skill folders.
        #[arg(long = "library")]
        libraries: Vec<PathBuf>,
    },

    /// Search a skill library with request forms, as the search tool of `rex skills serve` does.
    #[command(
        after_help = "Reads the arguments of the search tool as one JSON object on stdin: requests, limit, cursor, kinds and categories. Prints the answer as one JSON line. Give --library more than once for several folders, first one wins when a skill name repeats. With no --library, searches the library embedded in this binary."
    )]
    Search {
        /// A folder of skill folders.
        #[arg(long = "library")]
        libraries: Vec<PathBuf>,
    },

    /// Print the static index of a skill library as JSON, to publish beside it.
    #[command(
        after_help = "Prints one JSON document: each served skill with its name, type, description, url and sha256 digest, and the fields a search reads. Give --library more than once for several folders, first one wins when a skill name repeats. With no --library, indexes the library embedded in this binary."
    )]
    Index {
        /// A folder of skill folders.
        #[arg(long = "library")]
        libraries: Vec<PathBuf>,
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

    #[error("{} is a folder with no SKILL.md and no skill folder in it", path.display())]
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

    #[error(transparent)]
    Serve(#[from] serve::ServeError),

    #[error(transparent)]
    Search(#[from] search::SearchError),
}

impl SkillsError {
    fn read(path: &Path) -> impl FnOnce(io::Error) -> SkillsError {
        let path = path.to_path_buf();
        move |source| SkillsError::Read { path, source }
    }
}

impl CommandError for SkillsError {
    fn failure_class(&self) -> FailureClass {
        match self {
            SkillsError::Read { .. } => FailureClass::Retry,
            SkillsError::Missing { .. }
            | SkillsError::NoSkillFile { .. }
            | SkillsError::NotSkillFile { .. }
            | SkillsError::Findings { .. }
            | SkillsError::Json { .. } => FailureClass::Reject,
            SkillsError::Rex(error) => error.failure_class(),
            SkillsError::Serve(_) => FailureClass::Reject,
            SkillsError::Search(error) => error.failure_class(),
        }
    }

    fn is_usage(&self) -> bool {
        matches!(self, SkillsError::NotSkillFile { .. })
    }
}

/// Run `rex skills list`, `check`, `serve`, `search` or `index`.
///
/// # Errors
/// - [`SkillsError::Read`] if a skills folder or a skill file exists but
///   cannot be read.
/// - [`SkillsError::Missing`], [`SkillsError::NoSkillFile`] or
///   [`SkillsError::NotSkillFile`] if a path given to `check` is not a skill.
/// - [`SkillsError::Findings`] if `check` finds a broken rule.
/// - [`SkillsError::Json`] or [`SkillsError::Rex`] if the output cannot be
///   written.
/// - [`SkillsError::Serve`] or [`SkillsError::Search`] if a library cannot
///   be served, searched or indexed.
pub fn run(cwd: &Path, options: SkillsOptions) -> Result<(), SkillsError> {
    match options.command {
        SkillsCommand::List { json } => list::run(cwd, json),
        SkillsCommand::Check { paths, pools } => check::run(cwd, &paths, &pools),
        SkillsCommand::Serve { name, libraries } => Ok(serve::run(name, &libraries)?),
        SkillsCommand::Search { libraries } => search::run_search(&libraries),
        SkillsCommand::Index { libraries } => search::run_index(&libraries),
    }
}
