//! `rex migrate`: moves the repo's store to the version this rex reads, once, and
//! refuses a store of a version it does not know.

use std::path::Path;

use super::stdout::print_line_to_stdout;
use crate::error::{CommandError, FailureClass, RexError};
use crate::store::{Migrated, Store, StoreError, StorePaths};

/// Move the store to the current version. Safe to run twice.
#[derive(clap::Args)]
#[command(
    after_help = "Prints `store=v2 migrated=yes jobs=<n>` after a migration, or `store=v2 migrated=no` when there was nothing to do. A store of any other version is refused with exit 2, and nothing is written."
)]
pub struct MigrateOptions {}

#[derive(thiserror::Error, Debug)]
pub enum MigrateError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Rex(#[from] RexError),
}

impl CommandError for MigrateError {
    fn failure_class(&self) -> FailureClass {
        match self {
            MigrateError::Store(error) => error.failure_class(),
            MigrateError::Rex(error) => error.failure_class(),
        }
    }

    fn exit_code(&self) -> u8 {
        match self {
            MigrateError::Store(error) => error.exit_code(),
            MigrateError::Rex(error) => error.exit_code(),
        }
    }

    fn is_bare_line(&self) -> bool {
        match self {
            MigrateError::Store(error) => error.is_bare_line(),
            MigrateError::Rex(error) => error.is_bare_line(),
        }
    }
}

/// # Errors
/// - [`StoreError::NotFound`] if the repo has no store.
/// - [`StoreError::UnknownVersion`] or [`StoreError::BrokenReferences`] for a
///   store this rex will not migrate, with nothing written.
/// - [`StoreError::Sqlite`] if the migration fails; it is rolled back.
pub fn run(cwd: &Path, _options: MigrateOptions) -> Result<(), MigrateError> {
    let paths = StorePaths::discover(cwd)?;
    let line = match Store::migrate(&paths)? {
        Migrated::AlreadyCurrent => "store=v2 migrated=no".to_owned(),
        Migrated::FromVersionOne { jobs } => format!("store=v2 migrated=yes jobs={jobs}"),
    };
    print_line_to_stdout(&line)?;
    Ok(())
}
