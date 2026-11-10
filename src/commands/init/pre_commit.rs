//! Writes the pre-commit config and installs its git hooks, so leaked secrets,
//! vulnerable dependencies and failing Rust checks stop a commit or a push.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::process::Command;

use super::error::{InitError, ToolStderr};
use super::file_change::{ProjectWrite, contains_marker, write_unless_linked};
use super::on_path::is_missing;
use super::print_line_to_stderr;
use crate::error::RexError;

pub(super) const CONFIG_FILENAME: &str = ".pre-commit-config.yaml";

/// The command that turns the config into git hooks. Printed as a hint
/// when the tool is missing, so the text is kept in one place.
///
/// `--install-hooks` also downloads the hook programs now. Agents commit
/// inside the Claude Code sandbox, which cannot fill that download store, so
/// it must be filled here, by the person's own run of `rex init`.
pub(super) const INSTALL_COMMAND: &str =
    "pre-commit install --install-hooks --hook-type pre-commit --hook-type pre-push";

/// The first line of the config. Finding it means rex wrote the file, so a
/// second run has nothing to say about it.
const CONFIG_MARKER: &str = "# rex:pre-commit";

/// The programs the hooks in the config run, besides pre-commit itself, and
/// the git action each hook runs at.
const HOOK_TOOLS: [(&str, &str); 2] = [("gitleaks", "commit"), ("trivy", "push")];

const CONFIG: &str = "\
# rex:pre-commit
# Installed by `rex init`. Needs pre-commit, gitleaks and trivy on PATH.
repos:
  - repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v6.0.0
    hooks:
      - id: check-added-large-files
        args: [--maxkb=1024]
      - id: check-merge-conflict
      - id: check-toml
      - id: check-yaml
      - id: detect-private-key
      - id: end-of-file-fixer
      - id: trailing-whitespace

  - repo: local
    hooks:
      - id: gitleaks
        name: gitleaks
        entry: gitleaks git --pre-commit --staged --redact --verbose
        language: system
        pass_filenames: false
      - id: trivy
        name: trivy secrets, vulnerabilities and misconfigurations
        entry: trivy fs --timeout 15m --quiet --skip-version-check --scanners secret,vuln,misconfig --severity HIGH,CRITICAL --exit-code 1 --skip-dirs target,node_modules,.claude,.rex --skip-files .env .
        language: system
        pass_filenames: false
        stages: [pre-push]
      - id: cargo-fmt
        name: cargo fmt
        entry: cargo fmt --all -- --check
        language: system
        types: [rust]
        pass_filenames: false
      - id: cargo-clippy
        name: cargo clippy
        entry: cargo clippy --workspace --all-targets -- -D warnings
        language: system
        types: [rust]
        pass_filenames: false
      - id: cargo-test
        name: cargo test
        entry: cargo test --workspace
        language: system
        types: [rust]
        pass_filenames: false
        stages: [pre-push]
";

/// The outcome of making sure the config file is there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ConfigWrite {
    Created,
    /// Rex wrote the config on an earlier run.
    AlreadyWritten,
    /// Someone else's config is there. It is never replaced.
    SkippedExists,
    /// The path is a symbolic link, so nothing was written through it.
    SkippedLink,
}

/// The outcome of asking the pre-commit tool to install its hooks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum HookInstall {
    Installed,
    /// `pre-commit` is not on PATH.
    SkippedNoTool,
}

/// Write `cwd/.pre-commit-config.yaml` unless a config is already there, or
/// the path is a symbolic link; see [`write_unless_linked`].
pub(super) fn ensure_config(cwd: &Path) -> Result<ConfigWrite, RexError> {
    let path = cwd.join(CONFIG_FILENAME);
    match fs::read(&path) {
        Ok(existing) if contains_marker(&existing, CONFIG_MARKER) => {
            return Ok(ConfigWrite::AlreadyWritten);
        }
        Ok(_) => return Ok(ConfigWrite::SkippedExists),
        Err(source) if source.kind() == ErrorKind::NotFound => {}
        Err(source) => return Err(RexError::Io { path, source }),
    }
    match write_unless_linked(&path, CONFIG.as_bytes())? {
        ProjectWrite::Written => Ok(ConfigWrite::Created),
        ProjectWrite::SkippedLink => Ok(ConfigWrite::SkippedLink),
    }
}

/// Say on stderr which of the programs the config's hooks run are not on
/// PATH. Without one of them, its hook fails every time it runs.
pub(super) fn warn_about_missing_hook_tools() {
    for (tool, action) in HOOK_TOOLS.into_iter().filter(|(tool, _)| is_missing(tool)) {
        print_line_to_stderr(&format!(
            "rex init: warning: `{tool}` not found on PATH. Its hook in {CONFIG_FILENAME} fails every {action} until it is installed"
        ));
    }
}

/// Run the pre-commit tool so the config becomes real git hooks. Its own
/// output is not shown; a non-zero exit is an error.
///
/// # Errors
/// [`InitError::Rex`] holding [`RexError::Io`] if the tool cannot be started
/// for a reason other than being missing, and
/// [`InitError::PreCommitInstall`] if it exits non-zero.
pub(super) fn install_hooks(cwd: &Path) -> Result<HookInstall, InitError> {
    let mut parts = INSTALL_COMMAND.split(' ');
    let tool = parts.next().unwrap_or_default();
    let output = match Command::new(tool).args(parts).current_dir(cwd).output() {
        Ok(output) => output,
        Err(source) if source.kind() == ErrorKind::NotFound => {
            return Ok(HookInstall::SkippedNoTool);
        }
        Err(source) => {
            return Err(RexError::Io {
                path: cwd.to_path_buf(),
                source,
            }
            .into());
        }
    };

    if !output.status.success() {
        return Err(InitError::PreCommitInstall {
            status: output.status,
            stderr: ToolStderr::from_output(&output.stderr),
        });
    }
    Ok(HookInstall::Installed)
}
