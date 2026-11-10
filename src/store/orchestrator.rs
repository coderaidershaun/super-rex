//! The orchestrator index: rex gives each orchestrator folder a version number
//! that rises when its files change, so no agent has to keep a version by hand.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use rusqlite::{OptionalExtension, Transaction, params};

use super::error::StoreError;

/// One version of one orchestrator in the index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrchestratorVersion {
    pub name: String,
    pub version: i64,
    pub description: String,
}

/// Indexes the folder `<library>/<name>/` and returns its current version: the
/// newest one when the content is unchanged, else a new one. `describe` reads
/// the description from `<name>/SKILL.md`; the store parses no YAML itself.
///
/// # Errors
/// - [`StoreError::Io`] if the folder or one of its files cannot be read.
/// - [`StoreError::Sqlite`] if a read or write of the index fails.
pub fn index(
    tx: &Transaction<'_>,
    library: &Path,
    name: &str,
    describe: &dyn Fn(&Path) -> Option<String>,
) -> Result<OrchestratorVersion, StoreError> {
    let folder = library.join(name);
    let hash = content_hash(&folder)?;

    let newest: Option<(i64, String, String)> = tx
        .query_row(
            "SELECT version, content_hash, description FROM orchestrator
             WHERE name = ?1 ORDER BY version DESC LIMIT 1",
            [name],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let version = match newest {
        Some((version, newest_hash, description)) if newest_hash == hash => {
            return Ok(OrchestratorVersion {
                name: name.to_owned(),
                version,
                description,
            });
        }
        Some((version, _, _)) => version + 1,
        None => 1,
    };

    let description = describe(&folder.join("SKILL.md")).unwrap_or_default();
    tx.execute(
        "INSERT INTO orchestrator (name, version, content_hash, description, indexed_at)
         VALUES (?1, ?2, ?3, ?4, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
        params![name, version, hash, description],
    )?;
    Ok(OrchestratorVersion {
        name: name.to_owned(),
        version,
        description,
    })
}

/// Runs [`index`] for every orchestrator folder in `library`, sorted by name.
///
/// # Errors
/// As [`index`]; also [`StoreError::Io`] if `library` cannot be listed.
pub fn index_all(
    tx: &Transaction<'_>,
    library: &Path,
    describe: &dyn Fn(&Path) -> Option<String>,
) -> Result<Vec<OrchestratorVersion>, StoreError> {
    let mut names = Vec::new();
    for entry in fs::read_dir(library).map_err(StoreError::io(library))? {
        let entry = entry.map_err(StoreError::io(library))?;
        let file_type = entry.file_type().map_err(StoreError::io(&entry.path()))?;
        if !file_type.is_dir() || is_hidden(&entry.file_name()) {
            continue;
        }
        let name = entry.file_name().into_string().map_err(|_| {
            let source = io::Error::new(io::ErrorKind::InvalidData, "folder name is not UTF-8");
            StoreError::io(&entry.path())(source)
        })?;
        names.push(name);
    }
    names.sort();
    names
        .iter()
        .map(|name| index(tx, library, name, describe))
        .collect()
}

/// FNV-1a, 64 bits: short, the same in every Rust release (unlike the
/// standard library's hasher), and enough to see a change. It is not for security.
struct Fnv1a(u64);

impl Fnv1a {
    fn new() -> Fnv1a {
        Fnv1a(0xcbf2_9ce4_8422_2325)
    }

    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 ^= u64::from(byte);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
}

/// The hash of every file under `folder`, in the order of their paths, as 16
/// lower-case hex characters.
fn content_hash(folder: &Path) -> Result<String, StoreError> {
    let mut files = Vec::new();
    collect_files(folder, &[], &mut files)?;
    files.sort();

    let mut hash = Fnv1a::new();
    for (relative, path) in &files {
        let content = fs::read(path).map_err(StoreError::io(path))?;
        hash.write(relative);
        hash.write(&[0]);
        hash.write(&(content.len() as u64).to_le_bytes());
        hash.write(&content);
    }
    Ok(format!("{:016x}", hash.0))
}

/// Adds each regular file under `dir`: its relative path, with `/` between
/// parts, and its full path. Symbolic links and hidden files and folders are skipped.
fn collect_files(
    dir: &Path,
    prefix: &[u8],
    files: &mut Vec<(Vec<u8>, PathBuf)>,
) -> Result<(), StoreError> {
    for entry in fs::read_dir(dir).map_err(StoreError::io(dir))? {
        let entry = entry.map_err(StoreError::io(dir))?;
        let file_name = entry.file_name();
        if is_hidden(&file_name) {
            continue;
        }
        let mut relative = prefix.to_vec();
        if !relative.is_empty() {
            relative.push(b'/');
        }
        relative.extend_from_slice(file_name.as_encoded_bytes());

        let file_type = entry.file_type().map_err(StoreError::io(&entry.path()))?;
        if file_type.is_dir() {
            collect_files(&entry.path(), &relative, files)?;
        } else if file_type.is_file() {
            files.push((relative, entry.path()));
        }
    }
    Ok(())
}

fn is_hidden(file_name: &std::ffi::OsStr) -> bool {
    file_name.as_encoded_bytes().starts_with(b".")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv1a_matches_the_published_test_vectors() {
        let hash_of = |bytes: &[u8]| {
            let mut hash = Fnv1a::new();
            hash.write(bytes);
            hash.0
        };
        assert_eq!(hash_of(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(hash_of(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(hash_of(b"foobar"), 0x8594_4171_f739_67e8);
    }
}
