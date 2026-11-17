//! `rex worktree add` and `rex worktree remove`: make or remove a job's Git
//! worktree beside the repo, and list it so that Claude Code's file tools may work in it.
//!
//! A folder listed in `permissions.additionalDirectories` is a working
//! directory of every session in the repo, so its files can be read and
//! edited there. An agent runs these commands with no prompt, so each one
//! checks its arguments itself: a worktree goes only in
//! `<parent>/<name>.worktrees/`, and only a real linked worktree is listed.

mod error;
mod place;
mod register;

pub use error::WorktreeError;
pub(crate) use place::{rex_worktrees, worktrees_root};
pub(crate) use register::{SETTINGS_FILE, SettingsLock};

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::stdout::print_line_to_stdout;
use place::{is_below, is_linked_worktree_of, is_plain_absolute, resolved};
use register::Registry;

/// Make or remove a job's worktree in <repo>.worktrees/, beside the repo, and list or unlist it in .claude/settings.local.json, so Claude Code's file tools may work in it.
#[derive(clap::Args, Clone, Debug)]
#[command(
    after_help = "Prints worktree=<path> and settings=added|removed|unchanged. A worktree goes only in <parent>/<name>.worktrees/ of the repo, and its path is written with every link resolved."
)]
pub struct WorktreeOptions {
    #[command(subcommand)]
    pub action: WorktreeAction,
}

/// The commands of `rex worktree`.
#[derive(clap::Subcommand, Clone, Debug)]
pub enum WorktreeAction {
    /// Make a worktree from a commit, on a new branch, on an existing branch at that commit, or detached, and list it.
    Add {
        /// The main checkout of the repo.
        repo: PathBuf,
        /// The absolute path of the new worktree, under <parent>/<name>.worktrees/.
        dir: PathBuf,
        /// The full hash of the commit the worktree starts from.
        commit: String,
        /// The branch to make there, or an existing branch whose tip is the commit; with none, the worktree is detached.
        #[arg(long)]
        branch: Option<String>,
    },

    /// Remove a worktree without --force, then unlist it.
    Remove {
        /// The main checkout of the repo.
        repo: PathBuf,
        /// The absolute path of the worktree, under <parent>/<name>.worktrees/.
        dir: PathBuf,
    },
}

/// Run `rex worktree` from `cwd`, against which a relative repo path is read.
///
/// # Errors
/// A [`WorktreeError`] that names what was refused or what failed. Nothing is
/// made, removed or written when an argument is refused.
pub fn run(cwd: &Path, options: WorktreeOptions) -> Result<(), WorktreeError> {
    match options.action {
        WorktreeAction::Add {
            repo,
            dir,
            commit,
            branch,
        } => add(&cwd.join(repo), &dir, &commit, branch.as_deref()),
        WorktreeAction::Remove { repo, dir } => remove(&cwd.join(repo), &dir),
    }
}

/// Git sets these when it runs a hook; left in place they would point git at
/// another repo than the one named.
const GIT_LOCATION_VARIABLES: [&str; 4] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_COMMON_DIR",
    "GIT_INDEX_FILE",
];

fn add(repo: &Path, dir: &Path, commit: &str, branch: Option<&str>) -> Result<(), WorktreeError> {
    let repo = main_checkout(repo)?;
    let root = root_of(&repo)?;
    let is_hash = matches!(commit.len(), 40 | 64) && commit.bytes().all(|b| b.is_ascii_hexdigit());
    if !is_hash {
        return Err(WorktreeError::NotCommitHash {
            commit: commit.to_owned(),
        });
    }
    if let Some(branch) = branch.filter(|branch| branch.starts_with('-')) {
        return Err(WorktreeError::BranchLikeOption {
            branch: branch.to_owned(),
        });
    }
    let planned = place_in(dir, &root)?;
    if fs::symlink_metadata(&planned).is_ok() {
        return Err(WorktreeError::Exists { dir: planned });
    }

    // The settings are read and checked before git runs, so a file rex cannot
    // edit leaves no worktree behind; the lock is held until the write.
    let registry = Registry::open(&repo)?;
    let dir_text = text_of(&planned)?.to_owned();
    let mut args = vec!["worktree".to_owned(), "add".to_owned()];
    match branch.map(|branch| (branch, branch_tip(&repo, branch))) {
        Some((branch, None)) => args.extend([
            "-b".to_owned(),
            branch.to_owned(),
            dir_text,
            commit.to_owned(),
        ]),
        Some((branch, Some(tip))) if tip.eq_ignore_ascii_case(commit) => {
            args.extend([dir_text, branch.to_owned()])
        }
        Some((branch, Some(tip))) => {
            return Err(WorktreeError::BranchAtOtherCommit {
                branch: branch.to_owned(),
                tip,
                commit: commit.to_owned(),
            });
        }
        None => args.extend(["--detach".to_owned(), dir_text, commit.to_owned()]),
    }
    git(&repo, &args)?;

    let made = planned
        .canonicalize()
        .ok()
        .filter(|made| is_below(made, &root) && is_linked_worktree_of(made, &repo))
        .ok_or_else(|| WorktreeError::NotLinkedAfterAdd {
            dir: planned.clone(),
            repo: repo.clone(),
        })?;
    let text = text_of(&made)?;
    let change = registry.add(text)?;
    print_result(text, change.word())
}

