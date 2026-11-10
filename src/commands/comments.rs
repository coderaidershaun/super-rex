//! Finds the comments in Rust source text. String and character literals are
//! skipped, so a `//` inside one is not taken for a comment.

/// The text of one line of a comment, after the marker that opens it.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct CommentLine<'a> {
    /// Line in the source, counted from 1.
    pub(super) line: usize,
    pub(super) text: &'a [u8],
}

/// List every comment line in `source`, in order. A `//` comment gives one
/// line. A `/* */` comment gives one line for each line it spans, and a nested
/// `/* */` stays part of the outer comment.
///
/// The source is read as bytes, so text that is not valid UTF-8 is fine.
pub(super) fn comment_lines(source: &[u8]) -> Vec<CommentLine<'_>> {
    Scanner {
        source,
        index: 0,
        line: 1,
        comments: Vec::new(),
    }
    .scan()
}

struct Scanner<'a> {
    source: &'a [u8],
    index: usize,
    line: usize,
    comments: Vec<CommentLine<'a>>,
}

impl<'a> Scanner<'a> {
    fn scan(mut self) -> Vec<CommentLine<'a>> {
        let source = self.source;

        while let Some(&byte) = source.get(self.index) {
            let rest = &source[self.index..];
            if rest.starts_with(b"//") {
                self.line_comment();
            } else if rest.starts_with(b"/*") {
                self.block_comment();
            } else if byte == b'"' {
                self.skip_to(string_end(source, self.index));
            } else if let Some(end) = raw_string_end(source, self.index) {
                self.skip_to(end);
            } else if byte == b'\'' {
                self.skip_to(char_literal_end(source, self.index));
            } else {
                self.skip_to(self.index + 1);
            }
        }

        self.comments
    }

    /// Moves to `end` and counts the lines passed on the way.
    fn skip_to(&mut self, end: usize) {
        let passed = &self.source[self.index..end];
        self.line += passed.iter().filter(|&&byte| byte == b'\n').count();
        self.index = end;
    }

    fn line_comment(&mut self) {
        let source = self.source;
        let start = self.index + 2;
        let end = source[start..]
            .iter()
            .position(|&byte| byte == b'\n')
            .map_or(source.len(), |offset| start + offset);

        self.comments.push(CommentLine {
            line: self.line,
            text: &source[start..end],
        });
        self.index = end;
    }

    fn block_comment(&mut self) {
        let source = self.source;
        let mut depth = 1;
        self.index += 2;
        let mut text_start = self.index;

        while let Some(&byte) = source.get(self.index) {
            let rest = &source[self.index..];
            if rest.starts_with(b"*/") {
                depth -= 1;
                if depth == 0 {
                    self.push_block_line(text_start);
                    self.index += 2;
                    return;
                }
                self.index += 2;
            } else if rest.starts_with(b"/*") {
                depth += 1;
                self.index += 2;
            } else if byte == b'\n' {
                self.push_block_line(text_start);
                self.line += 1;
                self.index += 1;
                text_start = self.index;
            } else {
                self.index += 1;
            }
        }
        self.push_block_line(text_start);
    }

    /// Records the text from `text_start` up to the current position as one
    /// line of the block comment.
    fn push_block_line(&mut self, text_start: usize) {
        let source = self.source;
        self.comments.push(CommentLine {
            line: self.line,
            text: &source[text_start..self.index],
        });
    }
}

/// Index just past the string literal whose opening quote is at `quote`.
fn string_end(source: &[u8], quote: usize) -> usize {
    let mut index = quote + 1;
    while let Some(&byte) = source.get(index) {
        match byte {
            // The byte after a backslash is part of the string, even a quote.
            b'\\' => index += 2,
            b'"' => return index + 1,
            _ => index += 1,
        }
    }
    source.len()
}

/// Index just past the raw string (`r"…"`, `r#"…"#`, `br"…"`) that starts with
/// the `r` at `r`, or `None` when no raw string starts there.
fn raw_string_end(source: &[u8], r: usize) -> Option<usize> {
    if source[r] != b'r' || !starts_a_token(source, r) {
        return None;
    }

    let hashes = source[r + 1..]
        .iter()
        .take_while(|&&byte| byte == b'#')
        .count();
    let quote = r + 1 + hashes;
    if source.get(quote) != Some(&b'"') {
        return None;
    }

    let closes = |quote_index: usize| {
        let after = &source[quote_index + 1..];
        after.len() >= hashes && after[..hashes].iter().all(|&byte| byte == b'#')
    };
    let closing_quote =
        (quote + 1..source.len()).find(|&index| source[index] == b'"' && closes(index));
    Some(closing_quote.map_or(source.len(), |index| index + 1 + hashes))
}

