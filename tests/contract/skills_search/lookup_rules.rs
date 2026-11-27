//! Lookup A on hand-built index values: fact claims, the evidence a tool skill
//! needs, and a language with no gap skill.

use serde_json::json;
use super_rex::commands::skills::search::{
    Gap, Index, Kind, Need, SkillEntry, SkillMetadata, Tier, language_gaps, lookup_a,
};

use super::lookup_declared::{main_index, names, read, step, steps};

#[test]
fn writes_code_finds_the_rule_skill() {
    let index = main_index();
    let form = read(json!({"id": "T4", "facts": {"writes_code": true}}));

    let a = lookup_a(&index, &form);

    assert_eq!(
        steps(&index, &a),
        [step(
            "fx-coding-rules",
            Tier::A,
            vec![Need::Fact("writes_code".to_owned())],
            "rule: writes_code=true",
        )]
    );
    assert_eq!(a.gaps, []);
}

#[test]
fn tool_skill_needs_evidence_in_the_repo_or_the_task() {
    let index = main_index();
    let on_dependency = read(json!({
        "facts": {"writes_code": true, "languages": ["rust"], "repo_dependencies": ["proptest"]}
    }));
    let on_task_words = read(json!({
        "facts": {"writes_code": true, "languages": ["rust"]},
        "needs": ["write a property test with proptest"]
    }));
    let on_nothing = read(json!({"facts": {"writes_code": true, "languages": ["rust"]}}));

    let by_dependency = lookup_a(&index, &on_dependency);
    let by_name = lookup_a(&index, &on_task_words);
    let without = lookup_a(&index, &on_nothing);

    let coding_rules = step(
        "fx-coding-rules",
        Tier::A,
        vec![Need::Fact("writes_code".to_owned())],
        "rule: writes_code=true",
    );
    assert_eq!(
        steps(&index, &by_dependency),
        [
            coding_rules.clone(),
            step(
                "fx-proptest",
                Tier::A,
                vec![Need::Fact(
                    "writes_code+languages+repo_dependencies".to_owned()
                )],
                "rule: writes_code=true, languages=rust, repo_dependencies=proptest",
            ),
        ]
    );
    assert_eq!(
        steps(&index, &by_name),
        [
            coding_rules,
            step(
                "fx-proptest",
                Tier::A,
                vec![Need::Fact("writes_code+languages".to_owned())],
                "rule: writes_code=true, languages=rust; tool named in the task: proptest",
            ),
        ]
    );
    assert_eq!(names(&index, &without), ["fx-coding-rules"]);

    let mut metadata = SkillMetadata::new(Kind::Tool, "testing");
    metadata.claims.facts.repo_dependencies = vec!["tokio".to_owned()];
    let mut tokio = SkillEntry::new("fx-tokio", "skills-search");
    tokio.metadata = Some(metadata);
    let only_named = Index {
        skills: vec![tokio],
    };
    let named_in_need = read(json!({"needs": ["use tokio for the server"]}));
    assert_eq!(
        steps(&only_named, &lookup_a(&only_named, &named_in_need)),
        [step(
            "fx-tokio",
            Tier::A,
            vec![Need::Fact("named".to_owned())],
            "rule: tool named in the task: tokio",
        )]
    );
}

#[test]
fn language_with_no_gap_skill_is_a_gap() {
    let index = main_index();
    let form = read(json!({"facts": {"writes_code": true, "languages": ["go", "ts", "rust"]}}));
    let not_writing_code = read(json!({"facts": {"writes_code": false, "languages": ["go"]}}));

    let gaps = language_gaps(&index, &form);

    assert_eq!(
        gaps,
        [Gap {
            need: "languages:go".to_owned(),
            reason: "no skill claims the language go".to_owned(),
            skill: None,
            library: None,
            not_when: None,
        }]
    );
    assert_eq!(language_gaps(&index, &not_writing_code), []);
}
