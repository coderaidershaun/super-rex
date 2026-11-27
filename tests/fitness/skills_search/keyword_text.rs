//! `not_when` text is never searched, and every candidate of lookup C carries
//! a yes or no question.

use super_rex::commands::skills::search::{
    CARD_WORDS, FormTerms, Index, Kind, Need, Question, SkillEntry, SkillMetadata, TextQuery,
    lookup_c,
};

fn form_of_needs(texts: &[&str]) -> FormTerms {
    FormTerms {
        id: "T1".to_owned(),
        needs: texts.iter().map(|text| (*text).to_owned()).collect(),
        keyword_queries: texts
            .iter()
            .enumerate()
            .map(|(position, text)| TextQuery {
                need: Need::Text(position),
                text: (*text).to_owned(),
            })
            .collect(),
        ..FormTerms::default()
    }
}

/// An entry as the loader would build it from these header values.
fn skill(
    name: &str,
    library: &str,
    description: &str,
    when_to_use: &str,
    intents: Option<&[&str]>,
) -> SkillEntry {
    let mut entry = SkillEntry::new(name, library);
    entry.description = description.to_owned();
    entry.when_to_use = when_to_use.to_owned();
    entry.card = description
        .split_whitespace()
        .take(CARD_WORDS)
        .collect::<Vec<_>>()
        .join(" ");
    entry.metadata = intents.map(|intents| {
        let mut metadata = SkillMetadata::new(Kind::Method, "testing");
        metadata.intents = intents.iter().map(|intent| (*intent).to_owned()).collect();
        metadata
    });
    entry
}

/// The names, descriptions and intents of the fixture library.
fn fixture_index() -> Index {
    let main = "skills-search";
    let mut skills = vec![
        skill(
            "fx-coding-rules",
            main,
            "Rules for names, types and function signatures in any language.",
            "Use when a task writes or changes code.",
            Some(&[
                "name a public function",
                "choose a function signature",
                "write a clear error message",
            ]),
        ),
        skill(
            "fx-debugging",
            main,
            "Find the cause of a bug before you change the code.",
            "Use when a task fixes a bug in code.",
            Some(&[
                "find the cause of a bug",
                "reproduce a fault before a fix",
                "read a stack trace",
            ]),
        ),
        skill(
            "fx-design-sketch",
            main,
            "Design sketch before code, with boxes, arrows and one named pattern.",
            "Use when a task designs a new system, feature or module.",
            Some(&[
                "sketch a module design before code",
                "name the pattern of a design",
                "list the boxes of a new feature",
            ]),
        ),
        skill(
            "fx-diagrams",
            main,
            "How to draw flowcharts and sequence diagrams in Mermaid.",
            "Use when a plan or a document needs a diagram.",
            Some(&[
                "draw a flowchart of a module",
                "draw a sequence diagram",
                "update the diagram in a readme",
            ]),
        ),
        skill(
            "fx-gap-ts",
            main,
            "What TypeScript lacks that Rust gives by default, and the rule for each gap.",
            "Use when a task writes TypeScript code.",
            Some(&[
                "write typescript with checked errors",
                "avoid the any type in typescript",
                "narrow a union type in typescript",
            ]),
        ),
        skill(
            "fx-latency",
            main,
            "Reference for Rust hot paths that must stay fast, how to measure them, how to read a profile, and how to cut tail latency from nanoseconds to tens of microseconds on a Linux host.",
            "Use when a Rust hot path must stay fast or a profile shows slow code.",
            Some(&[
                "measure a hot path",
                "cut tail latency of a loop",
                "find what slows the matching loop",
            ]),
        ),
        skill(
            "fx-old-diagrams",
            main,
            "An older guide to diagrams drawn in plain text.",
            "Use when a document needs a diagram in plain text.",
            Some(&[
                "draw a diagram in ascii art",
                "draw boxes and arrows in plain text",
                "sketch a diagram in a code comment",
            ]),
        ),
        skill(
            "fx-proptest",
            main,
            "How to write property tests with the proptest crate in Rust.",
            "Use when a Rust task writes a property test with proptest.",
            Some(&[
                "write a property test with proptest",
                "shrink a failing proptest case",
                "generate random inputs for a test",
            ]),
        ),
        skill(
            "fx-release-notes",
            main,
            "Writes release notes and a changelog entry for a new version.",
            "Use when you write release notes or a changelog entry.",
            None,
        ),
        skill(
            "fx-test-first",
            main,
            "Test-driven development with a red-green-refactor loop.",
            "Use when a task builds a feature or fixes a bug in code.",
            Some(&[
                "build a feature test first",
                "fix a bug with a failing test first",
                "red green refactor loop",
            ]),
        ),
        skill(
            "fx-validation-plan",
            main,
            "Expected traces of each flow and failure, written in the plan before code.",
            "Use when a plan needs the expected trace of a flow.",
            Some(&[
                "write the expected trace of a flow",
                "list failure scenarios before code",
                "say how a test proves the order",
            ]),
        ),
        skill(
            "fx-coding-rules",
            "unrelated",
            "Another copy of coding rules, from a library that is not related.",
            "Use when a task writes or changes code.",
            Some(&[
                "name a public function",
                "choose a function signature",
                "write a clear error message",
            ]),
        ),
        skill(
            "fx-spreadsheets",
            "unrelated",
            "Formulas and charts for spreadsheets, such as a chart of web request latency in milliseconds.",
            "Use when a task builds a spreadsheet or a chart.",
            Some(&[
                "build a spreadsheet formula",
                "chart web request latency",
                "sum a column of numbers",
            ]),
        ),
        skill(
            "fx-web-timing",
            "unrelated",
            "Times web requests and reports their latency in milliseconds.",
            "Use when you time a web request or a call to an outside API.",
            None,
        ),
    ];
    skills[11].library_priority = 1;
    skills[11].shadowed_by = Some(main.to_owned());
    skills[12].library_priority = 1;
    skills[13].library_priority = 1;
    Index { skills }
}

