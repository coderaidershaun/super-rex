//! Integration tests of `rex codebase` on source files in languages other
//! than Rust: the sentence `--with-context` reads, and what `--only` keeps.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use tempfile::{TempDir, tempdir};

/// Writes each `(path, contents)` pair into a new temp dir.
fn write_tree(files: &[(&str, &str)]) -> TempDir {
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

/// Writes `files` into a new temp dir and prints the tree with context.
fn context_tree(files: &[(&str, &str)]) -> String {
    let dir = write_tree(files);
    codebase_stdout(dir.path(), &["--with-context"])
}

#[test]
fn codebase_with_context_reads_the_opening_comment_of_each_language() {
    let stdout = context_tree(&[
        ("UPPER.GO", "// Shouts the feed.\n\npackage feed\n"),
        ("build.sh", "# Builds the release archive.\nset -e\n"),
        ("chart.js", "// Draws the order book.\nimport a from 'a';\n"),
        (
            "chart.ts",
            "/** Draws the price chart. */\nimport a from 'a';\n",
        ),
        (
            "config.py",
            "\"\"\"Loads the run config.\"\"\"\nimport os\n",
        ),
        (
            "feed.c",
            "/* Reads the price feed. */\n#include <stdio.h>\n",
        ),
        ("feed.cpp", "// Parses the price feed.\n#include <string>\n"),
        ("feed.go", "// Reconnects the feed.\n\npackage feed\n"),
        ("main.rs", "//! Starts the tool.\nfn main() {}\n"),
    ]);

    assert_eq!(
        stdout,
        "UPPER.GO - Shouts the feed.\n\
        build.sh - Builds the release archive.\n\
        chart.js - Draws the order book.\n\
        chart.ts - Draws the price chart.\n\
        config.py - Loads the run config.\n\
        feed.c - Reads the price feed.\n\
        feed.cpp - Parses the price feed.\n\
        feed.go - Reconnects the feed.\n\
        main.rs - Starts the tool.\n"
    );
}

#[test]
fn codebase_with_context_passes_over_a_licence_header() {
    let stdout = context_tree(&[
        (
            "a_go_line_licence.go",
            "// Copyright 2026 Acme.\n\n// Reads the feed.\n\npackage feed\n",
        ),
        (
            "b_c_block_licence.h",
            "/*\n * Licensed under the MIT licence.\n */\n#ifndef FEED_H\n#define FEED_H\n\
             // Declares the feed reader.\n",
        ),
        (
            "c_python_licence.py",
            "# Copyright 2026 Acme.\n\n\"\"\"Loads the run config.\"\"\"\n",
        ),
        (
            "d_licence_alone.go",
            "// Copyright 2026 Acme.\n\npackage feed\n",
        ),
        (
            "e_no_blank_line.go",
            "// Copyright 2026 Acme.\n// Reads the feed.\npackage feed\n",
        ),
    ]);

    assert_eq!(
        stdout,
        "a_go_line_licence.go - Reads the feed.\n\
        b_c_block_licence.h - Declares the feed reader.\n\
        c_python_licence.py - Loads the run config.\n\
        d_licence_alone.go\n\
        e_no_blank_line.go\n"
    );
}

#[test]
fn codebase_with_context_ignores_lines_written_for_tools() {
    let stdout = context_tree(&[
        (
            "a_shebang.sh",
            "#!/bin/sh\n# shellcheck disable=SC2086\n# Builds the archive.\n",
        ),
        (
            "b_build_line.go",
            "//go:build linux\n\n// Reads the feed.\n\npackage feed\n",
        ),
        (
            "c_coding_line.py",
            "#!/usr/bin/env python3\n# -*- coding: utf-8 -*-\n\"\"\"Loads the config.\"\"\"\n",
        ),
        (
            "d_lint_and_strict.js",
            "/* eslint-disable */\n\"use strict\";\n// Draws the chart.\n",
        ),
        ("e_pragma.h", "#pragma once\n// Declares the reader.\n"),
        (
            "f_spdx.c",
            "// SPDX-License-Identifier: MIT\n// Reads the feed.\n",
        ),
    ]);

    assert_eq!(
        stdout,
        "a_shebang.sh - Builds the archive.\n\
        b_build_line.go - Reads the feed.\n\
        c_coding_line.py - Loads the config.\n\
        d_lint_and_strict.js - Draws the chart.\n\
        e_pragma.h - Declares the reader.\n\
        f_spdx.c - Reads the feed.\n"
    );
}

#[test]
fn codebase_with_context_shows_no_sentence_when_code_comes_first() {
    let stdout = context_tree(&[
        ("a_below_package.go", "package feed\n\n// Reads the feed.\n"),
        (
            "b_below_include.c",
            "#include <stdio.h>\n// Reads the feed.\n",
        ),
        ("c_below_command.sh", "set -e\n# Builds the archive.\n"),
        (
            "d_below_import.py",
            "import os\n\"\"\"Loads the config.\"\"\"\n",
        ),
        ("e_no_comment.go", "package feed\n"),
        ("f_empty.go", ""),
    ]);

    assert_eq!(
        stdout,
        "a_below_package.go\n\
        b_below_include.c\n\
        c_below_command.sh\n\
        d_below_import.py\n\
        e_no_comment.go\n\
        f_empty.go\n"
    );
}

#[test]
fn codebase_with_context_reads_a_go_package_comment() {
    let stdout = context_tree(&[
        (
            "a_line.go",
            "// Package feed reads the price feed.\npackage feed\n",
        ),
        (
            "b_block.go",
            "/*\nPackage feed reads the order feed.\n*/\npackage feed\n",
        ),
    ]);

    assert_eq!(
        stdout,
        "a_line.go - Package feed reads the price feed.\n\
        b_block.go - Package feed reads the order feed.\n"
    );
}

#[test]
fn codebase_with_context_reads_a_python_docstring() {
    let stdout = context_tree(&[
        ("a_one_line.py", "\"\"\"Loads the run config.\"\"\"\n"),
        (
            "b_opened_alone.py",
            "\"\"\"\nSaves the run config.\n\"\"\"\n",
        ),
        (
            "c_many_lines.py",
            "#!/usr/bin/env python3\n\"\"\"Checks the run config\nbefore a run. More.\n\"\"\"\n",
        ),
    ]);

    assert_eq!(
        stdout,
        "a_one_line.py - Loads the run config.\n\
        b_opened_alone.py - Saves the run config.\n\
        c_many_lines.py - Checks the run config before a run.\n"
    );
}

#[test]
fn codebase_with_context_reads_a_header_and_its_source_file() {
    let stdout = context_tree(&[
        ("feed.h", "// Declares the feed reader.\n#pragma once\n"),
        (
            "feed.cpp",
            "// Reads the feed over a socket.\n#include \"feed.h\"\n",
        ),
    ]);

    assert_eq!(
        stdout,
        "feed.cpp - Reads the feed over a socket.\n\
        feed.h - Declares the feed reader.\n"
    );
}

#[test]
fn codebase_with_context_reads_decorated_comments() {
    let stdout = context_tree(&[
        (
            "a_doc_block.ts",
            "/**\n * Draws the price chart.\n */\nimport a from 'a';\n",
        ),
        (
            "b_triple_slash.cpp",
            "/// Parses the price feed.\n#include <string>\n",
        ),
        (
            "c_banner.c",
            "/*****************\n * Reads the feed.\n *****************/\n",
        ),
        (
            "d_rule_lines.go",
            "// ==========\n// Reconnects the feed.\n// ==========\n\npackage feed\n",
        ),
    ]);

    assert_eq!(
        stdout,
        "a_doc_block.ts - Draws the price chart.\n\
        b_triple_slash.cpp - Parses the price feed.\n\
        c_banner.c - Reads the feed.\n\
        d_rule_lines.go - Reconnects the feed.\n"
    );
}

#[test]
fn codebase_with_context_lists_a_file_of_an_unknown_language_without_a_sentence() {
    let stdout = context_tree(&[
        ("Makefile", "# Builds everything.\nall:\n"),
        ("notes.txt", "// Notes on the feed.\n"),
        ("run", "#!/bin/sh\n# Runs the tool.\n"),
    ]);

    assert_eq!(stdout, "Makefile\nnotes.txt\nrun\n");
}

#[test]
fn codebase_with_context_reads_a_file_that_is_not_valid_utf8() {
    let dir = tempdir().expect("create tempdir");
    fs::write(
        dir.path().join("feed.go"),
        b"// Reads the \xff\xfe feed.\n\npackage feed\n",
    )
    .expect("write feed.go");

    let stdout = codebase_stdout(dir.path(), &["--with-context"]);

    assert_eq!(stdout, "feed.go - Reads the \u{FFFD}\u{FFFD} feed.\n");
}

#[test]
fn codebase_only_code_lists_source_files_and_their_dirs() {
    let dir = write_tree(&[
        ("Makefile", "all:\n"),
        ("a/b/x.txt", "text\n"),
        ("a/c/y.go", "package c\n"),
        ("a/z.py", "import os\n"),
        ("docs/n.md", "notes\n"),
        ("m.rs", "fn m() {}\n"),
        ("web/app.TSX", "export {};\n"),
    ]);

    let stdout = codebase_stdout(dir.path(), &["--only", "code"]);

    assert_eq!(
        stdout,
        "a/\n  c/\n    y.go\n  z.py\nm.rs\nweb/\n  app.TSX\n"
    );
}

#[test]
fn codebase_only_one_language_lists_that_language() {
    let dir = write_tree(&[
        ("docs/x.md", "notes\n"),
        ("feed.c", "int a;\n"),
        ("feed.cpp", "int b;\n"),
        ("feed.go", "package feed\n"),
        ("feed.h", "int c;\n"),
        ("lib/x.go", "package lib\n"),
        ("main.rs", "fn main() {}\n"),
    ]);

    let cases = [
        ("go", "feed.go\nlib/\n  x.go\n"),
        ("cpp", "feed.cpp\nfeed.h\n"),
        ("c", "feed.c\nfeed.h\n"),
    ];
    for (language, expected) in cases {
        let stdout = codebase_stdout(dir.path(), &["--only", language]);
        assert_eq!(stdout, expected, "--only {language}");
    }
}

#[test]
fn codebase_rust_only_prints_the_same_tree_as_only_rust() {
    let dir = write_tree(&[
        ("a/b/x.go", "package b\n"),
        ("a/c/y.rs", "//! Reads y. More.\n"),
        ("upper.RS", "fn main() {}\n"),
    ]);

    let rust_only = codebase_stdout(dir.path(), &["--rust-only", "--with-context"]);
    let only_rust = codebase_stdout(dir.path(), &["--only", "rust", "--with-context"]);

    assert_eq!(rust_only, "a/\n  c/\n    y.rs - Reads y.\nupper.RS\n");
    assert_eq!(only_rust, rust_only);
}

#[test]
fn the_codebase_md_footer_names_the_only_value() {
    let dir = write_tree(&[("src/feed.go", "// Reads the feed.\n\npackage feed\n")]);

    codebase_stdout(
        dir.path(),
        &["--with-context", "--only", "go", "--for-human"],
    );

    let written = fs::read_to_string(dir.path().join("CODEBASE.md")).expect("read CODEBASE.md");
    assert_eq!(
        written,
        "# Codebase\n\n\
        ```text\n\
        src/\n\
        \x20\x20feed.go - Reads the feed.\n\
        ```\n\n\
        Generated by `rex codebase --for-human --only go --with-context`.\n"
    );
}

/// Runs `rex codebase` with `args`, checks that clap refused the command line,
/// and returns what it printed to stderr.
fn refused_stderr(args: &[&str]) -> String {
    let dir = tempdir().expect("create tempdir");
    let assert = Command::cargo_bin("rex")
        .expect("find rex binary")
        .arg("codebase")
        .args(args)
        .current_dir(dir.path())
        .assert()
        .code(2);
    assert!(assert.get_output().stdout.is_empty());
    String::from_utf8(assert.get_output().stderr.clone()).expect("utf8 stderr")
}

#[test]
fn codebase_refuses_rust_only_together_with_only() {
    let stderr = refused_stderr(&["--rust-only", "--only", "go"]);

    assert!(
        stderr.contains("'--rust-only'") && stderr.contains("'--only <WHAT>'"),
        "stderr must name both flags, got: {stderr}"
    );
}

#[test]
fn codebase_refuses_an_unknown_language() {
    let stderr = refused_stderr(&["--only", "bogus"]);

    assert!(
        stderr.contains("'bogus'"),
        "stderr must name the bad value, got: {stderr}"
    );
    assert!(
        stderr.contains("[possible values: code, rust, go, c, cpp, python, js, ts, shell]"),
        "got: {stderr}"
    );
}
