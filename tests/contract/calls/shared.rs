//! The other files of `fixtures/shared/` that the binary pins for the plugin:
//! the 44 cases of `rex oms`, and the hash vectors both sides compute.

use serde_json::Value;
use sha2::{Digest, Sha256};

use super::harness::{World, crate_root};

fn shared(path: &str) -> Value {
    let text = std::fs::read_to_string(crate_root().join("fixtures/shared").join(path)).unwrap();
    serde_json::from_str(&text).unwrap()
}

// The plugin picks each role's agent from the line `rex oms --json` prints; a
// case that prints another line would give a role another model unseen.
#[test]
fn each_oms_case_prints_its_line() {
    let cases = shared("oms/cases.json");
    let cases = cases["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 44);
    let world = World::new();
    for case in cases {
        let argv: Vec<&str> = case["argv"]
            .as_array()
            .unwrap()
            .iter()
            .map(|arg| arg.as_str().unwrap())
            .collect();
        assert_eq!(argv[..3], ["rex", "oms", "--json"]);
        let output = world.rex(&argv[1..]).output().unwrap();
        assert_eq!(output.status.code(), Some(0), "case {}", case["number"]);
        let printed: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            printed, case["expect"]["stdout_json"],
            "case {}",
            case["number"]
        );
        assert_eq!(printed["inputs"], case["inputs"], "case {}", case["number"]);
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

// A skill file's hash names the version an agent was given; if the two sides
// hashed one file differently, every manifest would look changed.
#[test]
fn hash_vectors_are_what_the_binary_computes() {
    let vectors = shared("hashes/vectors.json");
    for file in vectors["files"].as_array().unwrap() {
        let text = file["text"].as_str().unwrap();
        assert_eq!(
            file["hash"].as_str().unwrap(),
            format!("sha256:{}", sha256_hex(text.as_bytes())),
            "{}",
            file["name"]
        );
    }

    // The binary stores exactly the vector's hash for a stream's secret.
    let world = World::new();
    assert!(
        world
            .ingest_file("fixtures/shared/ingest/batch-open-stream.json")
            .status
            .success()
    );
    let stored: String = world
        .store()
        .conn()
        .query_row("SELECT secret_sha256 FROM ingest_stream", [], |row| {
            row.get(0)
        })
        .unwrap();
    let secret = vectors["secrets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|vector| vector["secret"] == "1".repeat(64))
        .unwrap();
    assert_eq!(Value::from(stored), secret["secret_sha256"]);
    for vector in vectors["secrets"].as_array().unwrap() {
        let secret = vector["secret"].as_str().unwrap();
        assert_eq!(
            vector["secret_sha256"],
            Value::from(sha256_hex(secret.as_bytes()))
        );
    }
}
