//! Lookup B on hand-built index values: declared terms, aliases, the scope
//! check and its gaps, `requires`, and unrecognised words.

use serde_json::{Value, json};
use super_rex::commands::skills::search::{
    AskedTerm, FormTerms, Found, Gap, Index, Kind, Need, Placement, Question, RequestForm,
    SkillEntry, SkillMetadata, TermField, TextQuery, Tier, language_gaps, lookup_a, lookup_b,
    required_skills,
};

const LIBRARY: &str = "skills-search";
const LATENCY_NOT_WHEN: &str = "Latency in milliseconds from a database or web call.";

/// One candidate as a trace step shows it: skill name, tier, needs, `why`
/// and questions.
pub(super) type Step = (String, Tier, Vec<Need>, String, Vec<Question>);

/// The skills of the fixture library `skills-search`, built by hand with the
/// claims their headers hold, in name order.
pub(super) fn main_index() -> Index {
    let skills = vec![
        entry("fx-coding-rules", Kind::Rule, "coding-standards", |m| {
            m.claims.facts.writes_code = Some(true);
        }),
        entry("fx-debugging", Kind::Method, "testing", |m| {
            m.claims.activity = strings(&["fix"]);
            m.scope.writes_code = vec![true];
            m.not_when = Some("A change that fixes no fault, such as a new feature.".to_owned());
        }),
        entry("fx-design-sketch", Kind::Method, "design", |m| {
            m.claims.activity = strings(&["design"]);
            m.requires = strings(&["fx-validation-plan"]);
        }),
        entry("fx-diagrams", Kind::Tool, "diagrams", |m| {
            m.claims.artefact = strings(&["diagram"]);
            m.replaces = strings(&["fx-old-diagrams"]);
        }),
        entry("fx-gap-ts", Kind::Rule, "coding-standards", |m| {
            m.claims.facts.writes_code = Some(true);
            m.claims.facts.languages = strings(&["ts"]);
        }),
        entry("fx-latency", Kind::Reference, "performance", |m| {
            m.claims.quality = strings(&["latency"]);
            m.claims.touches = strings(&["threads", "allocation"]);
            m.scope.writes_code = vec![true];
            m.scope.languages = strings(&["rust"]);
            m.scope.latency_scale = strings(&["ns_us"]);
            m.not_when = Some(LATENCY_NOT_WHEN.to_owned());
        }),
        entry("fx-old-diagrams", Kind::Tool, "diagrams", |m| {
            m.claims.artefact = strings(&["diagram"]);
        }),
        entry("fx-proptest", Kind::Tool, "testing", |m| {
            m.claims.facts.writes_code = Some(true);
            m.claims.facts.languages = strings(&["rust"]);
            m.claims.facts.repo_dependencies = strings(&["proptest"]);
        }),
        SkillEntry::new("fx-release-notes", LIBRARY),
        entry("fx-test-first", Kind::Method, "testing", |m| {
            m.claims.activity = strings(&["build", "fix"]);
            m.scope.writes_code = vec![true];
            m.not_when = Some("Work that writes no code, such as a change to a README.".to_owned());
        }),
        entry("fx-validation-plan", Kind::Rule, "design", |m| {
            m.claims.artefact = strings(&["plan"]);
            m.placement = Placement::OpenWhen;
        }),
    ];
    Index { skills }
}

/// The form of `value`, with the id `T` when it names none, read against
/// the vocabulary.
pub(super) fn read(value: Value) -> FormTerms {
    let mut value = value;
    if value.get("id").is_none() {
        value["id"] = json!("T");
    }
    let form: RequestForm = serde_json::from_value(value).expect("the test form is valid");
    FormTerms::read(&form)
}

pub(super) fn steps(index: &Index, found: &Found) -> Vec<Step> {
    found
        .candidates
        .iter()
        .map(|candidate| {
            (
                index.skills[candidate.skill].name.clone(),
                candidate.tier,
                candidate.needs.clone(),
                candidate.why.clone(),
                candidate.questions.clone(),
            )
        })
        .collect()
}

