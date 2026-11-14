//! Moves a store of version 1 to version 2 in one transaction, and refuses a
//! store of any other version before it writes a byte.

use std::path::Path;

use rusqlite::{Connection, OpenFlags, TransactionBehavior};

use super::{SCHEMA, SCHEMA_VERSION, Store, StoreError, StorePaths, configure, schema_version};

const MIGRATION: &str = include_str!("migrate.sql");

/// The line of the migration where the version 2 schema goes.
const SCHEMA_PLACE: &str = "-- rex:schema\n";

/// What `rex migrate` found and did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Migrated {
    /// The store was already at version 2; nothing was written.
    AlreadyCurrent,
    /// The store was at version 1 and now holds version 2, with this many jobs.
    FromVersionOne { jobs: u64 },
}

impl Store {
    /// The schema version of the store at `paths`, read without writing a byte:
    /// `None` when there is no `rex.db` yet.
    ///
    /// # Errors
    /// [`StoreError::Sqlite`] if the file cannot be read as a store.
    pub fn version_on_disk(paths: &StorePaths) -> Result<Option<i64>, StoreError> {
        if !paths.db.is_file() {
            return Ok(None);
        }
        let reader = Connection::open_with_flags(&paths.db, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(StoreError::sqlite(&paths.db))?;
        schema_version(&reader, paths).map(Some)
    }

    /// Writes a whole copy of the store at `paths` to `copy`, a file that must
    /// not exist yet, through SQLite, so pages still in the write-ahead log are
    /// in it too. The store itself is only read.
    ///
    /// # Errors
    /// [`StoreError::Sqlite`] if the copy cannot be made, for example when
    /// `copy` exists.
    pub fn back_up(paths: &StorePaths, copy: &Path) -> Result<(), StoreError> {
        let reader = Connection::open_with_flags(&paths.db, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(StoreError::sqlite(&paths.db))?;
        let target = copy.to_string_lossy();
        reader
            .execute("VACUUM INTO ?1", [target.as_ref()])
            .map_err(StoreError::sqlite(copy))?;
        Ok(())
    }

    /// Moves the store at `paths` to version 2. Safe to repeat: a store at
    /// version 2 is left as it is.
    ///
    /// # Errors
    /// - [`StoreError::NotFound`] if there is no `rex.db`.
    /// - [`StoreError::UnknownVersion`] for any version but 1 and 2, with
    ///   nothing written.
    /// - [`StoreError::BrokenReferences`] if a version 1 row names a row that
    ///   is not there; nothing is written.
    /// - [`StoreError::Sqlite`] if a step fails; the transaction is rolled
    ///   back and the store stays at version 1.
    pub fn migrate(paths: &StorePaths) -> Result<Migrated, StoreError> {
        super::check_file_exists(paths)?;
        // Read-only first, so a store of an unknown version is never written.
        let reader = Connection::open_with_flags(&paths.db, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(StoreError::sqlite(&paths.db))?;
        match schema_version(&reader, paths)? {
            SCHEMA_VERSION => return Ok(Migrated::AlreadyCurrent),
            1 => {}
            found => {
                return Err(StoreError::UnknownVersion {
                    db: paths.db.clone(),
                    found,
                });
            }
        }
        drop(reader);

        let mut conn = Connection::open(&paths.db).map_err(StoreError::sqlite(&paths.db))?;
        configure(&conn, paths)?;
        // Both settings belong to the connection and take effect only outside a
        // transaction. With them, a renamed table keeps the references of the
        // others pointing at its old name, which the new table then takes.
        conn.execute_batch("PRAGMA foreign_keys = OFF; PRAGMA legacy_alter_table = ON;")
            .map_err(StoreError::sqlite(&paths.db))?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(StoreError::sqlite(&paths.db))?;
        // Read again under the write lock, so two runs at once migrate once.
        if schema_version(&tx, paths)? == SCHEMA_VERSION {
            return Ok(Migrated::AlreadyCurrent);
        }
        tx.execute_batch(&MIGRATION.replace(SCHEMA_PLACE, SCHEMA))
            .map_err(StoreError::sqlite(&paths.db))?;
        let broken: i64 = tx
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                row.get(0)
            })
            .map_err(StoreError::sqlite(&paths.db))?;
        if broken > 0 {
            // A reference that pointed nowhere in version 1 must not be carried
            // into version 2 unseen; the rollback keeps version 1 as it was.
            return Err(StoreError::BrokenReferences {
                db: paths.db.clone(),
                count: broken,
            });
        }
        let jobs: i64 = tx
            .query_row("SELECT COUNT(*) FROM job", [], |row| row.get(0))
            .map_err(StoreError::sqlite(&paths.db))?;
        tx.commit().map_err(StoreError::sqlite(&paths.db))?;
        conn.execute_batch("PRAGMA legacy_alter_table = OFF; PRAGMA foreign_keys = ON;")
            .map_err(StoreError::sqlite(&paths.db))?;
        Ok(Migrated::FromVersionOne {
            jobs: u64::try_from(jobs).unwrap_or_default(),
        })
    }
}