/// Tells if the `r` at `index` can start a raw string. Inside a longer word
/// (`bar`) it cannot, but after a `b` or `c` prefix that starts its own token
/// (`br"…"`) it can.
fn starts_a_token(source: &[u8], index: usize) -> bool {
    let is_word_byte = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'_';

    match index.checked_sub(1).map(|before| source[before]) {
        None => true,
        Some(before) if !is_word_byte(before) => true,
        Some(b'b' | b'c') => index
            .checked_sub(2)
            .is_none_or(|earlier| !is_word_byte(source[earlier])),
        Some(_) => false,
    }
}

/// Index just past the character literal that starts with the quote at
/// `quote`. A quote that starts a lifetime or a loop label (`'a`) is not a
/// literal, so the index just past that quote is returned.
fn char_literal_end(source: &[u8], quote: usize) -> usize {
    let after = quote + 1;

    match source.get(after) {
        Some(b'\\') => source
            .get(after + 2..)
            .and_then(|rest| rest.iter().position(|&byte| byte == b'\''))
            .map_or(after, |offset| after + 2 + offset + 1),
        Some(&first) => {
            let width = utf8_width(first);
            if source.get(after + width) == Some(&b'\'') {
                after + width + 1
            } else {
                after
            }
        }
        None => after,
    }
}

/// Bytes in the UTF-8 character that starts with `first`.
const fn utf8_width(first: u8) -> usize {
    match first {
        0xF0..=0xFF => 4,
        0xE0..=0xEF => 3,
        0xC0..=0xDF => 2,
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(source: &str) -> Vec<(usize, String)> {
        comment_lines(source.as_bytes())
            .into_iter()
            .map(|comment| {
                (
                    comment.line,
                    String::from_utf8_lossy(comment.text).into_owned(),
                )
            })
            .collect()
    }

    #[test]
    fn a_line_comment_gives_its_line_number_and_text() {
        assert_eq!(
            texts("fn a() {}\nlet x = 1; // note\n"),
            vec![(2, " note".to_string())]
        );
    }

    #[test]
    fn a_marker_inside_a_string_or_character_literal_is_not_a_comment() {
        let sources = [
            r#"let url = "http://example.com";"#,
            r#"let quote = "say \"hi\" // not a comment";"#,
            r##"let raw = r#"has " and // inside"#;"##,
            r#"let bytes = b"// bytes";"#,
            r#"let quote = '"'; let slash = '/';"#,
            "let s = \"two\nlines // still text\";",
        ];

        for source in sources {
            assert_eq!(texts(source), vec![], "no comment expected in: {source}");
        }
    }

    #[test]
    fn a_comment_after_a_quote_character_or_a_raw_string_is_found() {
        assert_eq!(texts("let q = '\"'; // one"), vec![(1, " one".to_string())]);
        assert_eq!(
            texts("let r = r#\"a \" b\"#; // two"),
            vec![(1, " two".to_string())]
        );
    }

    #[test]
    fn a_block_comment_gives_one_entry_for_each_line_it_spans() {
        assert_eq!(
            texts("/* one */ let x = 1;\n/*\n * two\n */\nlet y = 2; // three\n"),
            vec![
                (1, " one ".to_string()),
                (2, String::new()),
                (3, " * two".to_string()),
                (4, " ".to_string()),
                (5, " three".to_string()),
            ]
        );
    }

    #[test]
    fn a_nested_block_comment_stays_inside_the_outer_one() {
        assert_eq!(
            texts("/* outer /* inner */ still outer */ let x = 1; // after"),
            vec![
                (1, " outer /* inner */ still outer ".to_string()),
                (1, " after".to_string()),
            ]
        );
    }

    #[test]
    fn a_lifetime_or_label_is_not_taken_for_a_character_literal() {
        assert_eq!(
            texts("fn f<'a>(x: &'a str) -> &'a str { x } // end"),
            vec![(1, " end".to_string())]
        );
        assert_eq!(
            texts("'outer: loop { break 'outer; } // end"),
            vec![(1, " end".to_string())]
        );
    }

    #[test]
    fn a_lifetime_does_not_swallow_the_byte_after_its_name() {
        assert_eq!(
            texts("fn f(x: &'a/* one */ str) {}"),
            vec![(1, " one ".to_string())]
        );
    }

    #[test]
    fn escaped_and_multi_byte_character_literals_end_where_they_should() {
        for source in [
            "let c = '\\''; // end",
            "let c = '\\\\'; // end",
            "let c = '\\u{1F600}'; // end",
            "let c = 'é'; // end",
            "let c = '😀'; // end",
        ] {
            assert_eq!(texts(source), vec![(1, " end".to_string())], "in: {source}");
        }
    }

    #[test]
    fn text_that_is_not_valid_utf8_does_not_stop_the_scan() {
        let source = b"let x = \"\xff\"; // after\n";

        let comments = comment_lines(source);

        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].text, b" after");
    }

    #[test]
    fn an_unterminated_string_or_block_comment_ends_at_the_end_of_the_text() {
        assert_eq!(texts("let s = \"never closed // text"), vec![]);
        assert_eq!(
            texts("/* never closed"),
            vec![(1, " never closed".to_string())]
        );
    }
}
