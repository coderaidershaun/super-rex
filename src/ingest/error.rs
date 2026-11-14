//! Why `rex ingest` refused a batch or could not store it. The plugin branches on
//! the exit code and reads the line, so each refusal text is fixed.

use std::path::PathBuf;

use crate::error::{CommandError, FailureClass, RexError};
use crate::store::StoreError;

/// A batch that was refused or could not be stored. A refused batch changes nothing.
#[derive(thiserror::Error, Debug)]
pub enum IngestError {
    /// The batch is not of the form of the batch schema.
    #[error("ingest refused: {reason}")]
    Malformed { reason: String },

    #[error("ingest refused: gap in seq: expected {expected}, got {got}")]
    Gap { expected: u64, got: u64 },

    /// The store holds another event under this sequence number.
    #[error("ingest refused: conflict at seq {seq}")]
    Conflict { seq: u64 },

    /// The event is of the right form but does not fit the record, such as a
    /// plan for a job that was never opened.
    #[error("ingest refused: seq {seq} {kind}: {reason}")]
    DoesNotFit {
        seq: u64,
        kind: &'static str,
        reason: String,
    },

    #[error("ingest refused: wrong secret for session {session} stream {stream}")]
    WrongSecret { session: String, stream: String },

    #[error("ingest refused: storage is off in .rex/config.json")]
    StorageOff,

    #[error("ingest refused: no store at {}; run rex init --with-storage", path.display())]
    NoStore { path: PathBuf },

    #[error("ingest failed: .rex/config.json of {} cannot be read", root.display())]
    ConfigUnreadable {
        root: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("ingest failed: the batch on standard input cannot be read")]
    Read {
        #[source]
        source: std::io::Error,
    },

    #[error(transparent)]
    Store(#[from] StoreError),

    #[error(transparent)]
    Rex(#[from] RexError),
}

impl CommandError for IngestError {
    /// What the plugin does next: it halts its sending on a conflict, with no
    /// store, or when storage is off; makes a new stream after a wrong secret;
    /// sends again after a failure of the store; and halts on a refused batch,
    /// whose events it cannot change.
    fn failure_class(&self) -> FailureClass {
        match self {
            IngestError::Malformed { .. }
            | IngestError::Gap { .. }
            | IngestError::DoesNotFit { .. } => FailureClass::Reject,
            IngestError::Conflict { .. }
            | IngestError::StorageOff
            | IngestError::NoStore { .. }
            | IngestError::ConfigUnreadable { .. } => FailureClass::Halt,
            IngestError::WrongSecret { .. } => FailureClass::Reset,
            IngestError::Read { .. } | IngestError::Rex(_) => FailureClass::Retry,
            IngestError::Store(error) => error.failure_class(),
        }
    }

    /// 2 for a batch that does not fit, 3 for a wrong secret, 4 when storage is
    /// off or there is no store, else the code of the store's error.
    fn exit_code(&self) -> u8 {
        match self {
            IngestError::Malformed { .. }
            | IngestError::Gap { .. }
            | IngestError::Conflict { .. }
            | IngestError::DoesNotFit { .. } => 2,
            IngestError::WrongSecret { .. } => 3,
            IngestError::StorageOff | IngestError::NoStore { .. } => 4,
            IngestError::ConfigUnreadable { .. } | IngestError::Read { .. } => 1,
            IngestError::Store(error) => error.exit_code(),
            IngestError::Rex(error) => error.exit_code(),
        }
    }

    fn is_bare_line(&self) -> bool {
        match self {
            IngestError::Malformed { .. }
            | IngestError::Gap { .. }
            | IngestError::Conflict { .. }
            | IngestError::DoesNotFit { .. }
            | IngestError::WrongSecret { .. }
            | IngestError::StorageOff
            | IngestError::NoStore { .. } => true,
            IngestError::ConfigUnreadable { .. } | IngestError::Read { .. } => false,
            IngestError::Store(error) => error.is_bare_line(),
            IngestError::Rex(error) => error.is_bare_line(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The plugin acts on the exit code; each code must stand for one class, so
    // a refusal it must halt on is never sent again as if it were a failure.
    #[test]
    fn each_refusal_has_the_exit_code_of_its_class() {
        let cases = [
            (IngestError::Conflict { seq: 7 }, 2, FailureClass::Halt),
            (
                IngestError::Gap {
                    expected: 5,
                    got: 7,
                },
                2,
                FailureClass::Reject,
            ),
            (
                IngestError::WrongSecret {
                    session: "s".into(),
                    stream: "aaaaaaaaaaaaaaaa".into(),
                },
                3,
                FailureClass::Reset,
            ),
            (IngestError::StorageOff, 4, FailureClass::Halt),
            (
                IngestError::NoStore {
                    path: PathBuf::from("/r/.rex/rex.db"),
                },
                4,
                FailureClass::Halt,
            ),
        ];
        for (error, code, class) in cases {
            assert_eq!(error.exit_code(), code, "{error}");
            assert_eq!(error.failure_class(), class, "{error}");
            assert!(error.is_bare_line(), "{error}");
        }
    }
}
