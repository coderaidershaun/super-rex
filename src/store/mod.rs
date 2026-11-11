//! The record of jobs, roles, sub-agent runs and feedback, kept in one SQLite
//! file for each repo.

pub mod class;
mod error;
mod job_id;
pub mod orchestrator;
mod paths;
pub mod status;

pub use error::{Accepts, Place, Reason, StoreError, Tried};
pub use job_id::{JobId, JobIdError};
pub use paths::StorePaths;

use std::fs;
use std::time::Duration;

use rusqlite::types::Type;
use rusqlite::{Connection, OpenFlags, OptionalExtension, Transaction, TransactionBehavior};

const SCHEMA: &str = include_str!("schema.sql");
const SCHEMA_VERSION: i64 = 1;

/// How long a writer waits for another writer before it fails with
/// `database is locked`.
const BUSY_TIMEOUT: Duration = Duration::from_secs(10);

/// An open connection to one store, with the settings its rules need. Write
/// to `rex.db` only through this type: it turns on `foreign_keys` and
/// `recursive_triggers` for each connection, and without them a write can
/// break the rules of the record with no error.
///
/// Each kind of row has one writer. The job commands write jobs, roles and
/// deliveries, and a job's status only through [`status::apply`]. Capture
/// writes the sub-agent runs, the session tokens, the `intake.` keys of
/// `meta`, and a job's builder model and effort while they are unset.
/// `rex feedback` writes feedback, `rex recall` the recalled experience, and
/// [`orchestrator::index`] the orchestrator index. Every update of a job row
/// sets `updated_at` in the same statement.
#[derive(Debug)]
pub struct Store {
    conn: Connection,
    paths: StorePaths,
}

impl Store {
    /// Makes the store for `rex init`, or opens it if it is already made. A
    /// store already at version 1 is left exactly as it was, so this is safe to
    /// run twice.
    ///
    /// # Errors
    /// - [`StoreError::NewerSchema`] if the file has a newer schema.
    /// - [`StoreError::Io`] if a folder cannot be made.
    /// - [`StoreError::Sqlite`] if SQLite fails.
    pub fn create(paths: &StorePaths) -> Result<Store, StoreError> {
        fs::create_dir_all(&paths.orchestrators).map_err(StoreError::io(&paths.orchestrators))?;
        let mut conn = Connection::open(&paths.db).map_err(StoreError::sqlite(&paths.db))?;
        configure(&conn, paths)?;

        let mode: String = conn
            .pragma_query_value(None, "journal_mode", |row| row.get(0))
            .map_err(StoreError::sqlite(&paths.db))?;
        if !mode.eq_ignore_ascii_case("wal") {
            // If WAL is not possible here, SQLite keeps the old mode; writers
            // still wait for each other, only readers wait for writers too.
            conn.pragma_update(None, "journal_mode", "WAL")
                .map_err(StoreError::sqlite(&paths.db))?;
        }

        // The version is read under the write lock, so two `rex init` runs at
        // once cannot both see version 0 and both apply the schema.
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(StoreError::sqlite(&paths.db))?;
        match schema_version(&tx, paths)? {
            0 => tx
                .execute_batch(SCHEMA)
                .map_err(StoreError::sqlite(&paths.db))?,
            SCHEMA_VERSION => {}
            found => return Err(newer_schema(paths, found)),
        }
        tx.commit().map_err(StoreError::sqlite(&paths.db))?;
        Ok(Store {
            conn,
            paths: paths.clone(),
        })
    }

    /// Opens an existing store. Never makes a file.
    ///
    /// # Errors
    /// - [`StoreError::NotFound`] if `rex.db` does not exist.
    /// - [`StoreError::NotInitialised`] if it has no schema.
    /// - [`StoreError::NewerSchema`] if it has a newer schema.
    /// - [`StoreError::Sqlite`] if SQLite fails.
    pub fn open(paths: &StorePaths) -> Result<Store, StoreError> {
        if !paths.db.is_file() {
            let tried = vec![Tried {
                path: paths.db.clone(),
                place: Place::Given,
                reason: Reason::NoDatabase,
            }];
            return Err(StoreError::NotFound { tried });
        }
        let flags = OpenFlags::default().difference(OpenFlags::SQLITE_OPEN_CREATE);
        let conn =
            Connection::open_with_flags(&paths.db, flags).map_err(StoreError::sqlite(&paths.db))?;
        configure(&conn, paths)?;

        match schema_version(&conn, paths)? {
            SCHEMA_VERSION => Ok(Store {
                conn,
                paths: paths.clone(),
            }),
            0 => Err(StoreError::NotInitialised {
                db: paths.db.clone(),
                root: paths.root.clone(),
            }),
            found => Err(newer_schema(paths, found)),
        }
    }

    /// The paths this store was opened with.
    pub fn paths(&self) -> &StorePaths {
        &self.paths
    }

    /// The connection, for reads only. Write through [`Store::write`].
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Runs `f` in one transaction that holds the write lock from its start,
    /// so a second writer waits for this one instead of failing. Commits when
    /// `f` returns `Ok` and rolls back when it returns `Err`.
    ///
    /// # Errors
    /// The error of `f`, or [`StoreError::Sqlite`] if the transaction cannot
    /// start or commit, for example when the lock is held for more than 10 seconds.
    pub fn write<T, E: From<StoreError>>(
        &mut self,
        f: impl FnOnce(&Transaction<'_>) -> Result<T, E>,
    ) -> Result<T, E> {
        let db = &self.paths.db;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(StoreError::sqlite(db))?;
        // Dropping the transaction when `f` fails rolls it back.
        let value = f(&tx)?;
        tx.commit().map_err(StoreError::sqlite(db))?;
        Ok(value)
    }

    /// The open job of `session_id` (status `created` or `planned`), if any.
    ///
    /// # Errors
    /// [`StoreError::Sqlite`] if the read fails, or if the stored ID is not a
    /// job ID.
    pub fn open_job_for_session(&self, session_id: &str) -> Result<Option<JobId>, StoreError> {
        let id: Option<String> = self
            .conn
            .query_row(
                "SELECT id FROM job WHERE session_id = ?1 AND status IN ('created', 'planned')",
                [session_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(StoreError::sqlite(&self.paths.db))?;
        id.map(|id| id.parse().map_err(stored_id_error(&self.paths)))
            .transpose()
    }
}

/// Settings that belong to each connection, not to the file, so every open
/// must set them again.
fn configure(conn: &Connection, paths: &StorePaths) -> Result<(), StoreError> {
    conn.busy_timeout(BUSY_TIMEOUT)
        .map_err(StoreError::sqlite(&paths.db))?;
    conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA recursive_triggers = ON;")
        .map_err(StoreError::sqlite(&paths.db))
}

/// A job ID in the store that is not of the job ID form, reported as SQLite's
/// own failure to convert a stored value.
fn stored_id_error(paths: &StorePaths) -> impl FnOnce(JobIdError) -> StoreError {
    let db = paths.db.clone();
    move |error| {
        let source = rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error));
        StoreError::sqlite(&db)(source)
    }
}

fn schema_version(conn: &Connection, paths: &StorePaths) -> Result<i64, StoreError> {
    conn.pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(StoreError::sqlite(&paths.db))
}

fn newer_schema(paths: &StorePaths, found: i64) -> StoreError {
    StoreError::NewerSchema {
        db: paths.db.clone(),
        found,
        supported: SCHEMA_VERSION,
    }
}
