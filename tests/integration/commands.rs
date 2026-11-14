//! `rex commands` lists what the binary serves, so an agent that reads it finds
//! no command the plugin replaced.

use assert_cmd::Command;

/// The commands of rex 0.7 that the plugin's mod replaced: its hooks, and the
/// job commands whose records `rex ingest` now writes.
const REMOVED: [&str; 6] = [
    "rex hook",
    "rex gate",
    "rex test-hook",
    "rex job create",
    "rex job plan",
    "rex job close",
];

#[test]
fn rex_commands_lists_no_hook_gate_or_test_hook() {
    let output = Command::cargo_bin("rex")
        .unwrap()
        .arg("commands")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let listed = String::from_utf8(output).unwrap();

    let paths: Vec<&str> = listed
        .lines()
        .map(|line| line.split("  —").next().unwrap_or_default().trim_end())
        .collect();
    assert!(paths.contains(&"rex init"), "got: {listed}");
    for removed in REMOVED {
        assert!(!paths.contains(&removed), "{removed} is listed: {listed}");
    }
    for removed in REMOVED.map(|path| path.trim_start_matches("rex ")) {
        Command::cargo_bin("rex")
            .unwrap()
            .arg(removed)
            .assert()
            .code(2);
    }
}