#[test]
fn not_when_text_is_never_searched() {
    let mut fenced = SkillEntry::new("road-safety", "main");
    fenced.description = "How to cross a street.".to_owned();
    fenced.card = fenced.description.clone();
    fenced.when_to_use = "Use when you walk to work.".to_owned();
    let mut metadata = SkillMetadata::new(Kind::Reference, "testing");
    metadata.not_when = Some("A zebra at the zoo.".to_owned());
    metadata.intents = vec!["cross a busy street".to_owned()];
    fenced.metadata = Some(metadata);
    let index = Index {
        skills: vec![fenced],
    };

    let found = lookup_c(&index, &form_of_needs(&["zebra"])).expect("lookup C");

    assert_eq!(found.candidates, []);
}

#[test]
fn every_keyword_candidate_has_a_question() {
    let index = fixture_index();
    let needs = [
        "name a public function",
        "write a clear error message",
        "find the cause of a bug",
        "read a stack trace",
        "sketch a module design before code",
        "draw a sequence diagram",
        "update the diagram in a readme",
        "avoid the any type in typescript",
        "cut tail latency of a loop",
        "measure a hot path",
        "draw a diagram in ascii art",
        "write a property test with proptest",
        "generate random inputs for a test",
        "write release notes for a new version",
        "red green refactor loop",
        "write the expected trace of a flow",
        "build a spreadsheet formula",
        "time a web request",
        "how to read a profile",
        "a changelog entry",
    ];
    let mut checked = 0;

    for need in needs {
        let found = lookup_c(&index, &form_of_needs(&[need])).expect("lookup C");

        assert!(!found.candidates.is_empty(), "no candidate for {need:?}");
        for candidate in &found.candidates {
            let expected = [Question::Keyword {
                skill: index.skills[candidate.skill].name.clone(),
                need: Need::Text(0),
            }];
            assert_eq!(candidate.questions, expected, "candidate of {need:?}");
            assert_eq!(candidate.needs, [Need::Text(0)]);
            checked += 1;
        }
    }
    assert!(checked >= needs.len());
}
