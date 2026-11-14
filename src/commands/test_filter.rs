//! `rex test-filter`: an agent's test run shows only its summary or its failures,
//! because every line of output stays in the agent's context.

use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, Write};
use std::panic;
use std::path::{Path, PathBuf};
use std::process::{self, ExitCode};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::stdout::{print_line_to_stdout, print_to_stdout};

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
    let cut = panic::catch_unwind(|| cut(&output, see_every_line));
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

/// The text to print for `output`. A line is left out only when it is in the
/// exact form a runner prints for a passing test or a build step, so no line
/// of a failure is ever cut, whether the run passed or not. The filter cannot
/// see the exit status of the test command: `set -o pipefail` gives it to the
/// shell. `see_every_line` ends the note and tells where the whole output is.
fn cut(output: &str, see_every_line: &str) -> String {
    let lines: Vec<&str> = output.lines().collect();
    let mut kept: Vec<&str> = Vec::new();
    let mut place = Place::BeforeTests;
    for line in &lines {
        place = place.after(line);
        let is_cut =
            place != Place::InFailureReport && is_passing(line, place) && !is_summary(line);
        if !is_cut {
            kept.push(line);
        }
    }
    let note = format!(
        "rex test-filter: left out {} passing lines of {}. {see_every_line}",
        lines.len() - kept.len(),
        lines.len()
    );
    with_note(&kept, &note)
}

/// Where a line stands in cargo's output. Build steps come only before the
/// tests, and the report of a failed test is never cut.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Place {
    BeforeTests,
    InTests,
    InFailureReport,
}

impl Place {
    /// The place of `line`, which follows a line at `self`. A failure report
    /// ends only at cargo's `test result:` line, because the captured output of
    /// the failed test can hold any other line.
    fn after(self, line: &str) -> Place {
        if line.starts_with("test result:") {
            return Place::InTests;
        }
        let opens_report =
            line == "failures:" || (line.starts_with("---- ") && line.ends_with(" ----"));
        if self == Place::InFailureReport || opens_report {
            return Place::InFailureReport;
        }
        if is_cargo_tests_start(line) {
            return Place::InTests;
        }
        self
    }
}

/// `running 12 tests` or `running 1 test`, as cargo prints it before the tests.
fn is_cargo_tests_start(line: &str) -> bool {
    let Some(rest) = line.strip_prefix("running ") else {
        return false;
    };
    let Some((count, word)) = rest.split_once(' ') else {
        return false;
    };
    !count.is_empty()
        && count.bytes().all(|b| b.is_ascii_digit())
        && (word == "test" || word == "tests")
}

fn with_note(lines: &[&str], note: &str) -> String {
    let mut text: String = lines.iter().map(|line| format!("{line}\n")).collect();
    text.push_str(note);
    text.push('\n');
    text
}

/// True for a line that reports the totals of a run of one of the runners named
/// below. Such a line is always kept, also when it has the form of a passing
/// line, such as go's `ok` line of a package.
fn is_summary(line: &str) -> bool {
    let line = line.trim();
    let pytest_totals = line.starts_with('=')
        && [" passed", " failed", " error"]
            .iter()
            .any(|word| line.contains(word));
    let starts = [
        "test result:",      // cargo
        "Summary [",         // cargo nextest
        "ok ",               // go, one line for each package
        "ok\t",              // go
        "Tests:",            // jest
        "Test Suites:",      // jest
        "Tests ",            // vitest
        "Test Files ",       // vitest
        "Passed!",           // dotnet
        "Total tests:",      // dotnet
        "Finished in ",      // rspec, mix
        "[INFO] Tests run:", // maven
        "[INFO] BUILD ",     // maven
    ];
    pytest_totals
        || line.contains(" examples, ") // rspec
        || line.contains(" tests, ") // mix
        || line.contains(" tests passed, ") // ctest
        || line.contains(" passing (") // mocha
        || starts.iter().any(|start| line.starts_with(start))
}

