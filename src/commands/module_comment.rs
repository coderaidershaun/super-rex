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

/// Lets any line come before the comment. Only `//!` uses it, because in
/// Rust and Zig that mark cannot be mistaken for the comment on the first item
/// of a file.
const ANY_LINE: &[&str] = &[""];

pub(super) const INNER_DOC: Syntax = Syntax {
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
    before: &["#pragma", "#if", "#define", "#undef", "#nullable"],
    ..SLASH
};
pub(super) const SCRIPT: Syntax = Syntax {
    before: &["\"use ", "'use "],
    ..SLASH
};
pub(super) const KOTLIN: Syntax = Syntax {
    before: &["@file:"],
    ..SLASH
};
pub(super) const PHP: Syntax = Syntax {
    before: &["<?php", "declare("],
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
pub(super) const RUBY: Syntax = Syntax {
    block: Some(("=begin", "=end")),
    ..PYTHON
};
/// Elixir keeps the text that says what a module is for in `@moduledoc`, the
/// first line inside `defmodule`, so that is read as well as `#` lines.
pub(super) const ELIXIR: Syntax = Syntax {
    block: Some(("@moduledoc \"\"\"", "\"\"\"")),
    before: &["defmodule "],
    ..PYTHON
};
pub(super) const SQL: Syntax = Syntax {
    line: "--",
    block: Some(("/*", "*/")),
    before: &[],
};
pub(super) const LUA: Syntax = Syntax {
    block: Some(("--[[", "]]")),
    before: &["#!"],
    ..SQL
};
pub(super) const HASKELL: Syntax = Syntax {
    block: Some(("{-", "-}")),
    ..SQL
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
    "@dart=",
    "swift-tools-version",
    "frozen_string_literal:",
    "encoding:",
    "coding:",
    "typed:",
    "+goose",
    "migrate:",
    "spdx-license-identifier",
    // The body of a Haskell pragma such as `{-# LANGUAGE GADTs #-}`.
    "# language",
    "# options_",
];

/// A comment whose first words are one of these is a licence header.
const LICENCE_STARTS: &[&str] = &[
    "copyright",
    "(c)",
    "©",
    "licensed",
    "this source code",
    "this file is part of",
];

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
    // The block mark is tried first, because Lua's `--[[` starts with its
    // line mark `--`. It can be indented, as Elixir's `@moduledoc` is.
    let (body, close) = if let Some(close) = *closer {
        (line, close)
    } else if let Some((open, close)) = syntax.block
        && let Some(body) = line.trim_start().strip_prefix(open)
    {
        (body, close)
    } else {
        return line.strip_prefix(syntax.line).map(|body| (body, false));
    };

    let inside = body.split_once(close).map(|(inside, _)| inside);
    *closer = inside.is_none().then_some(close);
    Some((inside.unwrap_or(body), inside.is_some()))
}

/// Drops the marks that decorate a comment line, such as the `*` that starts
/// each line of a block, the `|` of a Haskell doc comment or the `'` of an R
/// one. A line of marks alone, such as a rule, gives nothing.
fn plain_words(body: &str) -> &str {
    let words = body
        .trim_start_matches(['/', '*', '!', '#', '-', '|', '\''])
        .trim_start();
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
