//! Reads the comment that opens a source file and says what the file is for.
//! `rex codebase --with-context` prints its first sentence beside the file.

/// How one family of languages writes the comment that opens a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Syntax {
    /// Opens a comment that runs to the end of its line.
    line: &'static str,
    /// Opens and closes a comment that can span lines.
    block: Option<(&'static str, &'static str)>,
    /// Starts of the code lines that may come before the comment.
    before: &'static [&'static str],
}

/// Lets any line come before the comment. Only Rust uses it, because `//!`
/// cannot be mistaken for the comment on the first item of a file.
const ANY_LINE: &[&str] = &[""];

pub(super) const RUST: Syntax = Syntax {
    line: "//!",
    block: None,
    before: ANY_LINE,
};
pub(super) const SLASH: Syntax = Syntax {
    line: "//",
    block: Some(("/*", "*/")),
    before: &[],
};
pub(super) const C: Syntax = Syntax {
    before: &["#pragma", "#if", "#define"],
    ..SLASH
};
pub(super) const SCRIPT: Syntax = Syntax {
    before: &["\"use ", "'use "],
    ..SLASH
};
pub(super) const PYTHON: Syntax = Syntax {
    line: "#",
    block: Some(("\"\"\"", "\"\"\"")),
    before: &[],
};
pub(super) const SHELL: Syntax = Syntax {
    block: None,
    ..PYTHON
};

/// How many of the code lines that `before` allows may come before the comment.
const MAX_LINES_BEFORE: usize = 4;

/// A comment line that starts with one of these is an instruction to a tool.
const DIRECTIVE_STARTS: &[&str] = &[
    "!/",
    "! /",
    "go:",
    "+build",
    "-*-",
    "vim:",
    "shellcheck ",
    "eslint-",
    "@ts-",
    "spdx-license-identifier",
];

/// A comment whose first words are one of these is a licence header.
const LICENCE_STARTS: &[&str] = &["copyright", "(c)", "©", "licensed", "this source code"];

// SMELL: a file that starts with a byte order mark gets no sentence, and nor
// does a JavaScript or TypeScript file that starts with a `#!` line.
/// The first sentence of the comment that opens a file, if it has one. `head`
/// is the start of the file.
pub(super) fn leading_sentence(head: &str, syntax: &Syntax) -> Option<String> {
    let mut sentence = String::new();
    let mut is_licence = false;
    let mut closer = None;
    let mut lines_before = 0;

    for line in head.lines() {
        let part = comment_part(line, syntax, &mut closer);

        if let Some((body, _)) = part {
            let body = body.trim();
            let words = plain_words(body);
            let has_started = is_licence || !sentence.is_empty();

            if words.is_empty() || (!has_started && starts_with_any(body, DIRECTIVE_STARTS)) {
                // An empty comment line after some words ends the sentence, as
                // a full stop does.
                if !sentence.is_empty() {
                    break;
                }
            } else if is_licence || (!has_started && starts_with_any(words, LICENCE_STARTS)) {
                is_licence = true;
            } else if push_words(&mut sentence, words) {
                break;
            }
        }

        if part.is_none_or(|(_, is_last_line)| is_last_line) {
            // The comment has ended. A licence header is passed over, and the
            // search goes on to the comment after it.
            if !sentence.is_empty() {
                break;
            }
            is_licence = false;
        }

        if part.is_none() {
            let is_allowed = syntax.before.iter().any(|start| line.starts_with(start));
            lines_before += usize::from(is_allowed);
            let is_blank = line.trim().is_empty();
            if (!is_allowed && !is_blank) || lines_before > MAX_LINES_BEFORE {
                break;
            }
        }
    }

    (!sentence.is_empty()).then_some(sentence)
}

/// Adds `words` to the end of `sentence`, cut at its first full stop, and
/// tells if the sentence is now complete.
fn push_words(sentence: &mut String, words: &str) -> bool {
    if !sentence.is_empty() {
        sentence.push(' ');
    }
    sentence.push_str(words);

    let end = sentence_end(sentence);
    if let Some(end) = end {
        sentence.truncate(end);
    }
    end.is_some()
}

/// The part of `line` that is inside a comment, if the line starts in one,
/// and whether a block comment ends on it. `closer` holds the mark that ends
/// the block comment the next line starts in.
fn comment_part<'a>(
    line: &'a str,
    syntax: &Syntax,
    closer: &mut Option<&'static str>,
) -> Option<(&'a str, bool)> {
    let (body, close) = if let Some(close) = *closer {
        (line, close)
    } else if let Some(body) = line.strip_prefix(syntax.line) {
        return Some((body, false));
    } else {
        let (open, close) = syntax.block?;
        (line.strip_prefix(open)?, close)
    };

    let inside = body.split_once(close).map(|(inside, _)| inside);
    *closer = inside.is_none().then_some(close);
    Some((inside.unwrap_or(body), inside.is_some()))
}

