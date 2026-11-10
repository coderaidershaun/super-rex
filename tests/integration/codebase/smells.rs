//! Integration test of `rex smells`: which files it reads, and which comments
//! count as a flag.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use assert_cmd::Command;
use tempfile::tempdir;

fn smells(dir: &Path) -> String {
    let assert = Command::cargo_bin("rex")
        .expect("find rex binary")
        .arg("smells")
        .current_dir(dir)
        .assert()
        .success();
    String::from_utf8(assert.get_output().stdout.clone()).expect("utf8 stdout")
}

// Each row is one Rust file and the lines of it that must be listed. The rows
// hold every construct that can hide a flag from, or show a false one to, a
// reader that looks at one line at a time.
#[test]
fn smells_lists_each_flag_then_the_count() {
    type Row = (&'static str, &'static [u8], &'static [usize]);
    let rows: &[Row] = &[
        // What counts as a flag.
        ("flag_after_code", b"let total = 1; // SMELL: magic number\n", &[1]),
        ("flag_indented", b"    // SMELL: indented\n", &[1]),
        ("flag_no_space", b"//SMELL: no space\n", &[1]),
        ("flag_bare", b"// SMELL\n", &[1]),
        ("flag_tagged", b"// SMELL(naming): tagged\n", &[1]),
        ("flag_doc_comment", b"/// SMELL: on a doc comment\n//! SMELL: on a module doc\n", &[1, 2]),
        ("flag_windows_line_end", b"// SMELL: windows line ending\r\nfn a() {}\r\n", &[1]),
        ("flag_block_comment", b"/* SMELL: in a block comment */\n/** SMELL: in a block doc */\n", &[1, 2]),
        ("flag_two_on_a_line", b"/* SMELL: one */ /* SMELL: two */ // SMELL: three\n", &[1]),
        // What does not.
        ("not_flag_mentions_it", b"// The SMELL: flag is a comment.\n// SMELLY code\n// SMELL_COUNT\n// smell: lower case\n", &[]),
        ("not_flag_in_string", b"const TEXT: &str = \"// SMELL: only text\";\nconst FLAG: &str = \"SMELL:\";\n", &[]),
        ("not_flag_after_escaped_quote", b"let s = \"say \\\" // SMELL: still text\";\n", &[]),
        ("after_escaped_backslash_string", b"let s = \"a\\\\\"; // SMELL: after\n", &[1]),
        ("not_flag_in_byte_string", b"const TEXT: &[u8] = b\"// SMELL: only bytes\";\n", &[]),
        // Only a line that starts with the word counts, inside a block comment.
        ("block_later_lines", b"/*\n * SMELL: this line\n * mentions SMELL: in the middle\n */\nfn a() {}\n", &[2]),
        // A quote must not hide the flag that follows it.
        ("after_escaped_quotes", b"let q = \"say \\\"hi\\\"\"; // SMELL: after escaped quotes\n", &[1]),
        ("after_url_in_string", b"let u = \"http://example.com\"; // SMELL: after a url\n", &[1]),
        ("after_quote_char", b"fn quote() -> char { '\"' } // SMELL: after a quote character\n", &[1]),
        ("after_raw_string", b"fn raw() -> &'static str { r#\"a \" b\"# } // SMELL: after a raw string\n", &[1]),
        ("after_byte_char", b"let b = b'\"'; // SMELL: after a byte literal\n", &[1]),
        ("after_escaped_chars", b"let a = '\\''; // SMELL: one\nlet b = '\\\\'; // SMELL: two\nlet c = '\\u{1F600}'; // SMELL: three\n", &[1, 2, 3]),
        // A lifetime is not the start of a character literal.
        ("after_lifetimes", b"fn l<'a>(x: &'a str) -> &'a str { x } // SMELL: after lifetimes\n'outer: loop { break 'outer; } // SMELL: after a label\n", &[1, 2]),
        ("after_lifetime_then_comment", b"fn f(x: &'a/* SMELL: right after the name */ str) {}\n", &[1]),
        ("after_multi_byte", "fn accent() -> char { 'é' } // SMELL: after one\nfn face() -> char { '😀' } // SMELL: after four\n".as_bytes(), &[1, 2]),
        ("after_invalid_utf8", b"let x = \"\xff\"; // SMELL: after bytes that are not text\n", &[1]),
        // Block comments nest.
        ("block_nested_later_line", b"/* a /* b */\n SMELL: still inside the outer one\n*/\nfn a() {}\n", &[2]),
        ("block_nested_then_line_comment", b"/* outer /* inner */ still outer */ let x = 1; // SMELL: after\n", &[1]),
        // The end of the file ends a string or a comment that was not closed.
        ("unterminated_string", b"let s = \"never closed // SMELL: text\n", &[]),
        ("unterminated_block_comment", b"fn a() {}\n/* SMELL: never closed\n", &[2]),
    ];

    let dir = tempdir().expect("create tempdir");
    for (name, source, _) in rows {
        fs::write(dir.path().join(format!("{name}.rs")), source).expect("write row");
    }

    let stdout = smells(dir.path());

    let mut found: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for line in stdout
        .lines()
        .filter(|line| !line.starts_with("Code smells:"))
    {
        let (path, number) = line.rsplit_once(':').expect("a flag line is path:line");
        found
            .entry(path.to_string())
            .or_default()
            .push(number.parse().expect("line number"));
    }
    for (name, _, expected) in rows {
        let got = found.remove(&format!("{name}.rs")).unwrap_or_default();
        assert_eq!(&got, expected, "row {name}");
    }
    assert!(found.is_empty(), "flags in files no row expects: {found:?}");
    let total: usize = rows.iter().map(|(_, _, lines)| lines.len()).sum();
    assert!(
        stdout.ends_with(&format!("Code smells: {total}\n")),
        "count must be {total}: {stdout}"
    );

    // Which files are read, and in which order they are listed.
    let layout = tempdir().expect("create tempdir");
    for (path, contents) in [
        (
            "src/a/deep.rs",
            "fn deep() {\n    let total = 1; // SMELL: magic number\n}\n\n/// SMELL: does too much\nfn big() {}\n",
        ),
        ("src/b.rs", "// SMELL: bad name\nfn b() {}\n"),
        ("upper.RS", "// SMELL: an upper case extension\n"),
        ("notes.md", "// SMELL: not a Rust file\n"),
        ("target/gen.rs", "// SMELL: gitignored\n"),
        (".gitignore", "/target\n"),
        (".hidden/h.rs", "// SMELL: hidden\n"),
    ] {
        let file = layout.path().join(path);
        fs::create_dir_all(file.parent().expect("fixture has a parent")).expect("create dirs");
        fs::write(file, contents).expect("write fixture");
    }
    assert_eq!(
        smells(layout.path()),
        "src/a/deep.rs:2\nsrc/a/deep.rs:5\nsrc/b.rs:1\nupper.RS:1\nCode smells: 4\n"
    );

    let clean = tempdir().expect("create tempdir");
    fs::write(clean.path().join("main.rs"), "fn main() {}\n").expect("write main.rs");
    assert_eq!(smells(clean.path()), "Code smells: 0\n");
}
