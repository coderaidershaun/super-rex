//! Where a job's worktrees live, `<parent>/<name>.worktrees/` beside the repo,
//! and which folders there are real linked worktrees of the repo.

use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::commands::links::is_symlink;

/// The folder of the job worktrees of `repo`, a canonical main checkout:
/// `<parent>/<name>.worktrees`. `None` for a repo at the root of the disk,
/// or when that folder is a symbolic link, which could lead anywhere.
pub(crate) fn worktrees_root(repo: &Path) -> Option<PathBuf> {
    let mut name = repo.file_name()?.to_os_string();
    name.push(".worktrees");
    let root = repo.parent()?.join(name);
    (!is_symlink(&root)).then_some(root)
}

/// True when `path` is absolute and has no `.` or `..` in it, so its text
/// says where it is.
pub(super) fn is_plain_absolute(path: &Path) -> bool {
    path.is_absolute()
        && path
            .components()
            .all(|part| !matches!(part, Component::CurDir | Component::ParentDir))
}

/// `path`, which need not exist, with each link in the part that exists
/// resolved. `None` when no part of it can be resolved.
pub(super) fn resolved(path: &Path) -> Option<PathBuf> {
    let mut missing = Vec::new();
    let mut existing = path;
    loop {
        if fs::symlink_metadata(existing).is_ok() {
            let mut found = existing.canonicalize().ok()?;
            found.extend(missing.iter().rev());
            return Some(found);
        }
        missing.push(existing.file_name()?);
        existing = existing.parent()?;
    }
}

/// True when `dir` lies below `root` and is not `root` itself.
pub(super) fn is_below(dir: &Path, root: &Path) -> bool {
    dir != root && dir.starts_with(root)
}

/// The folder that the `gitdir:` line of `worktree/.git` names, or `None`
/// when `.git` is not a plain file with that line.
fn admin_dir_of(worktree: &Path) -> Option<PathBuf> {
    let dot_git = worktree.join(".git");
    if !fs::symlink_metadata(&dot_git).ok()?.is_file() {
        return None;
    }
    let text = fs::read_to_string(&dot_git).ok()?;
    let named = text.lines().next()?.strip_prefix("gitdir:")?.trim();
    Some(worktree.join(named))
}

/// True when `dir`, a canonical folder, is a linked worktree of `repo`, a
/// canonical main checkout: its `.git` file names a folder in the repo's
/// `.git/worktrees/`, and that folder's `gitdir` file names `dir/.git` back.
/// The same test Claude Code makes before it lets git write there.
pub(crate) fn is_linked_worktree_of(dir: &Path, repo: &Path) -> bool {
    let Some(admin) = admin_dir_of(dir).and_then(|admin| admin.canonicalize().ok()) else {
        return false;
    };
    let Ok(registry) = repo.join(".git").join("worktrees").canonicalize() else {
        return false;
    };
    if admin.parent() != Some(registry.as_path()) {
        return false;
    }
    let Ok(back) = fs::read_to_string(admin.join("gitdir")) else {
        return false;
    };
    let back = admin.join(back.trim()).canonicalize().ok();
    back.is_some() && back == dir.join(".git").canonicalize().ok()
}

/// Every linked worktree of `repo`, a canonical main checkout, that lies in
/// its worktrees folder: canonical, sorted, read from git's own files so no
/// `git` needs to run.
pub(crate) fn rex_worktrees(repo: &Path) -> Vec<PathBuf> {
    let Some(root) = worktrees_root(repo) else {
        return Vec::new();
    };
    let Ok(entries) = fs::read_dir(repo.join(".git").join("worktrees")) else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = entries
        .flatten()
        .filter_map(|entry| {
            let admin = entry.path();
            let named = fs::read_to_string(admin.join("gitdir")).ok()?;
            let dir = admin.join(named.trim()).parent()?.canonicalize().ok()?;
            (is_below(&dir, &root) && is_linked_worktree_of(&dir, repo)).then_some(dir)
        })
        .collect();
    found.sort();
    found.dedup();
    found
}
