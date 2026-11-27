//! The seam of the loader and the lookups: lookups A, B and C over an index
//! that the loader built from the fixture library.

use std::path::Path;

use serde_json::json;
use super_rex::commands::skills::search::{
    Gap, Index, Need, TermField, Tier, language_gaps, lookup_a, lookup_b, lookup_c,
};

use super::lookup_declared::{names, read, step, steps};

const LATENCY_NOT_WHEN: &str = "Latency in milliseconds from a database or web call.";

fn loaded_main() -> Index {
    let main = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/skills-search");
    Index::from_folders(&[main]).expect("the fixture library loads")
}

fn term(field: TermField, term: &str) -> Need {
    Need::Term {
        field,
        term: term.to_owned(),
    }
}

#[test]
fn lookups_find_the_fixture_skills_in_a_loaded_index() {
    let index = loaded_main();
    let rust_loop = read(json!({
        "id": "T1",
        "facts": {"writes_code": true, "languages": ["rust"], "latency_scale": "ns_us"},
        "activity": "build",
        "touches": ["threads"],
        "must_have": ["latency"]
    }));
    let go_service = read(json!({
        "id": "T2",
        "facts": {"writes_code": true, "languages": ["go"], "latency_scale": "ms"},
        "must_have": ["latency"]
    }));
    let release = read(json!({"needs": ["write release notes for a new version"]}));

    let rust_a = lookup_a(&index, &rust_loop);
    let rust_b = lookup_b(&index, &rust_loop);
    let go_b = lookup_b(&index, &go_service);
    let go_language_gaps = language_gaps(&index, &go_service);
    let release_c = lookup_c(&index, &release).expect("lookup C runs");

    assert!(
        rust_loop.unrecognised.is_empty(),
        "{:?}",
        rust_loop.unrecognised
    );
    assert_eq!(
        steps(&index, &rust_a),
        [step(
            "fx-coding-rules",
            Tier::A,
            vec![Need::Fact("writes_code".to_owned())],
            "rule: writes_code=true",
        )]
    );
    assert_eq!(
        steps(&index, &rust_b),
        [
            step(
                "fx-latency",
                Tier::B,
                vec![
                    term(TermField::Quality, "latency"),
                    term(TermField::Touches, "threads"),
                ],
                "declared: quality latency, touches threads; scope met",
            ),
            step(
                "fx-test-first",
                Tier::B,
                vec![term(TermField::Activity, "build")],
                "declared: activity build; scope met",
            ),
        ]
    );
    assert_eq!(rust_b.gaps, []);
    assert_eq!(steps(&index, &go_b), []);
    assert_eq!(
        go_b.gaps,
        [Gap {
            need: "must_have:latency".to_owned(),
            reason: "scope: languages go not in rust; latency_scale ms not in ns_us".to_owned(),
            skill: Some("fx-latency".to_owned()),
            library: Some("skills-search".to_owned()),
            not_when: Some(LATENCY_NOT_WHEN.to_owned()),
        }]
    );
    assert_eq!(
        go_language_gaps,
        [Gap {
            need: "languages:go".to_owned(),
            reason: "no skill claims the language go".to_owned(),
            skill: None,
            library: None,
            not_when: None,
        }]
    );
    assert_eq!(
        names(&index, &release_c).first().map(String::as_str),
        Some("fx-release-notes")
    );
}
