//! The checks `rex init` makes before it writes anything, in the order the
//! person reads their messages.

use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fmt};

use super::error::InitError;
use super::on_path::{first_on_path, is_missing};
use super::print_line_to_stderr;

/// The oldest Claude Code version the safety settings were tested on. An
/// older one might ignore a setting it does not know and run without it;
/// this is inferred, not tested.
const MIN_CLAUDE_CODE_VERSION: ClaudeVersion = ClaudeVersion {
    major: 2,
    minor: 1,
    patch: 322,
};

/// A Claude Code version. The derived order compares the parts as numbers,
/// major first, so 2.1.1000 is newer than 2.1.322.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ClaudeVersion {
    major: u32,
    minor: u32,
    patch: u32,
}

impl ClaudeVersion {
    /// The version in the first word of `line`, such as `2.1.322` in
    /// `2.1.322 (Claude Code)`, or `None` when that word is not three numbers
    /// joined by dots.
    fn from_first_word(line: &str) -> Option<Self> {
        let word = line.split_whitespace().next()?;
        let mut parts = word.split('.').map(str::parse::<u32>);
        let (Some(Ok(major)), Some(Ok(minor)), Some(Ok(patch)), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return None;
        };
        Some(Self {
            major,
            minor,
            patch,
        })
    }
}

impl fmt::Display for ClaudeVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            major,
            minor,
            patch,
        } = self;
        write!(formatter, "{major}.{minor}.{patch}")
    }
}

/// Make every check that must pass before the install starts.
///
/// # Errors
/// [`InitError::NotRepoRoot`] if `cwd` holds no `.git`, and
/// [`InitError::GitIsFile`] if its `.git` is not a directory. Then
/// [`InitError::ClaudeNotFound`], [`InitError::ClaudeNotRun`],
/// [`InitError::ClaudeVersionUnreadable`] or [`InitError::ClaudeTooOld`] if
/// no Claude Code that is new enough answers `claude --version`.
///
/// A check whose failure must not stop the install, such as the sandbox
/// tools on Linux, gives its failure in the list it returns.
pub(super) fn check(cwd: &Path) -> Result<Vec<InitError>, InitError> {
    check_repo_root(cwd)?;
    check_claude_version()?;
    warn_if_gh_is_missing();
    let failures = missing_sandbox_tools().into_iter().collect();
    warn_if_this_rex_is_not_on_path();
    Ok(failures)
}

/// rex looks at `.git` itself and runs no `git` command, so it does not
/// need `git` on PATH. Only the root of the main checkout passes: the store,
/// the settings and the skills belong there, and a worktree is removed after
/// its job with whatever was written in it.
fn check_repo_root(cwd: &Path) -> Result<(), InitError> {
    let git = cwd.join(".git");
    if git.is_dir() {
        Ok(())
    } else if git.exists() {
        Err(InitError::GitIsFile {
            cwd: canonical(cwd),
        })
    } else {
        Err(InitError::NotRepoRoot {
            cwd: canonical(cwd),
        })
    }
}

/// `cwd` as an absolute path, for a message, or as it is when that fails.
fn canonical(cwd: &Path) -> PathBuf {
    cwd.canonicalize().unwrap_or_else(|_| cwd.to_path_buf())
}

/// Run `claude --version`, the one program the checks start. Only what it
/// prints counts, not how it exits.
fn check_claude_version() -> Result<(), InitError> {
    let output = match Command::new("claude").arg("--version").output() {
        Ok(output) => output,
        Err(source) if source.kind() == ErrorKind::NotFound => {
            return Err(InitError::ClaudeNotFound);
        }
        Err(source) => return Err(InitError::ClaudeNotRun { source }),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_line = stdout.lines().next().unwrap_or_default().trim();
    let Some(found) = ClaudeVersion::from_first_word(first_line) else {
        let printed = if first_line.is_empty() {
            "nothing"
        } else {
            first_line
        };
        return Err(InitError::ClaudeVersionUnreadable {
            printed: printed.to_string(),
        });
    };

    if found < MIN_CLAUDE_CODE_VERSION {
        return Err(InitError::ClaudeTooOld {
            found: found.to_string(),
            minimum: MIN_CLAUDE_CODE_VERSION.to_string(),
        });
    }
    Ok(())
}

/// Without `gh`, delivery cannot open pull requests, but everything else
/// works, so this is a warning and not a failure.
fn warn_if_gh_is_missing() {
    if is_missing("gh") {
        print_line_to_stderr(
            "rex init: warning: gh not found on PATH. Delivery works without pull requests until it is installed.",
        );
    }
}

/// The hooks and the skills start `rex` by its name on PATH. When that is
/// no `rex`, or another one, their runs are lost or go to that one, so the
/// person is told. This sees the PATH of the shell that runs `rex init`,
/// which may not be the PATH Claude Code runs with.
fn warn_if_this_rex_is_not_on_path() {
    let Ok(this) = env::current_exe().and_then(|exe| exe.canonicalize()) else {
        return;
    };
    let warning = match first_on_path("rex") {
        None => "rex init: warning: rex not found on PATH. The hooks log nothing and the skills cannot run rex until it is.".to_string(),
        Some(found) => {
            let Ok(canonical) = found.canonicalize() else {
                return;
            };
            if canonical == this {
                return;
            }
            format!(
                "rex init: warning: the rex on PATH is {}, not this one ({}). The hooks and the skills run that one; put this rex first on PATH.",
                found.display(),
                this.display()
            )
        }
    };
    print_line_to_stderr(&warning);
}

/// The tools the Claude Code sandbox needs on Linux. Other systems have
/// what it needs built in.
const SANDBOX_TOOLS: [&str; 2] = ["bwrap", "socat"];

/// The failure that names each sandbox tool missing from PATH, on Linux.
///
/// The install still runs in full, because the settings it writes make
/// Claude Code refuse to start without a sandbox. Stopping here would leave
/// no settings, and Claude Code would then start with no sandbox at all.
fn missing_sandbox_tools() -> Option<InitError> {
    if !cfg!(target_os = "linux") {
        return None;
    }
    let missing: Vec<&str> = SANDBOX_TOOLS
        .into_iter()
        .filter(|tool| is_missing(tool))
        .collect();
    if missing.is_empty() {
        return None;
    }
    Some(InitError::SandboxToolsMissing {
        missing: missing.join(" and "),
    })
}
