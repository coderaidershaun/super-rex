//! `rex skills index` prints a byte-identical document on every run over the
//! same folders.

use std::path::Path;
use std::process::Command;

fn index_bytes(libraries: &[&Path]) -> Vec<u8> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rex"));
    command.args(["skills", "index"]);
    for library in libraries {
        command.arg("--library").arg(library);
    }
    let output = command.output().expect("run rex skills index");
    assert!(
        output.status.success(),
        "rex skills index failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

#[test]
fn index_file_is_byte_identical_on_two_runs() {
    let main = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/skills-search");
    let unrelated = main.join("unrelated");

    let first = index_bytes(&[&main, &unrelated]);
    let second = index_bytes(&[&main, &unrelated]);

    assert!(!first.is_empty());
    assert!(first == second, "the two runs differ");
}
