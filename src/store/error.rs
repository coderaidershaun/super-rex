//! What can go wrong when rex finds, opens or writes the store. Each text says
//! what failed and how to fix it, so the person or agent who reads it can act.

use std::fmt;
use std::path::{Path, PathBuf};

use crate::error::CommandError;

/// Failure from finding, opening or writing the store.
#[derive(thiserror::Error, Debug)]
pub enum StoreError {
    #[error(
        "no Git repository found at or above {}; run rex inside the repository where SuperRex is set up, or a worktree of it",
        start.display()
    )]
    NotInGitRepo { start: PathBuf },

    #[error(
        "cannot find the main checkout of {}: its Git folder {} is not named .git (a worktree of a bare repository, of a repository made with --separate-git-dir, or of a submodule); set REX_STORE to the .rex folder of the main checkout",
        folder.display(),
        common_dir.display()
    )]
    UnsupportedLayout {
        folder: PathBuf,
        common_dir: PathBuf,
    },

    /// `tried` holds each place in the order it was tried. When `REX_STORE` was
    /// set it is the only entry, because an explicit choice never falls through.
    #[error("{}", not_found_text(.tried))]
    NotFound { tried: Vec<Tried> },

    #[error("{} has no SuperRex schema; run rex init in {}", db.display(), root.display())]
    NotInitialised { db: PathBuf, root: PathBuf },

    #[error(
        "{} has schema version {found}, but this rex supports version {supported}; install a newer rex",
        db.display()
    )]
    NewerSchema {
        db: PathBuf,
        found: i64,
        supported: i64,
    },

    /// `path` is `None` when the error came through `?` inside a
    /// [`crate::store::Store::write`] closure, where the database path is not known.
    #[error("SQLite failed{}", on_path(.path.as_deref()))]
    Sqlite {
        path: Option<PathBuf>,
        #[source]
        source: rusqlite::Error,
    },

    #[error("cannot use {}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("{what}")]
    InvalidInput { what: String },

    #[error(
        "the system clock reads {unix_seconds} seconds since 1970, outside the years 2026 to 2094 that a job ID can hold; set the system clock to the right time"
    )]
    ClockOutOfRange { unix_seconds: i64 },
}

impl CommandError for StoreError {}

impl From<rusqlite::Error> for StoreError {
    fn from(source: rusqlite::Error) -> Self {
        StoreError::Sqlite { path: None, source }
    }
}

impl StoreError {
    pub(super) fn sqlite(path: &Path) -> impl FnOnce(rusqlite::Error) -> StoreError {
        let path = path.to_path_buf();
        move |source| StoreError::Sqlite {
            path: Some(path),
            source,
        }
    }

    pub(super) fn io(path: &Path) -> impl FnOnce(std::io::Error) -> StoreError {
        let path = path.to_path_buf();
        move |source| StoreError::Io { path, source }
    }
}

/// One place that `discover` or `open` looked for a store, and why it gave none.
#[derive(Debug)]
pub struct Tried {
    pub path: PathBuf,
    pub place: Place,
    pub reason: Reason,
}

/// Where a tried path came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    RexStore,
    ProjectDir,
    CurrentFolder,
    /// The paths a caller passed to [`crate::store::Store::open`].
    Given,
}

/// Why a tried place gave no store.
#[derive(Debug)]
pub enum Reason {
    NoDatabase,
    NotLocated(Box<StoreError>),
}

impl fmt::Display for Place {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Place::RexStore => "REX_STORE",
            Place::ProjectDir => "CLAUDE_PROJECT_DIR",
            Place::CurrentFolder => "current folder",
            Place::Given => "given",
        };
        f.write_str(name)
    }
}

impl fmt::Display for Reason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Reason::NoDatabase => f.write_str("no rex.db"),
            Reason::NotLocated(error) => match error.as_ref() {
                StoreError::NotInGitRepo { .. } => f.write_str("not inside a Git repository"),
                other => write!(f, "{other}"),
            },
        }
    }
}

fn not_found_text(tried: &[Tried]) -> String {
    if let [only] = tried
        && only.place == Place::RexStore
    {
        let dir = only.path.display();
        return format!(
            "REX_STORE is set to {dir}, but {dir}/rex.db does not exist; point REX_STORE at the .rex folder of the main checkout, or unset it"
        );
    }
    let places: Vec<String> = tried
        .iter()
        .map(|t| format!("{} ({}, {})", t.path.display(), t.place, t.reason))
        .collect();
    format!(
        "no SuperRex store found; tried {}; run rex from the repository where SuperRex is set up or a worktree of it, run rex init there, or set REX_STORE to its .rex folder",
        places.join(", ")
    )
}

fn on_path(path: Option<&Path>) -> String {
    path.map(|p| format!(" on {}", p.display()))
        .unwrap_or_default()
}