/// Drops the marks that decorate a comment line, such as the `*` that starts
/// each line of a block. A line of marks alone, such as a rule, gives nothing.
fn plain_words(body: &str) -> &str {
    let words = body.trim_start_matches(['/', '*', '!', '#']).trim_start();
    if words.chars().any(char::is_alphanumeric) {
        words
    } else {
        ""
    }
}

/// Tells if `text` starts with one of `starts`, in any letter case.
fn starts_with_any(text: &str, starts: &[&str]) -> bool {
    starts.iter().any(|start| {
        text.get(..start.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(start))
    })
}

/// Finds the byte offset just past the first full stop that is followed by
/// whitespace or the end of the text. A full stop inside a word, such as
/// `CODEBASE.md`, does not count.
fn sentence_end(text: &str) -> Option<usize> {
    text.char_indices()
        .filter(|&(_, c)| c == '.')
        .map(|(index, _)| index + 1)
        .find(|&end| {
            text.get(end..)
                .is_some_and(|rest| rest.chars().next().is_none_or(char::is_whitespace))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_drops_the_heading_mark() {
        assert_eq!(
            leading_sentence("//! # Title\n//! Body.\n", &RUST).as_deref(),
            Some("Title Body.")
        );
    }

    #[test]
    fn rust_skips_an_empty_comment_line_and_a_blank_line_before_the_text() {
        assert_eq!(
            leading_sentence("//!\n\n//! Text.\n", &RUST).as_deref(),
            Some("Text.")
        );
    }

    #[test]
    fn rust_passes_over_a_licence_comment() {
        let head = "//! Copyright 2026 Acme.\n\n//! Reads the feed.\n";
        assert_eq!(
            leading_sentence(head, &RUST).as_deref(),
            Some("Reads the feed.")
        );
        assert_eq!(leading_sentence("//! Licensed under MIT.\n", &RUST), None);
    }

    // Outside Rust a blank line does not count toward the limit.
    #[test]
    fn at_most_four_allowed_lines_may_come_before_the_comment() {
        let comment = "\n// Reads the feed.\n";
        let four = format!("{}{comment}", "#define A\n".repeat(4));
        let five = format!("{}{comment}", "#define A\n".repeat(5));
        assert_eq!(
            leading_sentence(&four, &C).as_deref(),
            Some("Reads the feed.")
        );
        assert_eq!(leading_sentence(&five, &C), None);
    }

    #[test]
    fn a_block_that_closes_on_its_opening_line_ends_the_comment() {
        let head = "/* Reads the feed */\n// Not this one.\npackage feed\n";
        assert_eq!(
            leading_sentence(head, &SLASH).as_deref(),
            Some("Reads the feed")
        );
        let head = "/* Reads the feed. */ int x;\n";
        assert_eq!(
            leading_sentence(head, &C).as_deref(),
            Some("Reads the feed.")
        );
    }

    #[test]
    fn a_block_that_never_closes_reads_to_the_end_of_the_head() {
        let head = "/*\n * Reads the feed\n * and more\n";
        assert_eq!(
            leading_sentence(head, &SLASH).as_deref(),
            Some("Reads the feed and more")
        );
        assert_eq!(
            leading_sentence("/*\n * Copyright 2026 Acme\n", &SLASH),
            None
        );
    }

    #[test]
    fn a_directive_is_ignored_in_any_letter_case() {
        for start in DIRECTIVE_STARTS {
            for start in [start.to_string(), start.to_uppercase()] {
                let head = format!("# {start}rest\n# Builds the archive.\n");
                assert_eq!(
                    leading_sentence(&head, &SHELL).as_deref(),
                    Some("Builds the archive."),
                    "directive {start:?}"
                );
            }
        }
    }

    #[test]
    fn a_licence_opening_is_passed_over_in_any_letter_case() {
        for start in LICENCE_STARTS {
            for start in [start.to_string(), start.to_uppercase()] {
                let head = format!("// {start} 2026 Acme\n\n// Reads the feed.\n");
                assert_eq!(
                    leading_sentence(&head, &SLASH).as_deref(),
                    Some("Reads the feed."),
                    "licence {start:?}"
                );
            }
        }
    }

    #[test]
    fn windows_line_endings_give_the_same_sentence() {
        assert_eq!(
            leading_sentence("//! Reads the feed\r\n//! today.\r\n", &RUST).as_deref(),
            Some("Reads the feed today.")
        );
        let head = "/*\r\n * Reads the feed\r\n * today.\r\n */\r\n";
        assert_eq!(
            leading_sentence(head, &SLASH).as_deref(),
            Some("Reads the feed today.")
        );
    }
}
