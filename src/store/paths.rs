//! Finds the one store of a repository from any folder of it, including a
//! sub-folder or a Git worktree. It only reads files and never runs `git`, so
//! the hook that calls it on every event stays fast.

use std::ffi::{OsStr, OsString};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::error::{Place, Reason, StoreError, Tried};

/// The absolute paths of one store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorePaths {
    /// The main checkout root, or the parent of `REX_STORE`.
    pub root: PathBuf,
    /// `<root>/.rex`, or `REX_STORE` itself.
    pub dir: PathBuf,
    /// `<dir>/rex.db`
    pub db: PathBuf,
    /// `<dir>/events.jsonl`
    pub event_log: PathBuf,
    /// `<dir>/orchestrators`
    pub orchestrators: PathBuf,
}

impl StorePaths {
    /// The store paths of the main checkout that holds `start`. A worktree gives
    /// the paths of its main checkout. Does not check that `.rex` exists.
    ///
    /// # Errors
    /// - [`StoreError::NotInGitRepo`] if no folder at or above `start` holds `.git`.
    /// - [`StoreError::UnsupportedLayout`] for a worktree whose main Git folder
    ///   is not named `.git`, such as a worktree of a bare repository.
    /// - [`StoreError::Io`] if `start` or a Git file cannot be read.
    pub fn locate(start: &Path) -> Result<StorePaths, StoreError> {
        let start = start.canonicalize().map_err(StoreError::io(start))?;
        for folder in start.ancestors() {
            let dot_git = folder.join(".git");
            let metadata = match fs::metadata(&dot_git) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(source) => {
                    return Err(StoreError::Io {
                        path: dot_git,
                        source,
                    });
                }
            };
            let root = if metadata.is_dir() {
                folder.to_path_buf()
            } else {
                root_of_git_file(folder, &dot_git)?
            };
            return Ok(StorePaths::at_root(root));
        }
        Err(StoreError::NotInGitRepo { start })
    }

    /// The store of this session, seen from `cwd`. Reads `REX_STORE` and
    /// `CLAUDE_PROJECT_DIR` from the environment; see [`StorePaths::discover_from`].
    ///
    /// # Errors
    /// [`StoreError::NotFound`] if no place gives an existing `rex.db`, or
    /// [`StoreError::Io`] if the `REX_STORE` folder cannot be resolved.
    pub fn discover(cwd: &Path) -> Result<StorePaths, StoreError> {
        let rex_store = std::env::var_os("REX_STORE");
        let project_dir = std::env::var_os("CLAUDE_PROJECT_DIR");
        StorePaths::discover_from(rex_store, project_dir, cwd)
    }

    /// The first store whose `rex.db` exists: `rex_store` (a `.rex` folder),
    /// else the main checkout of `project_dir`, else that of `cwd`. A set
    /// `rex_store` never falls through to the others. An empty value counts as
    /// unset, and a relative one is relative to `cwd`.
    ///
    /// # Errors
    /// - [`StoreError::NotFound`], listing each place tried and why it gave no store.
    /// - [`StoreError::Io`] if the `rex_store` folder cannot be resolved.
    pub fn discover_from(
        rex_store: Option<OsString>,
        project_dir: Option<OsString>,
        cwd: &Path,
    ) -> Result<StorePaths, StoreError> {
        if let Some(dir) = non_empty(rex_store) {
            let dir = cwd.join(dir);
            if dir.join("rex.db").is_file() {
                let dir = dir.canonicalize().map_err(StoreError::io(&dir))?;
                return Ok(StorePaths::in_dir(dir));
            }
            let tried = vec![Tried {
                path: dir,
                place: Place::RexStore,
                reason: Reason::NoDatabase,
            }];
            return Err(StoreError::NotFound { tried });
        }

        let starts = [
            (
                non_empty(project_dir).map(|dir| cwd.join(dir)),
                Place::ProjectDir,
            ),
            (Some(cwd.to_path_buf()), Place::CurrentFolder),
        ];
        let mut tried = Vec::new();
        for (start, place) in starts {
            let Some(start) = start else { continue };
            match StorePaths::locate(&start) {
                Ok(paths) if paths.db.is_file() => return Ok(paths),
                Ok(paths) => tried.push(Tried {
                    path: paths.db,
                    place,
                    reason: Reason::NoDatabase,
                }),
                Err(error) => tried.push(Tried {
                    path: start,
                    place,
                    reason: Reason::NotLocated(Box::new(error)),
                }),
            }
        }
        Err(StoreError::NotFound { tried })
    }

    fn at_root(root: PathBuf) -> StorePaths {
        StorePaths::with(root.join(".rex"), root)
    }

    fn in_dir(dir: PathBuf) -> StorePaths {
        let root = dir.parent().map_or_else(|| dir.clone(), Path::to_path_buf);
        StorePaths::with(dir, root)
    }

    fn with(dir: PathBuf, root: PathBuf) -> StorePaths {
        StorePaths {
            db: dir.join("rex.db"),
            event_log: dir.join("events.jsonl"),
            orchestrators: dir.join("orchestrators"),
            root,
            dir,
        }
    }
}

/// The main checkout root for a folder whose `.git` is a file. A linked
/// worktree's Git folder holds a `commondir` file that points at the main Git
/// folder; a submodule's or a `--separate-git-dir` checkout's does not.
fn root_of_git_file(folder: &Path, dot_git: &Path) -> Result<PathBuf, StoreError> {
    let text = fs::read_to_string(dot_git).map_err(StoreError::io(dot_git))?;
    let Some(git_dir) = text.trim().strip_prefix("gitdir:") else {
        let source = io::Error::new(
            io::ErrorKind::InvalidData,
            "a .git file must start with `gitdir: `",
        );
        return Err(StoreError::Io {
            path: dot_git.to_path_buf(),
            source,
        });
    };
    let git_dir = folder.join(git_dir.trim());

    let commondir_file = git_dir.join("commondir");
    let common = match fs::read_to_string(&commondir_file) {
        Ok(common) => common,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(folder.to_path_buf()),
        Err(source) => {
            return Err(StoreError::Io {
                path: commondir_file,
                source,
            });
        }
    };
    let common_dir = git_dir.join(common.trim());
    let common_dir = common_dir
        .canonicalize()
        .map_err(StoreError::io(&common_dir))?;

    if common_dir.file_name() == Some(OsStr::new(".git"))
        && let Some(root) = common_dir.parent()
    {
        return Ok(root.to_path_buf());
    }
    Err(StoreError::UnsupportedLayout {
        folder: folder.to_path_buf(),
        common_dir,
    })
}

fn non_empty(value: Option<OsString>) -> Option<OsString> {
    value.filter(|value| !value.is_empty())
}
