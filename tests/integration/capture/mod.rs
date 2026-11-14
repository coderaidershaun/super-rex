//! Integration tests of capture on the recorded session, `rex limit` and `rex status`.

mod limit;
mod main_transcript;
mod status;
mod transcript;

/// The recorded session in a store, shared with the contract tests.
#[allow(dead_code)]
#[path = "../../contract/capture/fixture.rs"]
mod recorded;

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use super_rex::capture::TokenCounts;

fn fixture(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/capture")
        .join(path)
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn count(value: &Value, key: &str) -> u64 {
    value[key]
        .as_u64()
        .unwrap_or_else(|| panic!("no count {key} in {value}"))
}

fn counts_from_expected(counts: &Value) -> TokenCounts {
    TokenCounts {
        input: count(counts, "input"),
        output: count(counts, "output"),
        cache_read: count(counts, "cache_read"),
        cache_creation: count(counts, "cache_write"),
    }
}

fn sum_over_models(by_model: &Value) -> TokenCounts {
    let mut total = TokenCounts::default();
    for counts in by_model.as_object().unwrap().values() {
        total += counts_from_expected(counts);
    }
    total
}
