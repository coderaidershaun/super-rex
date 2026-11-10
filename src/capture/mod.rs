//! Turns the events that `rex hook` logs into the record of sub-agent runs.

mod error;
mod tokens;
mod transcript;

use std::path::Path;

pub use error::CaptureError;
pub use tokens::{Reading, RunActivity, SETTLE, Step, TokenCounts, TokenTotal, TotalCheck};
pub use transcript::{TranscriptFacts, read_transcript};

/// Read the events that `rex hook` logged under `working_dir` into the record.
/// It does nothing yet.
pub fn take_in_events(_working_dir: &Path) {}
