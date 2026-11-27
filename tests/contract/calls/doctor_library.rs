//! `rex doctor` starts the library server as Claude Code does and names what it answers: the
//! library and its version, or why it is unreachable, and a plugin manifest of another version.

use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{Value, json};

use super::harness::{World, crate_root};

const SKEW_LINE_START: &str = "problem=version skew: ";

/// The `version` that `rex skills serve <args>` answers to `head`.
pub(super) fn served_version(args: &[&str]) -> String {
    let mut child = Command::new(env!("CARGO_BIN_EXE_rex"))
        .args(["skills", "serve"])
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("start rex skills serve");
    let requests = [
        json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} }),
        json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/call",
                "params": { "name": "head", "arguments": {} } }),
    ];
    let mut stdin = child.stdin.take().expect("stdin");
    for request in requests {
        writeln!(stdin, "{request}").expect("write request");
    }
    drop(stdin);
    let output = child.wait_with_output().expect("server output");
    let answer: Value = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .find(|answer| answer["id"] == 2)
        .expect("an answer to head");
    let text = answer["result"]["content"][0]["text"].as_str().unwrap();
    let head: Value = serde_json::from_str(text).unwrap();
    head["version"].as_str().unwrap().to_owned()
}

fn fixture_plugin(name: &str) -> PathBuf {
    crate_root()
        .join("tests/fixtures/doctor-plugins")
        .join(name)
}

/// `PATH` with the folder of the `rex` under test first, so a manifest's
/// `"command": "rex"` starts this build, as it starts the installed one.
fn path_with_this_rex() -> OsString {
    let folder = Path::new(env!("CARGO_BIN_EXE_rex")).parent().unwrap();
    let mut folders = vec![folder.to_path_buf()];
    folders.extend(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    ));
    std::env::join_paths(folders).unwrap()
}

fn doctor(world: &World, args: &[&str]) -> String {
    let mut full = vec!["doctor"];
    full.extend_from_slice(args);
    let output = world
        .rex(&full)
        .env("PATH", path_with_this_rex())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    String::from_utf8(output.stdout).unwrap()
}

fn skew_lines(stdout: &str) -> Vec<&str> {
    stdout
        .lines()
        .filter(|line| line.starts_with(SKEW_LINE_START))
        .collect()
}

#[test]
fn doctor_names_the_library_that_its_own_binary_serves() {
    let stdout = doctor(&World::new(), &[]);

    let line = format!("library=super-rex {}", served_version(&[]));
    assert!(stdout.lines().any(|each| each == line), "{stdout}");
    assert!(!stdout.contains("problem=library server"), "{stdout}");
}

#[test]
fn doctor_starts_the_server_that_the_plugin_manifest_declares() {
    let plugin = crate_root().join("plugin");
    let stdout = doctor(&World::new(), &["--plugin", plugin.to_str().unwrap()]);

    let library = plugin.join("library");
    let folder_version = served_version(&["--library", library.to_str().unwrap()]);
    assert_eq!(folder_version, served_version(&[]));
    let line = format!("library=super-rex {folder_version}");
    assert!(stdout.lines().any(|each| each == line), "{stdout}");
    assert!(skew_lines(&stdout).is_empty(), "{stdout}");
}

#[test]
fn doctor_reports_an_unreachable_server_whose_command_does_not_exist() {
    let plugin = fixture_plugin("missing-command");
    let stdout = doctor(&World::new(), &["--plugin", plugin.to_str().unwrap()]);

    assert!(stdout.starts_with("doctor=problems\n"), "{stdout}");
    assert!(
        stdout.lines().any(|line| line == "library=unreachable"),
        "{stdout}"
    );
    let problem = stdout
        .lines()
        .find(|line| line.starts_with("problem=library server: "))
        .unwrap_or_else(|| panic!("no library server problem in {stdout}"));
    assert!(problem.contains("rex-no-such-command"), "{problem}");
}

#[test]
fn doctor_names_both_versions_when_the_plugin_manifest_is_another_version() {
    let binary = env!("CARGO_PKG_VERSION");
    let plugin = fixture_plugin("old-version");
    let stdout = doctor(&World::new(), &["--plugin", plugin.to_str().unwrap()]);

    assert!(stdout.starts_with("doctor=problems\n"), "{stdout}");
    assert_eq!(
        skew_lines(&stdout),
        [format!(
            "problem=version skew: plugin 0.8.0, binary {binary}; install the same version of both"
        )]
    );
    let line = format!("library=old-plugin {}", served_version(&[]));
    assert!(stdout.lines().any(|each| each == line), "{stdout}");
}

#[test]
fn doctor_prints_one_skew_line_when_the_store_and_the_manifest_name_one_version() {
    let world = World::new();
    for path in [
        "fixtures/shared/ingest/batch-open-stream.json",
        "fixtures/shared/ingest/batch-example.json",
    ] {
        assert!(world.ingest_file(path).status.success());
    }
    let plugin = fixture_plugin("old-version");
    let stdout = doctor(&world, &["--plugin", plugin.to_str().unwrap()]);

    assert!(stdout.contains("\nplugin=0.8.0\n"), "{stdout}");
    assert_eq!(skew_lines(&stdout).len(), 1, "{stdout}");
}
