//! What counts as a Rust file. `rex codebase --rust-only` and `rex smells`
//! both read the Rust files of a project, so both ask the question here and
//! neither can drift from the other.

use std::path::Path;

/// True when `path` ends in the Rust source extension, in any case. macOS and
/// Windows hand the compiler `Main.RS` when it asks for `main.rs`, so a file
/// named that way is Rust source and belongs in the answer.
pub(super) fn is_rust_file(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("rs"))
}