fn remove(repo: &Path, dir: &Path) -> Result<(), WorktreeError> {
    let repo = main_checkout(repo)?;
    let root = root_of(&repo)?;
    let planned = place_in(dir, &root)?;
    let registry = Registry::open(&repo)?;
    let entry = if fs::symlink_metadata(&planned).is_ok() {
        if !is_linked_worktree_of(&planned, &repo) {
            return Err(WorktreeError::NotWorktree { dir: planned, repo });
        }
        git(
            &repo,
            &[
                "worktree".into(),
                "remove".into(),
                text_of(&planned)?.into(),
            ],
        )?;
        planned
    } else {
        // The folder is gone already: git still has its record until a prune.
        git(&repo, &["worktree".into(), "prune".into()])?;
        planned
    };
    let text = text_of(&entry)?;
    let change = registry.remove(text)?;
    print_result(text, change.word())
}

/// `repo`, canonical, when it is a main checkout: `.git` is a folder.
fn main_checkout(repo: &Path) -> Result<PathBuf, WorktreeError> {
    let refused = || WorktreeError::NotMainCheckout {
        repo: repo.to_path_buf(),
    };
    let canonical = repo.canonicalize().map_err(|_| refused())?;
    let is_main = fs::symlink_metadata(canonical.join(".git")).is_ok_and(|meta| meta.is_dir());
    if is_main {
        Ok(canonical)
    } else {
        Err(refused())
    }
}

fn root_of(repo: &Path) -> Result<PathBuf, WorktreeError> {
    worktrees_root(repo).ok_or_else(|| WorktreeError::NoWorktreesFolder {
        repo: repo.to_path_buf(),
    })
}

/// `dir` with its links resolved, when it is a plain absolute path that
/// stays below `root` once they are.
fn place_in(dir: &Path, root: &Path) -> Result<PathBuf, WorktreeError> {
    if !is_plain_absolute(dir) {
        return Err(WorktreeError::NotPlainAbsolute {
            dir: dir.to_path_buf(),
        });
    }
    resolved(dir)
        .filter(|place| is_below(place, root))
        .ok_or_else(|| WorktreeError::Outside {
            dir: dir.to_path_buf(),
            root: root.to_path_buf(),
        })
}

fn text_of(path: &Path) -> Result<&str, WorktreeError> {
    path.to_str().ok_or_else(|| WorktreeError::NotUtf8 {
        dir: path.to_path_buf(),
    })
}

/// `git -C repo args`, with no variable of a hook that points git elsewhere.
fn git_command(repo: &Path, args: &[String]) -> Command {
    let mut command = Command::new("git");
    command.arg("-C").arg(repo).args(args);
    for variable in GIT_LOCATION_VARIABLES {
        command.env_remove(variable);
    }
    command
}

/// The commit that `branch` of `repo` points at, or `None` when the repo has no
/// such branch, or git cannot say.
fn branch_tip(repo: &Path, branch: &str) -> Option<String> {
    let reference = format!("refs/heads/{branch}");
    let args = ["rev-parse", "--verify", "--quiet", reference.as_str()].map(str::to_owned);
    let output = git_command(repo, &args).output().ok()?;
    let tip = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (output.status.success() && !tip.is_empty()).then_some(tip)
}

/// Run `git -C repo args`, with what it printed as the failure when it fails.
fn git(repo: &Path, args: &[String]) -> Result<(), WorktreeError> {
    let output = git_command(repo, args)
        .output()
        .map_err(|source| WorktreeError::GitNotRun { source })?;
    if output.status.success() {
        return Ok(());
    }
    let printed = String::from_utf8_lossy(&output.stderr);
    let output = printed
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    Err(WorktreeError::Git {
        args: args.join(" "),
        output,
    })
}

fn print_result(dir: &str, change: &str) -> Result<(), WorktreeError> {
    print_line_to_stdout(&format!("worktree={dir}"))?;
    print_line_to_stdout(&format!("settings={change}"))?;
    Ok(())
}
