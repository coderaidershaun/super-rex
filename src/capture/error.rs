//! Why capture could not finish. A transcript that is missing or damaged is not an
//! error; only a file that exists and cannot be read, or a failing store, is.

use std::path::PathBuf;

use crate::error::CommandError;
use crate::store::StoreError;

/// Failure of capture.
#[derive(thiserror::Error, Debug)]
pub enum CaptureError {
    #[error("failed to read transcript {}", path.display())]
    ReadTranscript {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read the event log {}", path.display())]
    ReadLog {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(transparent)]
    Store(#[from] StoreError),
}

impl From<rusqlite::Error> for CaptureError {
    fn from(source: rusqlite::Error) -> Self {
        CaptureError::Store(StoreError::from(source))
    }
}

impl CommandError for CaptureError {}
