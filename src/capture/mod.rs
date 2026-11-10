//! Turns the events that `rex hook` logs into the record of sub-agent runs.

use std::path::Path;

/// Read the events that `rex hook` logged under `working_dir` into the record.
/// It does nothing yet.
pub fn take_in_events(_working_dir: &Path) {}