/// True only for a line in the exact form a runner prints for a passing test or
/// a build step, so a line of a failure message is never taken for one.
fn is_passing(line: &str, place: Place) -> bool {
    is_cargo_test_ok(line)
        || (place == Place::BeforeTests && is_cargo_build_step(line))
        || is_go_pass(line)
        || is_pytest_pass(line)
        || is_jest_pass(line)
        || is_nextest_pass(line)
        || is_dotnet_pass(line)
}

/// `test name ... ok` or `test name ... ignored`, at the start of the line.
fn is_cargo_test_ok(line: &str) -> bool {
    let Some(rest) = line.strip_prefix("test ") else {
        return false;
    };
    let Some((name, result)) = rest.split_once(" ... ") else {
        return false;
    };
    let is_one_word = !name.is_empty() && !name.contains(' ');
    is_one_word && (result == "ok" || result.starts_with("ignored"))
}

/// `   Compiling name v1.2.3 (...)`: indented, a step word, a crate name and a
/// version; or `    Finished ... target(s) in 1.2s`.
fn is_cargo_build_step(line: &str) -> bool {
    if !line.starts_with(' ') {
        return false;
    }
    let words: Vec<&str> = line.split_whitespace().collect();
    let is_step = matches!(
        words.first(),
        Some(&("Compiling" | "Checking" | "Downloaded" | "Fresh"))
    );
    let has_version = words.get(2).is_some_and(|version| {
        version
            .strip_prefix('v')
            .is_some_and(|number| number.starts_with(|c: char| c.is_ascii_digit()))
    });
    let is_finished = words.first() == Some(&"Finished") && line.contains(" target(s) in ");
    (is_step && has_version) || is_finished
}

/// `=== RUN   TestName`, `--- PASS: TestName (0.00s)`, `PASS`, or
/// `ok  \tpackage\t0.01s`.
fn is_go_pass(line: &str) -> bool {
    let is_marker = ["=== RUN ", "=== PAUSE ", "=== CONT "]
        .iter()
        .any(|start| line.starts_with(start));
    let trimmed = line.trim_start();
    let is_pass = (trimmed.starts_with("--- PASS: ") || trimmed.starts_with("--- SKIP: "))
        && trimmed.ends_with("s)");
    is_marker || is_pass || line == "PASS" || line.starts_with("ok  \t")
}

/// `path::name PASSED`, with an optional `[ 10%]` after it.
fn is_pytest_pass(line: &str) -> bool {
    let Some((node, rest)) = line.split_once(" PASSED") else {
        return false;
    };
    let rest = rest.trim();
    let is_progress = rest.is_empty() || (rest.starts_with('[') && rest.ends_with("%]"));
    node.contains("::") && !node.contains(' ') && is_progress
}

/// `PASS path/to/file.test.js`, or an indented `✓ name`, `✔ name` or `√ name`.
fn is_jest_pass(line: &str) -> bool {
    let is_suite = line
        .strip_prefix("PASS ")
        .is_some_and(|file| !file.is_empty() && !file.contains(' '));
    let is_test = line.starts_with(' ')
        && ["✓ ", "✔ ", "√ "]
            .iter()
            .any(|mark| line.trim_start().starts_with(mark));
    is_suite || is_test
}

/// An indented `Passed Name [1 ms]`.
fn is_dotnet_pass(line: &str) -> bool {
    line.starts_with(' ')
        && line
            .trim_start()
            .strip_prefix("Passed ")
            .is_some_and(|rest| {
                rest.ends_with(']') && rest.contains(" [") && !rest.starts_with(' ')
            })
}

/// `        PASS [   0.004s] crate tests::name`, as cargo nextest prints it.
fn is_nextest_pass(line: &str) -> bool {
    line.trim_start()
        .strip_prefix("PASS [")
        .and_then(|rest| rest.split_once("s] "))
        .is_some_and(|(time, name)| !time.trim().is_empty() && !name.is_empty())
}
