//! Installs the super-rex plugin for the repo through the `claude` command, at
//! project scope, so every session in the repo loads it and no file is copied.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::process::Command;

use serde_json::Value;

use super::error::{InitError, ToolStderr};

/// The marketplace `rex init` adds when the person names none.
pub(super) const DEFAULT_SOURCE: &str = "coderaidershaun/super-rex";

/// The plugin and the marketplace that ships it, as `claude plugin install` names them.
pub(super) const PLUGIN: &str = "super-rex@super-rex";

/// The project settings file where Claude Code records a plugin installed at project scope.
const PROJECT_SETTINGS: &str = ".claude/settings.json";

/// The outcome of the install step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PluginInstall {
    Installed,
    AlreadyInstalled,
}

/// Add the marketplace at `source`, then install the plugin from it, both at
/// project scope. A plugin that the project settings already turn on is left
/// alone, so a second run starts no `claude` command.
///
/// # Errors
/// [`InitError::PluginNotRun`] when `claude` cannot be started, and
/// [`InitError::PluginCommand`] when one of the two commands fails.
pub(super) fn install(cwd: &Path, source: &str) -> Result<PluginInstall, InitError> {
    if is_enabled(cwd) {
        return Ok(PluginInstall::AlreadyInstalled);
    }
    run_claude(
        cwd,
        &["plugin", "marketplace", "add", source, "--scope", "project"],
    )?;
    run_claude(cwd, &["plugin", "install", PLUGIN, "--scope", "project"])?;
    Ok(PluginInstall::Installed)
}

/// True when the project settings turn the plugin on. A file that cannot be
/// read or is not JSON counts as not, and the install runs.
fn is_enabled(cwd: &Path) -> bool {
    let Ok(bytes) = fs::read(cwd.join(PROJECT_SETTINGS)) else {
        return false;
    };
    let Ok(settings) = serde_json::from_slice::<Value>(&bytes) else {
        return false;
    };
    settings
        .get("enabledPlugins")
        .and_then(|plugins| plugins.get(PLUGIN))
        == Some(&Value::Bool(true))
}

fn run_claude(cwd: &Path, args: &[&str]) -> Result<(), InitError> {
    let command = format!("claude {}", args.join(" "));
    let output = match Command::new("claude").args(args).current_dir(cwd).output() {
        Ok(output) => output,
        Err(source) if source.kind() == ErrorKind::NotFound => {
            return Err(InitError::ClaudeNotFound);
        }
        Err(source) => return Err(InitError::PluginNotRun { command, source }),
    };
    if output.status.success() {
        return Ok(());
    }
    Err(InitError::PluginCommand {
        command,
        status: output.status,
        stderr: ToolStderr::from_output(&output.stderr),
    })
}
