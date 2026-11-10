//! What language a source file is written in. `rex codebase` and `rex smells`
//! both ask the question here, so neither can drift from the other.

use std::path::Path;

use clap::ValueEnum;

use super::module_comment::{self, Syntax};

/// A language whose files rex knows by their extension. The names clap gives
/// the variants are the values of `rex codebase --only`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub(super) enum Language {
    Rust,
    Go,
    C,
    Cpp,
    Python,
    Js,
    Ts,
    Shell,
}

impl Language {
    /// The language of the file at `path`, told by its extension in any letter
    /// case. A `.h` file is C here, though C++ uses the extension too.
    pub(super) fn of(path: &Path) -> Option<Self> {
        Self::value_variants()
            .iter()
            .copied()
            .find(|language| language.has_extension(path))
    }

    /// Tells if `path` ends in one of this language's extensions, in any
    /// letter case. On macOS and Windows `Main.RS` and `main.rs` name the same
    /// file, so both are source.
    pub(super) fn has_extension(self, path: &Path) -> bool {
        path.extension().is_some_and(|extension| {
            self.extensions()
                .iter()
                .any(|known| extension.eq_ignore_ascii_case(known))
        })
    }

    fn extensions(self) -> &'static [&'static str] {
        match self {
            Self::Rust => &["rs"],
            Self::Go => &["go"],
            Self::C => &["c", "h"],
            Self::Cpp => &["cc", "cpp", "cxx", "h", "hh", "hpp", "hxx"],
            Self::Python => &["py"],
            Self::Js => &["js", "jsx", "mjs", "cjs"],
            Self::Ts => &["ts", "tsx", "mts", "cts"],
            Self::Shell => &["sh", "bash", "zsh"],
        }
    }

    /// How files in this language write the comment that opens them.
    pub(super) fn syntax(self) -> Syntax {
        match self {
            Self::Rust => module_comment::RUST,
            Self::Go => module_comment::SLASH,
            Self::C | Self::Cpp => module_comment::C,
            Self::Python => module_comment::PYTHON,
            Self::Js | Self::Ts => module_comment::SCRIPT,
            Self::Shell => module_comment::SHELL,
        }
    }
}
