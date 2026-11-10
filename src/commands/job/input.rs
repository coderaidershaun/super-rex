//! What the job commands read besides their flags: input files, where `-`
//! stands for standard input, and the session's environment variables.

use std::io::{self, Read};
use std::path::Path;

/// The text of `path`, relative to `cwd`, or of standard input when `path` is `-`.
pub(super) fn read_text(cwd: &Path, path: &Path) -> io::Result<String> {
    if path == Path::new("-") {
        let mut text = String::new();
        io::stdin().read_to_string(&mut text)?;
        return Ok(text);
    }
    std::fs::read_to_string(cwd.join(path))
}

/// An environment variable, where an empty value counts as not set.
pub(super) fn non_empty_var(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}
