//! `rex test-hook` and `rex test-filter`: an agent's test run shows only its summary
//! or its failures, because every line of output stays in the agent's context.

mod command;
mod filter;

pub use command::{FILTER_COMMAND, OUTPUT_FLAGS, PIPEFAIL_COMMAND, TEST_COMMANDS};

use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, Read, Write};
use std::panic;
use std::path::{Path, PathBuf};
use std::process::{self, ExitCode};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use super::hook::{IgnoredArguments, silently};
use super::stdout::{print_line_to_stdout, print_to_stdout};
use crate::error::RexError;

/// Called by Claude Code, not by people. Pipe a test command's output through
/// `rex test-filter`.
#[derive(clap::Args)]
pub struct TestHookOptions {
    #[command(flatten)]
    pub ignored: IgnoredArguments,
}

/// Reads one PreToolUse event on standard input. When it is a Bash call of a test
/// command, prints the same call with the command's output piped through
/// `rex test-filter`. Otherwise prints nothing, so the command runs as it is.
///
/// # Errors
/// None. Every failure, a panic included, ends in silence with success, so the
/// command runs unchanged.
pub fn run_hook(_cwd: &Path, _options: TestHookOptions) -> Result<(), RexError> {
    // Whether the command was changed is not reported: there is no one to tell.
    let _changed = silently(|| {
        let mut input = Vec::new();
        io::stdin().read_to_end(&mut input).ok()?;
        let event: Value = serde_json::from_slice(&input).ok()?;
        let output = hook_output(&event)?;
        print_line_to_stdout(&output.to_string()).ok()
    });
    Ok(())
}

/// What the hook prints for `event`, or `None` when the command stays as it is.
/// It never holds a permission decision: an `allow` would skip the person's
/// permission prompt, and the changed command needs none of its own.
pub fn hook_output(event: &Value) -> Option<Value> {
    if event.get("hook_event_name")?.as_str()? != "PreToolUse"
        || event.get("tool_name")?.as_str()? != "Bash"
    {
        return None;
    }
    // In auto mode Claude Code can refuse a changed call that asks to run outside
    // the sandbox, so such a call is left as it is.
    let tool_input = event.get("tool_input")?;
    if tool_input.get("dangerouslyDisableSandbox") == Some(&Value::Bool(true)) {
        return None;
    }
    // Claude Code replaces the whole input with the one returned, so every
    // other field is copied as it came.
    let mut input = event.get("tool_input")?.as_object()?.clone();
    let command = input.get("command")?.as_str()?;
    let filtered = command::filtered_command(command)?;
    input.insert("command".into(), Value::String(filtered));
    Some(json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "updatedInput": input,
        }
    }))
}

/// Read a test run's output on stdin, print only its summary or its failures,
/// and exit with its status.
#[derive(clap::Args)]
pub struct FilterOptions {}

/// Reads a test run's output on standard input and always exits with 0, so that
/// `set -o pipefail` in front of the pipe ends it with the exit status of the
/// test command.
///
/// It first prints the name of a file and writes each line to that file as the
/// line arrives. A run that is stopped before it ends, such as at the time limit
/// of the shell tool, then still leaves every line it printed. After the run it
/// prints every line except the proven passing ones.
pub fn run_filter(_options: FilterOptions) -> ExitCode {
    let mut copy = EveryLine::create();
    let see_every_line = match &copy {
        Some(every_line) => {
            let first = format!(
                "rex test-filter: every line of this run goes to {} as it comes.",
                every_line.path.display()
            );
            // A reader that cannot take the line leaves no one to tell.
            let _printed = print_line_to_stdout(&first);
            "To see every line, read the file named on the first line."
        }
        None => "To see every line, run the command again with | cat at its end.",
    };

    let mut input = Vec::new();
    let mut stdin = io::stdin().lock();
    loop {
        let start = input.len();
        match stdin.read_until(b'\n', &mut input) {
            Ok(0) => break,
            Ok(_) => {}
            // What was read is still shown below, so nothing is hidden.
            Err(_) => break,
        }
        if let Some(every_line) = &mut copy {
            every_line.write(&input[start..]);
        }
    }

    let output = String::from_utf8_lossy(&input);
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));
    let cut = panic::catch_unwind(|| filter::cut(&output, see_every_line));
    panic::set_hook(default_hook);
    // A filter that broke shows every line, so no failure is hidden.
    let text = cut.unwrap_or_else(|_| output.to_string());
    // A reader that cannot take the text leaves no one to tell.
    let _printed = print_to_stdout(&text);
    ExitCode::SUCCESS
}

/// The start and end of the name of each file that gets every line of a run.
const FILE_PREFIX: &str = "rex-test-output-";
const FILE_SUFFIX: &str = ".log";

/// How long a file of every line is kept. Each new run removes the older ones.
const FILE_KEPT_FOR: Duration = Duration::from_secs(24 * 60 * 60);

/// The file that gets every line of one run as the line arrives. It is kept
/// after the run, so the agent can read lines that the filter cut. Only its
/// owner can read it, because test output can hold secrets and the temporary
/// folder can be shared by every user.
struct EveryLine {
    path: PathBuf,
    file: Option<File>,
}

impl EveryLine {
    /// `None` when no file can be made in the temporary folder: the run then
    /// goes on with no copy.
    fn create() -> Option<EveryLine> {
        let folder = env::temp_dir();
        remove_old_files(&folder);
        let since_epoch = SystemTime::now().duration_since(UNIX_EPOCH).ok()?;
        let name = format!(
            "{FILE_PREFIX}{}-{}{FILE_SUFFIX}",
            process::id(),
            since_epoch.as_nanos()
        );
        let path = folder.join(name);
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        let file = options.open(&path).ok()?;
        Some(EveryLine {
            path,
            file: Some(file),
        })
    }

    /// Writes `line` at once, with no buffer. After a failed write the copy
    /// stops, and the run goes on.
    fn write(&mut self, line: &[u8]) {
        let is_written = self
            .file
            .as_mut()
            .is_some_and(|file| file.write_all(line).is_ok());
        if !is_written {
            self.file = None;
        }
    }
}

/// Removes each file of every line in `folder` older than [`FILE_KEPT_FOR`].
/// A file that cannot be removed, such as another user's, is left.
fn remove_old_files(folder: &Path) {
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with(FILE_PREFIX) || !name.ends_with(FILE_SUFFIX) {
            continue;
        }
        let age = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .ok()
            .and_then(|modified| modified.elapsed().ok());
        if age.is_some_and(|age| age > FILE_KEPT_FOR) {
            // A file left in place costs only space, so a failed removal is
            // not reported.
            let _removed = fs::remove_file(entry.path());
        }
    }
}
