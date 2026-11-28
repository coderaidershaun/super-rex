//! `rex skills search` on a library of the coding digest and the TypeScript gap skill:
//! a request may name a language by its common full name, such as `typescript`.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::answer::{field, result, skills};

fn languages_library() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/skills-search-languages")
}

fn code_in(languages: &[&str]) -> Value {
    json!({ "requests": [ { "id": "L",
                            "facts": { "writes_code": true, "languages": languages } } ] })
}

#[test]
fn typescript_finds_the_typescript_gap_skill_beside_the_digest() {
    let answer = result(&[languages_library()], &code_in(&["typescript"]));

    assert_eq!(skills(&answer), ["rex-code-essentials", "rex-gap-ts"]);
    assert_eq!(field(&answer, "tier"), ["A", "A"]);
    assert_eq!(
        answer["items"][1]["why"],
        "rule: writes_code=true, languages=ts"
    );
    assert_eq!(answer["unrecognised"], json!([]));
    assert_eq!(answer["gaps"], json!([]));
}

#[test]
fn each_full_name_of_a_language_is_read_as_its_short_name() {
    let full_names = ["javascript", "golang", "c++", "c#", "objective-c"];

    let answer = result(&[languages_library()], &code_in(&full_names));

    assert_eq!(skills(&answer), ["rex-code-essentials"]);
    assert_eq!(answer["unrecognised"], json!([]));
    let gaps = answer["gaps"].as_array().expect("gaps is a list");
    let needs: Vec<&str> = gaps
        .iter()
        .map(|gap| gap["need"].as_str().unwrap_or(""))
        .collect();
    assert_eq!(
        needs,
        [
            "languages:js",
            "languages:go",
            "languages:cpp",
            "languages:csharp",
            "languages:objc"
        ]
    );
}

#[test]
fn a_language_that_is_neither_a_value_nor_an_alias_is_unrecognised() {
    let answer = result(&[languages_library()], &code_in(&["cobol", "TypeScript"]));

    assert_eq!(skills(&answer), ["rex-code-essentials", "rex-gap-ts"]);
    assert_eq!(answer["unrecognised"], json!(["languages:cobol"]));
}

#[test]
fn a_language_named_twice_counts_once() {
    let answer = result(&[languages_library()], &code_in(&["go", "golang"]));

    assert_eq!(answer["gaps"].as_array().map(Vec::len), Some(1));
    assert_eq!(answer["gaps"][0]["need"], "languages:go");
}
