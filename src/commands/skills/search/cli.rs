//! `rex skills search`: reads the arguments of the `search` tool as JSON on
//! stdin and prints the answer as one JSON line, for tests and gate A runs.

use std::io::{self, Read};
use std::path::PathBuf;

use serde_json::Value;

use super::answer::search_with;
use super::index::{Index, IndexVersion};
use super::keyword::KeywordSettings;
use super::request::SearchRequest;
use super::response::SearchError;
use crate::commands::skills::SkillsError;
use crate::commands::skills::serve::library::Libraries;
use crate::commands::stdout::print_line_to_stdout;

/// Search the library in `libraries` with the arguments on stdin. `settings`
/// holds `--keyword-ratio` and `--keyword-scores`.
pub(crate) fn run(folders: &[PathBuf], settings: &KeywordSettings) -> Result<(), SkillsError> {
    let mut arguments = String::new();
    io::stdin()
        .read_to_string(&mut arguments)
        .map_err(SearchError::Stdin)?;
    let bad_arguments = |error: serde_json::Error| SearchError::BadArguments {
        message: error.to_string(),
    };
    // Through a `Value`, as the serve tool reads its arguments, so that both
    // word a bad field the same way.
    let arguments: Value = serde_json::from_str(&arguments).map_err(bad_arguments)?;
    let request: SearchRequest = serde_json::from_value(arguments).map_err(bad_arguments)?;
    let libraries = Libraries::open(None, folders).map_err(SearchError::from)?;
    let index = Index::load(&libraries)?;
    let vocabulary = libraries.vocabulary_version().map_err(SearchError::from)?;
    let version = IndexVersion::of(&index, vocabulary.as_deref());
    let response = search_with(&index, &version, &request, settings)?;
    let line = serde_json::to_string(&response).map_err(|source| SkillsError::Json { source })?;
    print_line_to_stdout(&line)?;
    Ok(())
}
