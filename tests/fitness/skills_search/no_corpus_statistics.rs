//! Lookups A and B read no corpus statistics: entries that claim nothing a
//! request asks for change no A or B candidate.

use serde_json::{Value, json};
use super_rex::commands::skills::search::{
    FormTerms, Found, Gap, Index, Kind, Need, Question, RequestForm, SkillEntry, SkillMetadata,
    Tier, language_gaps, lookup_a, lookup_b, required_skills,
};

const NOISE_ENTRIES: usize = 50;

/// What a lookup found, with each skill named instead of placed, so that two
/// indexes of different sizes can be compared.
type Named = (
    Vec<(String, Tier, Vec<Need>, String, Vec<Question>, bool)>,
    Vec<Gap>,
);

#[derive(Debug, PartialEq)]
struct Lookups {
    a: Named,
    b: Named,
    required: Named,
    language_gaps: Vec<Gap>,
}

fn entry(name: &str, kind: Kind, fill: impl FnOnce(&mut SkillMetadata)) -> SkillEntry {
    let mut metadata = SkillMetadata::new(kind, "testing");
    fill(&mut metadata);
    let mut entry = SkillEntry::new(name, "skills-search");
    entry.metadata = Some(metadata);
    entry
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn claiming_index() -> Index {
    let skills = vec![
        entry("fx-coding-rules", Kind::Rule, |m| {
            m.claims.facts.writes_code = Some(true);
        }),
        entry("fx-design-sketch", Kind::Method, |m| {
            m.claims.activity = strings(&["design"]);
            m.requires = strings(&["fx-validation-plan"]);
        }),
        entry("fx-latency", Kind::Reference, |m| {
            m.claims.quality = strings(&["latency"]);
            m.claims.touches = strings(&["threads", "allocation"]);
            m.scope.writes_code = vec![true];
            m.scope.languages = strings(&["rust"]);
            m.scope.latency_scale = strings(&["ns_us"]);
            m.not_when = Some("Latency in milliseconds from a database or web call.".to_owned());
        }),
        entry("fx-proptest", Kind::Tool, |m| {
            m.claims.facts.writes_code = Some(true);
            m.claims.facts.languages = strings(&["rust"]);
            m.claims.facts.repo_dependencies = strings(&["proptest"]);
        }),
        entry("fx-test-first", Kind::Method, |m| {
            m.claims.activity = strings(&["build", "fix"]);
            m.scope.writes_code = vec![true];
        }),
        entry("fx-validation-plan", Kind::Rule, |m| {
            m.claims.artefact = strings(&["plan"]);
        }),
    ];
    Index { skills }
}

/// An entry that claims only terms and facts no test form asks for, while
/// its card and intents repeat every word of those forms.
fn noise(number: usize) -> SkillEntry {
    let mut entry = entry(&format!("fx-noise-{number:02}"), Kind::Method, |m| {
        m.claims.activity = strings(&["review"]);
        m.claims.touches = strings(&["retry"]);
        m.claims.quality = strings(&["readability"]);
        m.claims.artefact = strings(&["brief"]);
        m.claims.facts.role = Some("noise-reviewer".to_owned());
        m.intents = strings(&["latency threads build design rust go writes code proptest"]);
    });
    entry.card = "latency of threads in a rust build, design of go code with proptest".to_owned();
    entry.description = entry.card.clone();
    entry
}

/// The claiming index with the noise entries placed among its entries in
/// name order, so every position of a claiming entry moves.
fn noisy_index() -> Index {
    let mut skills = claiming_index().skills;
    skills.extend((0..NOISE_ENTRIES).map(noise));
    skills.sort_by(|left, right| left.name.cmp(&right.name));
    Index { skills }
}

fn read(value: Value) -> FormTerms {
    let mut value = value;
    value["id"] = json!("T");
    let form: RequestForm = serde_json::from_value(value).expect("the test form is valid");
    FormTerms::read(&form)
}

fn named(index: &Index, found: &Found) -> Named {
    let candidates = found
        .candidates
        .iter()
        .map(|candidate| {
            (
                index.skills[candidate.skill].name.clone(),
                candidate.tier,
                candidate.needs.clone(),
                candidate.why.clone(),
                candidate.questions.clone(),
                candidate.through_alias,
            )
        })
        .collect();
    (candidates, found.gaps.clone())
}

fn lookups(index: &Index, form: &FormTerms) -> Lookups {
    let a = lookup_a(index, form);
    let b = lookup_b(index, form);
    let mut found = a.candidates.clone();
    found.extend(b.candidates.iter().cloned());
    let required = required_skills(index, &found);
    Lookups {
        a: named(index, &a),
        b: named(index, &b),
        required: named(index, &required),
        language_gaps: language_gaps(index, form),
    }
}

#[test]
fn unrelated_entries_change_no_a_or_b_candidate() {
    let forms = [
        read(json!({
            "facts": {"writes_code": true, "languages": ["rust"], "latency_scale": "ns_us"},
            "activity": "build", "touches": ["threads"], "must_have": ["latency"]
        })),
        read(json!({
            "facts": {"writes_code": true, "languages": ["go"], "latency_scale": "ms"},
            "must_have": ["latency"]
        })),
        read(
            json!({"facts": {"writes_code": true, "languages": ["rust"]}, "must_have": ["latency"]}),
        ),
        read(json!({"facts": {"writes_code": true}, "activity": "design"})),
    ];
    let small = claiming_index();
    let large = noisy_index();
    assert_eq!(large.skills.len(), small.skills.len() + NOISE_ENTRIES);

    for form in &forms {
        let before = lookups(&small, form);
        let after = lookups(&large, form);

        assert!(
            !before.a.0.is_empty() || !before.b.0.is_empty() || !before.b.1.is_empty(),
            "the form must find something, or the test proves nothing: {form:?}"
        );
        assert_eq!(before, after, "form {form:?}");
    }
}
