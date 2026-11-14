//! What can go wrong in `rex job show` that the store cannot see. Each text
//! says what failed, then the fix.

use crate::capture::CaptureError;
use crate::error::{CommandError, FailureClass, RexError};
use crate::store::StoreError;

/// Failure of `rex job show`.
#[derive(thiserror::Error, Debug)]
pub enum JobError {
    #[error(
        "give a job ID; CLAUDE_CODE_SESSION_ID is not set, so there is no session to find the open job of"
    )]
    NoJobArgument,

    #[error(transparent)]
    Store(#[from] StoreError),

    #[error(transparent)]
    Capture(#[from] CaptureError),

    #[error(transparent)]
    Rex(#[from] RexError),
}

impl CommandError for JobError {
    fn failure_class(&self) -> FailureClass {
        match self {
            JobError::NoJobArgument => FailureClass::Reject,
            JobError::Store(error) => error.failure_class(),
            JobError::Capture(error) => error.failure_class(),
            JobError::Rex(error) => error.failure_class(),
        }
    }

    fn is_usage(&self) -> bool {
        matches!(self, JobError::NoJobArgument)
    }

    fn exit_code(&self) -> u8 {
        match self {
            JobError::NoJobArgument => 2,
            JobError::Store(error) => error.exit_code(),
            JobError::Capture(error) => error.exit_code(),
            JobError::Rex(error) => error.exit_code(),
        }
    }
}

impl From<rusqlite::Error> for JobError {
    fn from(source: rusqlite::Error) -> Self {
        JobError::Store(source.into())
    }
}
