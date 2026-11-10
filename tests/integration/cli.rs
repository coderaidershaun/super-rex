//! The command line as a whole: which commands `rex` offers, and that
//! `rex hook` never fails or prints, because either can stop an agent.

use std::collections::BTreeSet;

use assert_cmd::Command;
use tempfile::TempDir;

fn command_names_in_help(arguments: &[&str]) -> BTreeSet<String> {
    let assert = Command::cargo_bin("rex")
        .expect("find rex binary")
        .args(arguments)
        .arg("--help")
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8 stdout");

    // Each command starts a line indented by two spaces. A wrapped
    // description is indented further, so it is not read as a command.
    // The `help` command comes from clap, not from the design, and a command
    // group may turn it off, so it is left out.
    stdout
        .lines()
        .skip_while(|line| *line != "Commands:")
        .skip(1)
        .take_while(|line| !line.is_empty())
        .filter(|line| line.starts_with("  ") && !line.starts_with("   "))
        .filter_map(|line| line.split_whitespace().next())
        .filter(|name| *name != "help")
        .map(str::to_owned)
        .collect()
}

fn names(expected: &[&str]) -> BTreeSet<String> {
    expected.iter().map(|name| (*name).to_owned()).collect()
}

#[test]
fn help_lists_every_command_of_the_design_and_no_other() {
    let top = names(&[
        "init", "job", "hook", "status", "oms", "skills", "recall", "jobs", "watch", "feedback",
        "codebase", "smells", "commands",
    ]);
    let job = names(&["create", "plan", "close", "show"]);
    let skills = names(&["list", "check"]);

    assert_eq!(command_names_in_help(&[]), top);
    assert_eq!(command_names_in_help(&["job"]), job);
    assert_eq!(command_names_in_help(&["skills"]), skills);
}

#[test]
fn hook_exits_zero_and_prints_nothing_for_any_arguments_and_input() {
    let argument_sets: [&[&str]; 5] = [
        &[],
        &["--help"],
        &["-h"],
        &["--event", "PreToolUse", "unknown", "words"],
        &["-x", "--", "--version", "-V"],
    ];
    let inputs: [&[u8]; 3] = [b"", b"not json {", &[0xff, 0xfe, 0x00, 0x01]];
    let working_dir = TempDir::new().expect("create temp dir");

    for arguments in argument_sets {
        for input in inputs {
            let assert = Command::cargo_bin("rex")
                .expect("find rex binary")
                .current_dir(working_dir.path())
                .env_clear()
                .arg("hook")
                .args(arguments)
                .write_stdin(input)
                .assert();
            let output = assert.get_output();

            assert_eq!(output.status.code(), Some(0), "rex hook {arguments:?}");
            assert!(output.stdout.is_empty(), "stdout of rex hook {arguments:?}");
            assert!(output.stderr.is_empty(), "stderr of rex hook {arguments:?}");
        }
    }
}
