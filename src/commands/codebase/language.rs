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
    Java,
    Kotlin,
    Scala,
    #[value(name = "csharp")]
    CSharp,
    Swift,
    #[value(name = "objc")]
    ObjectiveC,
    Dart,
    Php,
    Ruby,
    Lua,
    Haskell,
    Elixir,
    R,
    Sql,
    Zig,
    Perl,
}

impl Language {
    /// The language of the file at `path`, told by its extension in any letter
    /// case. A `.h` file is C here, though C++ and Objective-C use it too. An
    /// extension that two languages share goes to the one that uses it most:
    /// `.m` is Objective-C, not MATLAB, and `.pl` is Perl, not Prolog.
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
            Self::Java => &["java"],
            Self::Kotlin => &["kt", "kts"],
            Self::Scala => &["scala", "sc"],
            Self::CSharp => &["cs"],
            Self::Swift => &["swift"],
            Self::ObjectiveC => &["m", "mm", "h"],
            Self::Dart => &["dart"],
            Self::Php => &["php"],
            Self::Ruby => &["rb", "rake"],
            Self::Lua => &["lua"],
            Self::Haskell => &["hs"],
            Self::Elixir => &["ex", "exs"],
            Self::R => &["r"],
            Self::Sql => &["sql"],
            Self::Zig => &["zig"],
            Self::Perl => &["pl", "pm"],
        }
    }

    /// How files in this language write the comment that opens them.
    pub(super) fn syntax(self) -> Syntax {
        match self {
            Self::Rust | Self::Zig => module_comment::INNER_DOC,
            Self::Go | Self::Java | Self::Scala | Self::Swift | Self::Dart => module_comment::SLASH,
            Self::C | Self::Cpp | Self::ObjectiveC | Self::CSharp => module_comment::C,
            Self::Js | Self::Ts => module_comment::SCRIPT,
            Self::Kotlin => module_comment::KOTLIN,
            Self::Php => module_comment::PHP,
            Self::Python => module_comment::PYTHON,
            Self::Shell | Self::Perl | Self::R => module_comment::SHELL,
            Self::Ruby => module_comment::RUBY,
            Self::Elixir => module_comment::ELIXIR,
            Self::Sql => module_comment::SQL,
            Self::Lua => module_comment::LUA,
            Self::Haskell => module_comment::HASKELL,
        }
    }
}
