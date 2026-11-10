//! Lists every `SMELL` flag comment in the Rust files of the working
//! directory, so the known problems left in the code can all be found again.

use std::fs;
use std::path::Path;

use super::comments::comment_lines;
use super::language::Language;
use super::stdout::print_to_stdout;
use super::walk::{skip_if_denied, visible_entries};
use crate::error::RexError;

const FLAG_WORD: &[u8] = b"SMELL";

/// Arguments of `rex smells`.
#[derive(clap::Args)]
pub struct SmellsOptions {}

/// Walk `cwd`, skip dotfiles and gitignored paths, and print one `path:line`
/// for each `SMELL` flag comment in a Rust file, then the count of them all.
///
/// # Errors
/// - [`RexError::Walk`] if the directory walk fails (loop, glob). A path that
///   cannot be read is skipped with a warning on stderr.
/// - [`RexError::Io`] if a Rust file cannot be read for a reason other than its
///   permissions. A file that is denied is skipped with a warning on stderr.
/// - [`RexError::Stdout`] if the report cannot be written to stdout.
pub fn run(cwd: &Path, _options: SmellsOptions) -> Result<(), RexError> {
    print_to_stdout(&render_report(cwd)?)
}

fn render_report(cwd: &Path) -> Result<String, RexError> {
    let mut flags: Vec<String> = Vec::new();

    for entry in visible_entries(cwd) {
        let entry = entry?;
        let path = entry.path();

        let is_rust_source = entry.file_type().is_some_and(|ft| ft.is_file())
            && Language::of(path) == Some(Language::Rust);
        if !is_rust_source {
            continue;
        }

        // The file is read as bytes because text that is not valid UTF-8 must
        // not fail the whole command.
        let Some(source) = skip_if_denied(path, fs::read(path))? else {
            continue;
        };
        let shown_path = path.strip_prefix(cwd).unwrap_or(path).display();

        let lines = flag_lines(&source).into_iter();
        flags.extend(lines.map(|line| format!("{shown_path}:{line}\n")));
    }

    let total = format!("Code smells: {}\n", flags.len());
    Ok(flags.concat() + &total)
}

/// The line number of each `SMELL` flag comment in `source`, counted from 1.
/// A line is listed once, even when it holds more than one flag.
fn flag_lines(source: &[u8]) -> Vec<usize> {
    let mut lines: Vec<usize> = comment_lines(source)
        .into_iter()
        .filter(|comment| is_flag(comment.text))
        .map(|comment| comment.line)
        .collect();
    lines.dedup();
    lines
}

/// Tells if the text of a comment starts with the flag word.
///
/// A comment that only talks about the word further in is not a flag, and
/// neither is a longer word that starts the same way, such as `SMELLY`.
fn is_flag(comment: &[u8]) -> bool {
    // Doc comments open with `///` or `//!`, and the lines of a block comment
    // often open with `*`, so those marks are skipped along with the spaces.
    let text_start = comment
        .iter()
        .position(|byte| !matches!(byte, b'/' | b'!' | b'*') && !byte.is_ascii_whitespace())
        .unwrap_or(comment.len());

    comment[text_start..]
        .strip_prefix(FLAG_WORD)
        .is_some_and(|rest| {
            rest.first()
                .is_none_or(|byte| !byte.is_ascii_alphanumeric() && *byte != b'_')
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_flagged(source: &str) -> bool {
        !flag_lines(source.as_bytes()).is_empty()
    }

    #[test]
    fn a_comment_that_starts_with_the_flag_word_is_a_flag() {
        let flagged = [
            "// SMELL: bad name",
            "    // SMELL: indented",
            "//SMELL: no space",
            "// SMELL",
            "// SMELL(naming): tagged",
            "/// SMELL: on a doc comment",
            "//! SMELL: on a module doc comment",
            "let total = 1; // SMELL: magic number",
            "let url = \"http://example.com\"; // SMELL: fixed address",
            "let quote = \"say \\\"hi\\\"\"; // SMELL: escaped quotes before it",
            "// SMELL: windows line ending\r",
            "/* SMELL: block comment */",
            "/** SMELL: block doc comment */",
        ];

        for source in flagged {
            assert!(is_flagged(source), "expected a flag in: {source}");
        }
    }

    #[test]
    fn text_that_is_not_a_flag_comment_is_ignored() {
        let clean = [
            "",
            "fn main() {}",
            "// a plain comment",
            "// This removes the SMELL: flag from the list.",
            "// SMELLY code lives here",
            "// SMELL_COUNT is the total",
            "// smell: lower case",
            "const FLAG: &str = \"SMELL:\";",
            "let fixture = \"// SMELL: inside a string\";",
        ];

        for source in clean {
            assert!(!is_flagged(source), "expected no flag in: {source}");
        }
    }

    #[test]
    fn a_line_with_two_flags_is_listed_once() {
        assert_eq!(
            flag_lines(b"/* SMELL: one */ /* SMELL: two */ // SMELL: three\n"),
            vec![1]
        );
    }

    #[test]
    fn only_the_lines_of_a_block_comment_that_start_with_the_flag_word_count() {
        let source = b"/*\n * SMELL: this line\n * mentions SMELL: in the middle\n */\n";

        assert_eq!(flag_lines(source), vec![2]);
    }
}
