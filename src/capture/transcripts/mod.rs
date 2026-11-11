//! Readers of Claude Code transcript files: one for a sub-agent run, one for the main
//! session's part in a job.

mod main_session;
mod run;

pub(crate) use main_session::read_main_transcript;
pub use run::{TranscriptFacts, read_transcript};
pub(crate) use run::{blocks_of, content_text, read_file, records, text_at, usage_of};
