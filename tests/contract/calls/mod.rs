//! The binary side of each call the plugin makes to `rex`: each file of
//! `fixtures/shared/calls/` run against the real binary gives the output the file
//! names, so the plugin's tests of the same file meet the same promise.

mod doctor;
pub(crate) mod harness;
mod ingest;
mod shared;

use std::collections::BTreeSet;
use std::fs;

use harness::{World, crate_root, read_call};

fn check(name: &str) {
    let call = read_call(name);
    assert_eq!(call.name, name, "the file names its own call");
    World::new().check(&call);
}

#[test]
fn call_ingest() {
    check("ingest");
}

#[test]
fn call_ingest_empty() {
    check("ingest-empty");
}

#[test]
fn call_ingest_repeat() {
    check("ingest-repeat");
}

#[test]
fn call_ingest_wrong_secret() {
    check("ingest-wrong-secret");
}

#[test]
fn call_oms() {
    check("oms");
}

#[test]
fn call_recall() {
    check("recall");
}

#[test]
fn call_job_show() {
    check("job-show");
}

#[test]
fn call_jobs() {
    check("jobs");
}

#[test]
fn call_doctor() {
    check("doctor");
}

#[test]
fn call_version() {
    check("version");
}

#[test]
fn call_limit() {
    check("limit");
}

#[test]
fn call_tree_show() {
    check("tree-show");
}

/// The calls tested above, by file name.
const TESTED: [&str; 12] = [
    "doctor",
    "ingest",
    "ingest-empty",
    "ingest-repeat",
    "ingest-wrong-secret",
    "job-show",
    "jobs",
    "limit",
    "oms",
    "recall",
    "tree-show",
    "version",
];

// A call file with no test here is a promise of the binary that nothing checks,
// and the plugin's own test of it would pass against a binary that broke it.
#[test]
fn every_call_file_has_a_test() {
    let folder = crate_root().join("fixtures/shared/calls");
    let files: BTreeSet<String> = fs::read_dir(&folder)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter_map(|name| name.strip_suffix(".json").map(str::to_owned))
        .collect();
    let tested: BTreeSet<String> = TESTED.iter().map(|name| (*name).to_owned()).collect();
    assert_eq!(files, tested);
}
