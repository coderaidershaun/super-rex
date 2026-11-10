//! Reads the comment that opens a source file and says what the file is for.
//! `rex codebase --with-context` prints its first sentence beside the file.

/// How one family of languages writes the comment that opens a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Syntax {
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

pub(crate) const RUST: Syntax = Syntax {
    line: "//!",
    block: None,
    before: ANY_LINE,
};
pub(crate) const SLASH: Syntax = Syntax {
    line: "//",
    block: Some(("/*", "*/")),
    before: &[],
};
pub(crate) const C: Syntax = Syntax {
    before: &["#pragma", "#if", "#define"],
    ..SLASH
};
pub(crate) const SCRIPT: Syntax = Syntax {
    before: &["\"use ", "'use "],
    ..SLASH
};
pub(crate) const PYTHON: Syntax = Syntax {
    line: "#",
    block: Some(("\"\"\"", "\"\"\"")),
    before: &[],
};
pub(crate) const SHELL: Syntax = Syntax {
    block: None,
    ..PYTHON
};

/// The comment must start before this many `before` lines have gone by.
const LAST_DOC_START_LINE: usize = 5;

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

/// The first sentence of the comment that opens a file, if it has one. `head`
/// is the start of the file.
pub(crate) fn leading_sentence(head: &str, syntax: &Syntax) -> Option<String> {
    let mut text = String::new();
    let mut is_licence = false;
    let mut closer = None;
    let mut lines_before = 0;

    for line in head.lines() {
        let part = comment_part(line, syntax, &mut closer);

        if let Some((body, _)) = part {
            let body = body.trim();
            let words = plain_words(body);
            let has_started = is_licence || !text.is_empty();

            if words.is_empty() || (!has_started && starts_with_any(body, DIRECTIVE_STARTS)) {
                // An empty comment line ends the sentence, as a full stop does.
                if !text.is_empty() {
                    break;
                }
            } else if is_licence || (!has_started && starts_with_any(words, LICENCE_STARTS)) {
                is_licence = true;
            } else {
                if !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(words);
                if let Some(end) = sentence_end(&text) {
                    text.truncate(end);
                    break;
                }
            }
        }

        if part.is_none_or(|(_, is_last_line)| is_last_line) {
            // The comment has ended. A licence header is passed over, and the
            // search goes on to the comment after it.
            if !text.is_empty() {
                break;
            }
            is_licence = false;
        }

        if part.is_none() {
            let is_allowed = syntax.before.iter().any(|start| line.starts_with(start));
            lines_before += usize::from(is_allowed);
            let is_blank = line.trim().is_empty();
            if (!is_allowed && !is_blank) || lines_before >= LAST_DOC_START_LINE {
                break;
            }
        }
    }

    Some(text).filter(|sentence| !sentence.is_empty())
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

    fn sentence(head: &str, syntax: &Syntax) -> Option<String> {
        leading_sentence(head, syntax)
    }

    #[test]
    fn rust_joins_the_lines_up_to_the_first_full_stop() {
        let head = "//! Reads the feed\n//! and retries. It also logs.\nfn main() {}\n";
        assert_eq!(
            sentence(head, &RUST).as_deref(),
            Some("Reads the feed and retries.")
        );
    }

    #[test]
    fn rust_file_without_a_module_comment_gives_none() {
        assert_eq!(
            sentence("fn main() {}\n// Not a module comment.\n", &RUST),
            None
        );
    }

    #[test]
    fn rust_drops_the_heading_mark() {
        assert_eq!(
            sentence("//! # Title\n//! Body.\n", &RUST).as_deref(),
            Some("Title Body.")
        );
    }

    #[test]
    fn rust_skips_an_empty_comment_line_and_a_blank_line_before_the_text() {
        assert_eq!(
            sentence("//!\n\n//! Text.\n", &RUST).as_deref(),
            Some("Text.")
        );
    }

    #[test]
    fn rust_passes_over_a_licence_comment() {
        let head = "//! Copyright 2026 Acme.\n\n//! Reads the feed.\n";
        assert_eq!(sentence(head, &RUST).as_deref(), Some("Reads the feed."));
        assert_eq!(sentence("//! Licensed under MIT.\n", &RUST), None);
    }

    #[test]
    fn a_comment_must_start_before_the_fifth_line_before_it() {
        let comment = "// Reads the feed.\n";
        let four = format!("{}{comment}", "#define A\n".repeat(4));
        let five = format!("{}{comment}", "#define A\n".repeat(5));
        assert_eq!(sentence(&four, &C).as_deref(), Some("Reads the feed."));
        assert_eq!(sentence(&five, &C), None);

        let four = format!("{}//! Reads the feed.\n", "use a;\n".repeat(4));
        let five = format!("{}//! Reads the feed.\n", "use a;\n".repeat(5));
        assert_eq!(sentence(&four, &RUST).as_deref(), Some("Reads the feed."));
        assert_eq!(sentence(&five, &RUST), None);
    }

    #[test]
    fn a_block_that_closes_on_its_opening_line_ends_the_comment() {
        let head = "/* Reads the feed */\n// Not this one.\npackage feed\n";
        assert_eq!(sentence(head, &SLASH).as_deref(), Some("Reads the feed"));
        let head = "/* Reads the feed. */ int x;\n";
        assert_eq!(sentence(head, &C).as_deref(), Some("Reads the feed."));
    }

    #[test]
    fn a_block_that_never_closes_reads_to_the_end_of_the_head() {
        let head = "/*\n * Reads the feed\n * and more\n";
        assert_eq!(
            sentence(head, &SLASH).as_deref(),
            Some("Reads the feed and more")
        );
        assert_eq!(sentence("/*\n * Copyright 2026 Acme\n", &SLASH), None);
    }

    #[test]
    fn a_directive_is_ignored_in_any_letter_case() {
        for start in DIRECTIVE_STARTS {
            for start in [start.to_string(), start.to_uppercase()] {
                let head = format!("# {start}rest\n# Builds the archive.\n");
                assert_eq!(
                    sentence(&head, &SHELL).as_deref(),
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
                    sentence(&head, &SLASH).as_deref(),
                    Some("Reads the feed."),
                    "licence {start:?}"
                );
            }
        }
    }

    #[test]
    fn a_licence_and_a_comment_in_one_run_are_both_passed_over() {
        assert_eq!(
            sentence("// Copyright 2026 Acme.\n// Reads the feed.\n", &SLASH),
            None
        );
    }

    #[test]
    fn windows_line_endings_give_the_same_sentence() {
        assert_eq!(
            sentence("//! Reads the feed\r\n//! today.\r\n", &RUST).as_deref(),
            Some("Reads the feed today.")
        );
        let head = "/*\r\n * Reads the feed\r\n * today.\r\n */\r\n";
        assert_eq!(
            sentence(head, &SLASH).as_deref(),
            Some("Reads the feed today.")
        );
    }

    #[test]
    fn an_empty_comment_line_ends_the_sentence_once_text_has_started() {
        assert_eq!(
            sentence("// Draws the chart\n//\n// More.\n", &SLASH).as_deref(),
            Some("Draws the chart")
        );
    }

    #[test]
    fn decoration_and_rule_lines_are_dropped() {
        let boxed = "/*****\n * Draws the chart.\n *****/\n";
        assert_eq!(sentence(boxed, &C).as_deref(), Some("Draws the chart."));
        let rule = "// ======\n// Draws the chart.\n";
        assert_eq!(sentence(rule, &SLASH).as_deref(), Some("Draws the chart."));
        assert_eq!(
            sentence("/// Draws the chart.\n", &C).as_deref(),
            Some("Draws the chart.")
        );
    }

    #[test]
    fn each_language_gives_the_sentence_of_its_opening_comment() {
        let cases = [
            (
                "go",
                SLASH,
                "// Package feed reads the price feed.\npackage feed\n",
                "Package feed reads the price feed.",
            ),
            (
                "go block",
                SLASH,
                "/* Package feed reads the feed. */\npackage feed\n",
                "Package feed reads the feed.",
            ),
            (
                "go licence",
                SLASH,
                "// Copyright 2026 Acme.\n\n// Reads the feed.\n\npackage feed\n",
                "Reads the feed.",
            ),
            (
                "c guard",
                C,
                "#ifndef FEED_H\n#define FEED_H\n// Declares the feed reader.\n#include <x.h>\n",
                "Declares the feed reader.",
            ),
            (
                "c above guard",
                C,
                "/* Declares the feed reader. */\n#ifndef FEED_H\n",
                "Declares the feed reader.",
            ),
            (
                "c licence",
                C,
                "/*\n * Licensed under MIT.\n */\n#pragma once\n// Declares the feed reader.\n",
                "Declares the feed reader.",
            ),
            (
                "js",
                SCRIPT,
                "\"use strict\";\n// Draws the order book chart.\nimport a from 'a';\n",
                "Draws the order book chart.",
            ),
            (
                "ts",
                SCRIPT,
                "/** Draws the order book chart. */\nimport a from 'a';\n",
                "Draws the order book chart.",
            ),
            (
                "python one line",
                PYTHON,
                "#!/usr/bin/env python3\n\"\"\"Loads the run config.\"\"\"\nimport os\n",
                "Loads the run config.",
            ),
            (
                "python opened alone",
                PYTHON,
                "\"\"\"\nLoads the run config.\n\"\"\"\n",
                "Loads the run config.",
            ),
            (
                "python many lines",
                PYTHON,
                "# -*- coding: utf-8 -*-\n\"\"\"Loads the run\nconfig from disk. More.\"\"\"\n",
                "Loads the run config from disk.",
            ),
            (
                "shell",
                SHELL,
                "#!/bin/bash\n# Builds the release archive.\nset -e\n",
                "Builds the release archive.",
            ),
        ];
        for (name, syntax, head, expected) in cases {
            assert_eq!(sentence(head, &syntax).as_deref(), Some(expected), "{name}");
        }
    }

    #[test]
    fn code_before_the_comment_gives_none() {
        let cases = [
            ("go", SLASH, "package feed\n// Reads the feed.\n"),
            ("c", C, "#include <x.h>\n// Reads the feed.\n"),
            ("js", SCRIPT, "import a from 'a';\n// Reads the feed.\n"),
            ("python", PYTHON, "import os\n\"\"\"Reads the feed.\"\"\"\n"),
            ("shell", SHELL, "set -e\n# Reads the feed.\n"),
            ("empty", SLASH, ""),
        ];
        for (name, syntax, head) in cases {
            assert_eq!(sentence(head, &syntax), None, "{name}");
        }
    }
}
