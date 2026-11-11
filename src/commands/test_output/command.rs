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

/// The command that the test command's output is piped into. `rex init` allows
/// exactly this command, so the changed command needs no permission beyond the
/// one the test command already has.
pub const FILTER_COMMAND: &str = "rex test-filter";

/// The shell option in front of the changed command. With it, the pipe ends with
/// the exit status of the test command, because the filter always ends with 0.
/// `rex init` allows exactly this command too.
pub const PIPEFAIL_COMMAND: &str = "set -o pipefail";

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
/// agent already chose what it sees. `;`, `&`, `&&` and a line end make more than
/// one command, and the pipe would take only the last one. `#` starts a comment
/// that would hide the pipe. A backslash at the end would join the pipe to the
/// test command.
const UNSAFE_TEXT: [&str; 11] = ["|", ">", "<", ";", "&", "`", "$(", "\n", "\r", "\\", "#"];

/// `command` changed so that its output goes through `rex test-filter`, or
/// `None` when it is not a test command or is not safe to change.
pub(super) fn filtered_command(command: &str) -> Option<String> {
    if !is_test_run(command) {
        return None;
    }
    // No braces, quotes or subshell: Claude Code's check of a Bash command
    // refuses `{ ...; echo "..."; }` and `( ... )` as hard to read before it
    // runs. A plain pipe of two allowed commands passes it.
    Some(format!(
        "{PIPEFAIL_COMMAND}; {command} 2>&1 | {FILTER_COMMAND}"
    ))
}

/// True when `command` is one test command, alone. The command runs on the left
/// of a pipe, so a `cd` in it would no longer move the session to that folder.
fn is_test_run(command: &str) -> bool {
    let checked = command.replace("2>&1", "");
    let is_unsafe = UNSAFE_TEXT.iter().any(|text| checked.contains(text))
        || checked
            .split_whitespace()
            .any(|word| word == "exit" || OUTPUT_FLAGS.contains(&word));
    if is_unsafe {
        return false;
    }
    let words: Vec<&str> = checked
        .split_whitespace()
        .skip_while(|word| is_assignment(word))
        .collect();
    TEST_COMMANDS.iter().any(|test| {
        let test_words: Vec<&str> = test.split(' ').collect();
        words.starts_with(&test_words)
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
