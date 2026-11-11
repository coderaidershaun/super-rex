//! Cuts the output of one test run to what an agent needs: every line except
//! the ones proven to report a passing test or a build step.

/// The text to print for `output`. A line is left out only when it is in the
/// exact form a runner prints for a passing test or a build step, so no line
/// of a failure is ever cut, whether the run passed or not. The filter cannot
/// see the exit status of the test command: `set -o pipefail` gives it to the
/// shell. `see_every_line` ends the note and tells where the whole output is.
pub(super) fn cut(output: &str, see_every_line: &str) -> String {
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
