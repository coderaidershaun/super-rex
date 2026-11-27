//! Every instruction of a library skill to open another skill, or a file of
//! one, is declared in the `open_when` list of its header, so that search sees it.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use super_rex::commands::skills::search::{Index, OpenWhen};

/// A span that names another skill's file but gives no instruction to open
/// it, so it needs no `open_when` item.
struct Boundary {
    skill: &'static str,
    span: &'static str,
    reason: &'static str,
}

/// Empty today: every span of the library is an instruction and is declared.
const BOUNDARIES: &[Boundary] = &[];

const SKILL_FILE: &str = "SKILL.md";

fn crate_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn folders_with_skill(root: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(root)
        .unwrap_or_else(|error| panic!("read {}: {error}", root.display()))
        .map(|entry| entry.expect("folder entry"))
        .filter(|entry| entry.path().join(SKILL_FILE).is_file())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// Every `.md` file below `folder`, as a path relative to it with `/`.
fn markdown_files(folder: &Path, relative: &str, found: &mut Vec<String>) {
    for entry in fs::read_dir(folder).expect("read skill folder") {
        let entry = entry.expect("folder entry");
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = if relative.is_empty() {
            name.clone()
        } else {
            format!("{relative}/{name}")
        };
        if entry.path().is_dir() {
            markdown_files(&entry.path(), &path, found);
        } else if name.ends_with(".md") {
            found.push(path);
        }
    }
}

/// The text of a file with the header of a `SKILL.md` cut off.
fn body_of(file: &str, text: &str) -> String {
    if file != SKILL_FILE {
        return text.to_owned();
    }
    let after_open = text.strip_prefix("---\n").unwrap_or(text);
    after_open
        .split_once("\n---\n")
        .map_or_else(|| text.to_owned(), |(_, body)| body.to_owned())
}

/// The backticked spans of `text` that look like `<skill>/<file>.md`.
fn path_spans(text: &str) -> Vec<(String, String)> {
    text.split('`')
        .skip(1)
        .step_by(2)
        .filter(|span| span.ends_with(".md") && !span.contains(char::is_whitespace))
        .filter_map(|span| span.split_once('/'))
        .map(|(skill, file)| (skill.to_owned(), file.to_owned()))
        .collect()
}

fn declares(items: &[OpenWhen], target: &str, file: &str) -> bool {
    items.iter().any(|item| {
        item.skill.as_deref() == Some(target)
            && match item.file.as_deref() {
                None => file == SKILL_FILE,
                Some(declared) => declared == file,
            }
    })
}

/// Each span of `library` that sends an agent to a skill of `library` or of
/// `other_skills` with no matching `open_when` item and no entry in `boundaries`,
/// written `holder: span`. Spans of the allow-list that are not in the text are
/// named too, so the list cannot go stale.
fn undeclared_spans(
    library: &Path,
    other_skills: Option<&Path>,
    boundaries: &[Boundary],
) -> Vec<String> {
    let skills = folders_with_skill(library);
    let mut targets: BTreeSet<String> = skills.iter().cloned().collect();
    targets.extend(
        other_skills
            .iter()
            .flat_map(|root| folders_with_skill(root)),
    );
    let index = Index::from_folders(&[library.to_path_buf()]).expect("index of the library");

    let mut problems = Vec::new();
    let mut allowed_seen = BTreeSet::new();
    for holder in &skills {
        let items: Vec<OpenWhen> = index
            .skills
            .iter()
            .find(|entry| &entry.name == holder)
            .and_then(|entry| entry.metadata.as_ref())
            .map(|metadata| metadata.open_when.clone())
            .unwrap_or_default();
        let mut files = Vec::new();
        markdown_files(&library.join(holder), "", &mut files);
        files.sort();
        for file in files {
            let text = fs::read_to_string(library.join(holder).join(&file)).expect("read file");
            for (target, target_file) in path_spans(&body_of(&file, &text)) {
                if &target == holder || !targets.contains(&target) {
                    continue;
                }
                let span = format!("{target}/{target_file}");
                if let Some(position) = boundaries
                    .iter()
                    .position(|b| b.skill == holder && b.span == span)
                {
                    allowed_seen.insert(position);
                } else if !declares(&items, &target, &target_file) {
                    problems.push(format!("{holder}/{file}: `{span}` has no open_when item"));
                }
            }
        }
    }
    for (position, boundary) in boundaries.iter().enumerate() {
        if !allowed_seen.contains(&position) {
            problems.push(format!(
                "allow-list entry `{}` of {} ({}) is in no text",
                boundary.span, boundary.skill, boundary.reason
            ));
        }
    }
    problems
}

// A body that sends an agent to a skill or file that the header does not
// declare is invisible to search and to the mod, which read only the header.
#[test]
fn every_library_instruction_to_open_a_skill_is_declared_in_its_header() {
    let problems = undeclared_spans(
        &crate_path("plugin/library"),
        Some(&crate_path("plugin/skills")),
        BOUNDARIES,
    );

    assert!(problems.is_empty(), "{problems:#?}");
}

#[test]
fn the_check_fails_on_a_span_with_no_open_when_item() {
    let library = crate_path("tests/fixtures/library-open-when/library");

    let problems = undeclared_spans(&library, None, &[]);

    assert_eq!(
        problems,
        [
            "holder/SKILL.md: `other/extra.md` has no open_when item",
            "holder/notes.md: `other/extra.md` has no open_when item",
        ]
    );
}

#[test]
fn a_boundary_entry_clears_its_span_and_a_stale_entry_is_named() {
    let library = crate_path("tests/fixtures/library-open-when/library");
    let boundaries = [
        Boundary {
            skill: "holder",
            span: "other/extra.md",
            reason: "names the table only",
        },
        Boundary {
            skill: "holder",
            span: "other/gone.md",
            reason: "stale",
        },
    ];

    let problems = undeclared_spans(&library, None, &boundaries);

    assert_eq!(
        problems,
        ["allow-list entry `other/gone.md` of holder (stale) is in no text"]
    );
}
