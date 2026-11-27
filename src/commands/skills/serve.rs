//! `rex skills serve`: a skill library server that speaks JSON-RPC over stdin and stdout, so any MCP client can ask what a library holds.

mod library;
mod rpc;
mod tools;
mod vocabulary;

use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use library::Libraries;

/// Failure of `rex skills serve`.
#[derive(thiserror::Error, Debug)]
pub enum ServeError {
    #[error("library folder {} does not exist or is not a folder", path.display())]
    NoLibrary { path: PathBuf },

    #[error("cannot read the vocabulary file {}", path.display())]
    VocabularyRead {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("vocabulary file {} is not valid: {reason}", path.display())]
    VocabularyInvalid { path: PathBuf, reason: String },

    #[error("cannot read the requests on stdin")]
    Stdin(#[source] io::Error),

    #[error("cannot write the answer on stdout")]
    Stdout(#[source] io::Error),
}

/// Serve `folders` (the embedded library when there are none) until stdin ends.
/// `head` reports `name`, or the first folder's base name when it is `None`.
///
/// # Errors
/// - [`ServeError::NoLibrary`] if a folder is missing or is not a folder.
/// - [`ServeError::VocabularyRead`] or [`ServeError::VocabularyInvalid`] if
///   `vocab/terms.json` beside the first folder exists but gives no version.
/// - [`ServeError::Stdin`] or [`ServeError::Stdout`] if the pipes break.
pub fn run(name: Option<String>, folders: &[PathBuf]) -> Result<(), ServeError> {
    let libraries = Libraries::open(name, folders)?;
    // A broken vocabulary file would otherwise show only on the first `head`.
    libraries.vocabulary_version()?;
    serve(&libraries, io::stdin().lock(), io::stdout().lock())
}

fn serve(
    libraries: &Libraries,
    input: impl BufRead,
    mut output: impl Write,
) -> Result<(), ServeError> {
    for line in input.lines() {
        let line = line.map_err(ServeError::Stdin)?;
        if let Some(answer) = rpc::answer(libraries, &line) {
            writeln!(output, "{answer}")
                .and_then(|()| output.flush())
                .map_err(ServeError::Stdout)?;
        }
    }
    Ok(())
}
