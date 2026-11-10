//! Integration tests of `rex codebase` on source files in many languages: the
//! sentence `--with-context` reads, and what `--only` keeps.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use tempfile::{TempDir, tempdir};

/// Writes each `(path, contents)` pair into a new temp dir.
fn write_tree(files: &[(&str, &[u8])]) -> TempDir {
    let dir = tempdir().expect("create tempdir");
    for (path, contents) in files {
        let file = dir.path().join(path);
        fs::create_dir_all(file.parent().expect("fixture has a parent")).expect("create dirs");
        fs::write(file, contents).expect("write fixture");
    }
    dir
}

/// Runs `rex codebase` with `args` in `dir` and returns what it printed.
fn codebase_stdout(dir: &Path, args: &[&str]) -> String {
    let assert = Command::cargo_bin("rex")
        .expect("find rex binary")
        .arg("codebase")
        .args(args)
        .current_dir(dir)
        .assert()
        .success();
    String::from_utf8(assert.get_output().stdout.clone()).expect("utf8 stdout")
}

// Each row is one file and the sentence that `--with-context` must print
// beside it, or `None` when the file must be listed bare.
#[test]
fn codebase_with_context_reads_the_opening_comment_of_each_language() {
    type Row = (&'static str, &'static [u8], Option<&'static str>);
    let rows: &[Row] = &[
        // One row for each language.
        ("UPPER.GO", b"// Shouts the feed.\n\npackage feed\n", Some("Shouts the feed.")),
        ("build.sh", b"# Builds the release archive.\nset -e\n", Some("Builds the release archive.")),
        ("chart.js", b"// Draws the order book.\nimport a from 'a';\n", Some("Draws the order book.")),
        ("chart.ts", b"/** Draws the price chart. */\nimport a from 'a';\n", Some("Draws the price chart.")),
        ("config.py", b"\"\"\"Loads the run config.\"\"\"\nimport os\n", Some("Loads the run config.")),
        ("feed.c", b"/* Reads the price feed. */\n#include <stdio.h>\n", Some("Reads the price feed.")),
        ("feed.cpp", b"// Parses the price feed.\n#include <string>\n", Some("Parses the price feed.")),
        ("feed.go", b"// Reconnects the feed.\n\npackage feed\n", Some("Reconnects the feed.")),
        ("main.rs", b"//! Starts the tool.\nfn main() {}\n", Some("Starts the tool.")),
        // Rust sentence rules.
        ("rs_three_lines.rs", b"//! Reads `a.md` and\n//! writes it back\n//! to disk. Then it stops.\nfn main() {}\n", Some("Reads `a.md` and writes it back to disk.")),
        ("rs_full_stop_on_first_line.rs", b"//! First line ends here.\n//! Second line is dropped.\n", Some("First line ends here.")),
        ("rs_empty_doc_lines.rs", b"//!\n//! Text with no full stop\n//!\n//! More text after a break.\n", Some("Text with no full stop")),
        ("rs_no_full_stop.rs", b"//! Runs until\n//! the doc lines end\nfn main() {}\n", Some("Runs until the doc lines end")),
        ("rs_no_doc.rs", b"fn main() {}\n", None),
        ("rs_doc_on_line_six.rs", b"fn a() {}\nfn b() {}\nfn c() {}\nfn d() {}\nfn e() {}\n//! Too late.\n", None),
        ("rs_heading_mark.rs", b"//! # Title\n//! Body.\n", Some("Title Body.")),
        ("rs_empty_line_first.rs", b"//!\n\n//! Text.\n", Some("Text.")),
        ("rs_licence.rs", b"//! Copyright 2026 Acme.\n\n//! Reads the feed.\n", Some("Reads the feed.")),
        ("rs_directive_in_capitals.rs", b"//! SPDX-LICENSE-IDENTIFIER: MIT\n//! Reads the feed.\n", Some("Reads the feed.")),
        ("not_rust.md", b"//! Not a module.\n", None),
        // A licence header is passed over.
        ("licence_go_line.go", b"// Copyright 2026 Acme.\n\n// Reads the feed.\n\npackage feed\n", Some("Reads the feed.")),
        ("licence_c_block.h", b"/*\n * Licensed under the MIT licence.\n */\n#ifndef FEED_H\n#define FEED_H\n// Declares the feed reader.\n", Some("Declares the feed reader.")),
        ("licence_python.py", b"# Copyright 2026 Acme.\n\n\"\"\"Loads the run config.\"\"\"\n", Some("Loads the run config.")),
        ("licence_alone.go", b"// Copyright 2026 Acme.\n\npackage feed\n", None),
        ("licence_no_blank_line.go", b"// Copyright 2026 Acme.\n// Reads the feed.\npackage feed\n", None),
        // Lines written for tools are passed over.
        ("tool_shebang.sh", b"#!/bin/sh\n# shellcheck disable=SC2086\n# Builds the archive.\n", Some("Builds the archive.")),
        ("tool_go_build.go", b"//go:build linux\n\n// Reads the feed.\n\npackage feed\n", Some("Reads the feed.")),
        ("tool_coding_line.py", b"#!/usr/bin/env python3\n# -*- coding: utf-8 -*-\n\"\"\"Loads the config.\"\"\"\n", Some("Loads the config.")),
        ("tool_eslint_strict.js", b"/* eslint-disable */\n\"use strict\";\n// Draws the chart.\n", Some("Draws the chart.")),
        ("tool_pragma.h", b"#pragma once\n// Declares the reader.\n", Some("Declares the reader.")),
        ("tool_spdx.c", b"// SPDX-License-Identifier: MIT\n// Reads the feed.\n", Some("Reads the feed.")),
        // Code before the comment: no sentence.
        ("code_first_package.go", b"package feed\n\n// Reads the feed.\n", None),
        ("code_first_include.c", b"#include <stdio.h>\n// Reads the feed.\n", None),
        ("code_first_command.sh", b"set -e\n# Builds the archive.\n", None),
        ("code_first_import.py", b"import os\n\"\"\"Loads the config.\"\"\"\n", None),
        ("code_only.go", b"package feed\n", None),
        ("empty.go", b"", None),
        // Go package comments.
        ("go_package_line.go", b"// Package feed reads the price feed.\npackage feed\n", Some("Package feed reads the price feed.")),
        ("go_package_block.go", b"/*\nPackage feed reads the order feed.\n*/\npackage feed\n", Some("Package feed reads the order feed.")),
        // Python docstrings.
        ("py_opened_alone.py", b"\"\"\"\nSaves the run config.\n\"\"\"\n", Some("Saves the run config.")),
        ("py_many_lines.py", b"#!/usr/bin/env python3\n\"\"\"Checks the run config\nbefore a run. More.\n\"\"\"\n", Some("Checks the run config before a run.")),
        // A header and its source file.
        ("pair.h", b"// Declares the feed reader.\n#pragma once\n", Some("Declares the feed reader.")),
        ("pair.cpp", b"// Reads the feed over a socket.\n#include \"pair.h\"\n", Some("Reads the feed over a socket.")),
        // Decorated comments.
        ("deco_doc_block.ts", b"/**\n * Draws the price chart.\n */\nimport a from 'a';\n", Some("Draws the price chart.")),
        ("deco_triple_slash.cpp", b"/// Parses the price feed.\n#include <string>\n", Some("Parses the price feed.")),
        ("deco_banner.c", b"/*****************\n * Reads the feed.\n *****************/\n", Some("Reads the feed.")),
        ("deco_rule_lines.go", b"// ==========\n// Reconnects the feed.\n// ==========\n\npackage feed\n", Some("Reconnects the feed.")),
        // An unknown language gets no sentence.
        ("Makefile", b"# Builds everything.\nall:\n", None),
        ("notes.txt", b"// Notes on the feed.\n", None),
        ("run", b"#!/bin/sh\n# Runs the tool.\n", None),
        // Bytes that are not UTF-8 are replaced, not fatal.
        ("not_utf8.go", b"// Reads the \xff\xfe feed.\n\npackage feed\n", Some("Reads the \u{FFFD}\u{FFFD} feed.")),
    ];

    // A generated or minified file can be one line megabytes long, and a doc
    // block can run for thousands of lines without a full stop. Neither may
    // decide how long a line of the tree is.
    let long_line = format!("//! {}\n", "x".repeat(200_000));
    let many_lines = "//! no full stop here\n".repeat(2_000);

    let dir = tempdir().expect("create tempdir");
    for (name, contents, _) in rows {
        fs::write(dir.path().join(name), contents).expect("write fixture");
    }
    fs::write(dir.path().join("cap_one_long_line.rs"), &long_line).expect("write long line");
    fs::write(dir.path().join("cap_many_doc_lines.rs"), &many_lines).expect("write many lines");

    let stdout = codebase_stdout(dir.path(), &["--with-context"]);

    let mut printed = std::collections::BTreeMap::new();
    for line in stdout.lines() {
        assert!(
            line.len() < 5_000,
            "a tree line grew to {} bytes: {:.60}",
            line.len(),
            line
        );
        let name = line.split(" - ").next().expect("a line has a name");
        printed.insert(name, line);
    }
    for (name, _, sentence) in rows {
        let expected = sentence.map_or_else(|| (*name).to_string(), |s| format!("{name} - {s}"));
        assert_eq!(printed.get(name), Some(&expected.as_str()), "row {name}");
    }
    assert!(
        printed["cap_one_long_line.rs"].starts_with("cap_one_long_line.rs - xxx"),
        "row cap_one_long_line.rs: {:.80}",
        printed["cap_one_long_line.rs"]
    );
    assert!(
        printed["cap_many_doc_lines.rs"]
            .starts_with("cap_many_doc_lines.rs - no full stop here no full stop"),
        "row cap_many_doc_lines.rs: {:.80}",
        printed["cap_many_doc_lines.rs"]
    );
    assert_eq!(printed.len(), rows.len() + 2, "no file is missing or extra");
}

// Each case is the arguments of one run and the tree it must print.
#[test]
fn codebase_only_code_lists_source_files_and_their_dirs() {
    let dir = write_tree(&[
        ("Makefile", b"all:\n"),
        ("a/b/x.txt", b"text\n"),
        ("a/c/y.go", b"package c\n"),
        ("a/c/y.rs", b"//! Reads y. More.\n"),
        ("a/z.py", b"import os\n"),
        ("a/z.rs", b"fn z() {}\n"),
        ("docs/n.md", b"notes\n"),
        ("feed.c", b"int a;\n"),
        ("feed.cpp", b"int b;\n"),
        ("feed.h", b"int c;\n"),
        ("lib/x.go", b"package lib\n"),
        ("m.rs", b"fn m() {}\n"),
        ("upper.RS", b"fn main() {}\n"),
        ("web/app.TSX", b"export {};\n"),
    ]);

    let rust = "a/\n  c/\n    y.rs\n  z.rs\nm.rs\nupper.RS\n";
    let cases: &[(&[&str], &str)] = &[
        (
            &["--only", "code"],
            "a/\n  c/\n    y.go\n    y.rs\n  z.py\n  z.rs\nfeed.c\nfeed.cpp\nfeed.h\n\
             lib/\n  x.go\nm.rs\nupper.RS\nweb/\n  app.TSX\n",
        ),
        (&["--only", "rust"], rust),
        (&["--rust-only"], rust),
        (&["--only", "go"], "a/\n  c/\n    y.go\nlib/\n  x.go\n"),
        (&["--only", "cpp"], "feed.cpp\nfeed.h\n"),
        (&["--only", "c"], "feed.c\nfeed.h\n"),
        (&["--only", "python"], "a/\n  z.py\n"),
        (
            &["--rust-only", "--with-context"],
            "a/\n  c/\n    y.rs - Reads y.\n  z.rs\nm.rs\nupper.RS\n",
        ),
    ];
    for (args, expected) in cases {
        assert_eq!(
            codebase_stdout(dir.path(), args),
            *expected,
            "args {args:?}"
        );
    }
    assert_eq!(
        codebase_stdout(dir.path(), &["--only", "rust", "--with-context"]),
        codebase_stdout(dir.path(), &["--rust-only", "--with-context"]),
        "--only rust must print the same as --rust-only"
    );
}