pub(super) fn names(index: &Index, found: &Found) -> Vec<String> {
    found
        .candidates
        .iter()
        .map(|candidate| index.skills[candidate.skill].name.clone())
        .collect()
}

pub(super) fn step(skill: &str, tier: Tier, needs: Vec<Need>, why: &str) -> Step {
    (skill.to_owned(), tier, needs, why.to_owned(), Vec::new())
}

fn entry(
    name: &str,
    kind: Kind,
    category: &str,
    fill: impl FnOnce(&mut SkillMetadata),
) -> SkillEntry {
    let mut metadata = SkillMetadata::new(kind, category);
    fill(&mut metadata);
    let mut entry = SkillEntry::new(name, LIBRARY);
    entry.metadata = Some(metadata);
    entry
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn term(field: TermField, term: &str) -> Need {
    Need::Term {
        field,
        term: term.to_owned(),
    }
}

fn asked(field: TermField, term: &str, from_alias: Option<&str>) -> AskedTerm {
    AskedTerm {
        field,
        term: term.to_owned(),
        from_alias: from_alias.map(str::to_owned),
    }
}

fn scope_question(fact: &str, values: &[&str]) -> Question {
    Question::Scope {
        fact: fact.to_owned(),
        values: strings(values),
    }
}

#[test]
fn latency_reference_is_returned_when_its_scope_is_met() {
    let index = main_index();
    let form = read(json!({
        "id": "T1",
        "facts": {"writes_code": true, "languages": ["rust"], "latency_scale": "ns_us"},
        "activity": "build",
        "touches": ["threads"],
        "must_have": ["latency"]
    }));

    let a = lookup_a(&index, &form);
    let b = lookup_b(&index, &form);

    assert_eq!(
        form.terms,
        [
            asked(TermField::Quality, "latency", None),
            asked(TermField::Activity, "build", None),
            asked(TermField::Touches, "threads", None),
        ]
    );
    assert!(form.unrecognised.is_empty(), "{:?}", form.unrecognised);
    assert_eq!(
        steps(&index, &a),
        [step(
            "fx-coding-rules",
            Tier::A,
            vec![Need::Fact("writes_code".to_owned())],
            "rule: writes_code=true",
        )]
    );
    assert_eq!(
        steps(&index, &b),
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
    assert_eq!(b.gaps, []);
}

#[test]
fn latency_reference_is_a_gap_for_a_go_service_in_milliseconds() {
    let index = main_index();
    let form = read(json!({
        "id": "T2",
        "facts": {"writes_code": true, "languages": ["go"], "latency_scale": "ms"},
        "must_have": ["latency"]
    }));

    let b = lookup_b(&index, &form);
    let language_gaps = language_gaps(&index, &form);

    assert_eq!(steps(&index, &b), []);
    assert_eq!(
        b.gaps,
        [Gap {
            need: "must_have:latency".to_owned(),
            reason: "scope: languages go not in rust; latency_scale ms not in ns_us".to_owned(),
            skill: Some("fx-latency".to_owned()),
            library: Some(LIBRARY.to_owned()),
            not_when: Some(LATENCY_NOT_WHEN.to_owned()),
        }]
    );
    assert_eq!(
        language_gaps,
        [Gap {
            need: "languages:go".to_owned(),
            reason: "no skill claims the language go".to_owned(),
            skill: None,
            library: None,
            not_when: None,
        }]
    );
}

#[test]
fn unknown_scope_returns_the_skill_with_its_question() {
    let index = main_index();
    let form = read(json!({
        "id": "T3",
        "facts": {"writes_code": true, "languages": ["rust"]},
        "must_have": ["latency"]
    }));

    let b = lookup_b(&index, &form);

    assert_eq!(
        steps(&index, &b),
        [(
            "fx-latency".to_owned(),
            Tier::B,
            vec![term(TermField::Quality, "latency")],
            "declared: quality latency; scope unknown: latency_scale".to_owned(),
            vec![scope_question("latency_scale", &["ns_us"])],
        )]
    );
    assert_eq!(b.gaps, []);
}

#[test]
fn required_skill_comes_with_the_skill_that_requires_it() {
    let index = main_index();
    let form = read(json!({"facts": {"writes_code": true}, "activity": "design"}));

    let b = lookup_b(&index, &form);
    let required = required_skills(&index, &b.candidates);

    assert_eq!(
        steps(&index, &b),
        [step(
            "fx-design-sketch",
            Tier::B,
            vec![term(TermField::Activity, "design")],
            "declared: activity design",
        )]
    );
    assert_eq!(
        steps(&index, &required),
        [step(
            "fx-validation-plan",
            Tier::B,
            vec![Need::Required("fx-design-sketch".to_owned())],
            "required by fx-design-sketch",
        )]
    );
    assert_eq!(required.gaps, []);

    let mut two_requirers = index.clone();
    two_requirers
        .skills
        .push(entry("fx-zrule", Kind::Rule, "design", |m| {
            m.claims.activity = strings(&["design"]);
            m.requires = strings(&["fx-validation-plan"]);
        }));
    let b = lookup_b(&two_requirers, &form);
    let required = required_skills(&two_requirers, &b.candidates);
    assert_eq!(names(&two_requirers, &b), ["fx-design-sketch", "fx-zrule"]);
    assert_eq!(
        steps(&two_requirers, &required),
        [step(
            "fx-validation-plan",
            Tier::B,
            vec![
                Need::Required("fx-design-sketch".to_owned()),
                Need::Required("fx-zrule".to_owned()),
            ],
            "required by fx-design-sketch",
        )]
    );

    let mut missing = index.clone();
    missing
        .skills
        .retain(|entry| entry.name != "fx-validation-plan");
    let b = lookup_b(&missing, &form);
    let required = required_skills(&missing, &b.candidates);
    assert_eq!(steps(&missing, &required), []);
    assert_eq!(
        required.gaps,
        [Gap {
            need: "requires:fx-validation-plan".to_owned(),
            reason: "required by fx-design-sketch, and no served library holds it".to_owned(),
            skill: Some("fx-design-sketch".to_owned()),
            library: None,
            not_when: None,
        }]
    );
}

#[test]
fn unknown_words_are_unrecognised_and_aliases_map_onto_terms() {
    let index = main_index();
    let form = read(json!({
        "must_have": ["p99"],
        "activity": "biuld",
        "touches": ["concurency"],
        "facts": {"frobnicate": 1, "languages": ["cobol"]}
    }));

    let b = lookup_b(&index, &form);

    assert_eq!(
        form.unrecognised,
        [
            "facts:frobnicate",
            "languages:cobol",
            "activity:biuld",
            "touches:concurency"
        ]
    );
    assert_eq!(
        form.terms,
        [asked(TermField::Quality, "latency", Some("p99"))]
    );
    assert_eq!(
        form.keyword_queries,
        [
            TextQuery {
                need: Need::Word("activity:biuld".to_owned()),
                text: "biuld".to_owned(),
            },
            TextQuery {
                need: Need::Word("touches:concurency".to_owned()),
                text: "concurency".to_owned(),
            },
        ]
    );
    assert_eq!(
        steps(&index, &b),
        [(
            "fx-latency".to_owned(),
            Tier::B,
            vec![term(TermField::Quality, "latency")],
            "declared: quality latency (from p99); scope unknown: writes_code, languages, latency_scale"
                .to_owned(),
            vec![
                scope_question("writes_code", &["true"]),
                scope_question("languages", &["rust"]),
                scope_question("latency_scale", &["ns_us"]),
            ],
        )]
    );
    assert!(b.candidates[0].through_alias);
    assert_eq!(b.gaps, []);

    let asked_twice = read(json!({
        "facts": {"writes_code": true, "languages": ["rust"], "latency_scale": "ns_us"},
        "must_have": ["latency", "p99"]
    }));
    let b = lookup_b(&index, &asked_twice);
    assert_eq!(
        asked_twice.terms,
        [asked(TermField::Quality, "latency", None)]
    );
    assert_eq!(
        steps(&index, &b),
        [step(
            "fx-latency",
            Tier::B,
            vec![term(TermField::Quality, "latency")],
            "declared: quality latency; scope met",
        )]
    );
}
