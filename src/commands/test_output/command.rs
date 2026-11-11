//! Which shell commands run tests, and the command that runs one of them with
//! its output piped through `rex test-filter`.

/// Every command that runs tests, as the words it starts with. This list is the
/// only place that decides which commands get their output cut.
pub const TEST_COMMANDS: [&str; 28] = [
    "cargo test",
    "cargo nextest run",
    "go test",
    "pytest",
    "python -m pytest",
    "python3 -m pytest",
    "uv run pytest",
    "npm test",
    "npm run test",
    "pnpm test",
    "pnpm run test",
    "yarn test",
    "bun test",
    "deno test",
    "npx jest",
    "npx vitest",
    "jest",
    "vitest",
    "mix test",
    "rspec",
    "bundle exec rspec",
    "dotnet test",
    "mvn test",
    "gradle test",
    "./gradlew test",
    "swift test",
    "ctest",
    "make test",
];

/// The start of the last line of the changed command's output, which carries
/// the exit status of the test command to the filter.
pub(super) const STATUS_MARKER: &str = "rex-test-exit-status=";

/// The command that the test command's output is piped into. `rex init` allows
/// exactly this command, so the changed command needs no permission beyond the
/// one the test command already has.
pub const FILTER_COMMAND: &str = "rex test-filter";

/// Words that ask the runner for output the agent wants to read, such as captured
/// prints, the list of tests, help, or a line for each test. A command with one
/// of these is left as it is, because its summary would hide what was asked for.
pub const OUTPUT_FLAGS: [&str; 17] = [
    "--nocapture",
    "--no-capture",
    "--show-output",
    "--list",
    "--listTests",
    "--help",
    "-h",
    "--version",
    "-s",
    "--capture=no",
    "--collect-only",
    "--co",
    "-v",
    "-vv",
    "-vvv",
    "--verbose",
    "--no-run",
];

/// Text that makes a command unsafe to change. A pipe or a redirect means the
/// agent already chose what it sees. `;`, a single `&` and `exit` can skip the
/// line that reports the status, or make it report another command's status.
/// A backslash at the end would join the status line to the test command.
const UNSAFE_TEXT: [&str; 10] = ["|", ">", "<", ";", "&", "`", "$(", "\n", "\r", "\\"];

/// `command` changed so that its output goes through `rex test-filter`, or
/// `None` when it is not a test command or is not safe to change.
pub(super) fn filtered_command(command: &str) -> Option<String> {
    if !is_test_run(command) {
        return None;
    }
    // The line end after the command also ends any `#` comment in it. The
    // status is printed with `echo`, which Claude Code's permission check
    // counts as read-only, so the changed command needs no rule for it.
    Some(format!(
        "{{ {command}\necho \"{STATUS_MARKER}$?\"; }} 2>&1 | {FILTER_COMMAND}"
    ))
}

/// True when every part of `command` between `&&` is a test command. A `cd` is
/// not allowed: the changed command runs in a subshell, so a `cd` in it would
/// no longer move the session to that folder.
fn is_test_run(command: &str) -> bool {
    let checked = command.replace("2>&1", "").replace("&&", " ");
    let is_unsafe = UNSAFE_TEXT.iter().any(|text| checked.contains(text))
        || checked
            .split_whitespace()
            .any(|word| word == "exit" || OUTPUT_FLAGS.contains(&word));
    if is_unsafe {
        return false;
    }
    command.replace("2>&1", "").split("&&").all(|part| {
        let words: Vec<&str> = part
            .split_whitespace()
            .skip_while(|word| is_assignment(word))
            .collect();
        TEST_COMMANDS.iter().any(|test| {
            let test_words: Vec<&str> = test.split(' ').collect();
            words.starts_with(&test_words)
        })
    })
}

/// True for a word such as `RUST_LOG=debug` that sets a variable for the command.
fn is_assignment(word: &str) -> bool {
    let Some((name, _value)) = word.split_once('=') else {
        return false;
    };
    let starts_well = name
        .chars()
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_');
    starts_well
        && name
            .chars()
            .all(|letter| letter.is_ascii_alphanumeric() || letter == '_')
}
