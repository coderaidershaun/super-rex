//! Lookup C on hand-built index values: column weights, one query per need,
//! the cut of each query, and caller text that holds FTS5 syntax.

use super_rex::commands::skills::search::{
    Candidate, FormTerms, Index, KeywordTable, Kind, Need, Question, SkillEntry, SkillMetadata,
    TextQuery, Tier, lookup_c,
};

fn entry(name: &str, card: &str, intents: &[&str]) -> SkillEntry {
    let mut entry = SkillEntry::new(name, "main");
    entry.card = card.to_owned();
    let mut metadata = SkillMetadata::new(Kind::Method, "testing");
    metadata.intents = intents.iter().map(|intent| (*intent).to_owned()).collect();
    entry.metadata = Some(metadata);
    entry
}

fn needs(texts: &[&str]) -> FormTerms {
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

/// A candidate as its skill name and library, tier, needs, `why` and
/// questions: everything but the relevance, which is a float.
type Step = (String, String, Tier, Vec<Need>, String, Vec<Question>);

fn trace(index: &Index, candidates: &[Candidate]) -> Vec<Step> {
    candidates
        .iter()
        .map(|candidate| {
            let entry = &index.skills[candidate.skill];
            (
                entry.name.clone(),
                entry.library.clone(),
                candidate.tier,
                candidate.needs.clone(),
                candidate.why.clone(),
                candidate.questions.clone(),
            )
        })
        .collect()
}

fn keyword_question(skill: &str, need: Need) -> Vec<Question> {
    vec![Question::Keyword {
        skill: skill.to_owned(),
        need,
    }]
}

#[test]
fn name_match_ranks_above_card_match() {
    let index = Index {
        skills: vec![
            entry(
                "alpha-test-method",
                "measures latency carefully",
                &["write failing tests"],
            ),
            entry(
                "folder-layout-rules",
                "where files belong",
                &["split big modules"],
            ),
            entry(
                "latency-budget-rules",
                "write clear code",
                &["pick good names"],
            ),
            entry("review-branch-method", "read the diff", &["find real bugs"]),
        ],
    };
    let table = KeywordTable::build(&index).expect("build the keyword table");

    let ranked = table.rank("latency").expect("rank latency");

    let names: Vec<&str> = ranked
        .iter()
        .map(|ranked| index.skills[ranked.skill].name.as_str())
        .collect();
    assert_eq!(names, ["latency-budget-rules", "alpha-test-method"]);
}

#[test]
fn each_need_is_its_own_query() {
    let index = Index {
        skills: vec![
            entry("alpha-skill", "", &["alpha"]),
            entry("beta-skill", "", &["beta"]),
        ],
    };

    let found = lookup_c(&index, &needs(&["alpha", "beta"])).expect("lookup C");

    assert_eq!(
        trace(&index, &found.candidates),
        [
            (
                "alpha-skill".to_owned(),
                "main".to_owned(),
                Tier::C,
                vec![Need::Text(0)],
                "keyword: need 1".to_owned(),
                keyword_question("alpha-skill", Need::Text(0)),
            ),
            (
                "beta-skill".to_owned(),
                "main".to_owned(),
                Tier::C,
                vec![Need::Text(1)],
                "keyword: need 2".to_owned(),
                keyword_question("beta-skill", Need::Text(1)),
            ),
        ]
    );
    assert!(found.gaps.is_empty());
}

#[test]
fn fts5_syntax_in_a_need_is_plain_text() {
    let index = Index {
        skills: vec![
            entry("plain-skill", "foo bar baz", &["read a fault"]),
            entry("other-skill", "draw a chart", &["sum a column"]),
        ],
    };
    let need = r#"foo* AND (bar OR "baz") NEAR/2 -x ^y :z {a b}"#;

    let found = lookup_c(&index, &needs(&[need]));

    let found = found.expect("FTS5 syntax in a need must not reach the query syntax");
    assert_eq!(
        trace(&index, &found.candidates),
        [(
            "plain-skill".to_owned(),
            "main".to_owned(),
            Tier::C,
            vec![Need::Text(0)],
            "keyword: need 1".to_owned(),
            keyword_question("plain-skill", Need::Text(0)),
        )]
    );
}

#[test]
fn shadowed_copy_takes_no_place_of_the_five() {
    let mut skills: Vec<SkillEntry> = [
        "alpha-a", "alpha-b", "alpha-c", "alpha-d", "alpha-e", "alpha-f",
    ]
    .iter()
    .map(|name| entry(name, "alpha tools", &["use alpha"]))
    .collect();
    let mut shadowed = entry("alpha-a", "alpha tools", &["use alpha"]);
    shadowed.library = "other".to_owned();
    shadowed.library_priority = 1;
    shadowed.shadowed_by = Some("main".to_owned());
    skills.push(shadowed);
    let index = Index { skills };

    let found = lookup_c(&index, &needs(&["alpha"])).expect("lookup C");

    let kept: Vec<(String, String)> = trace(&index, &found.candidates)
        .into_iter()
        .map(|(name, library, ..)| (name, library))
        .collect();
    let expected: Vec<(String, String)> = [
        ("alpha-a", "main"),
        ("alpha-a", "other"),
        ("alpha-b", "main"),
        ("alpha-c", "main"),
        ("alpha-d", "main"),
        ("alpha-e", "main"),
    ]
    .iter()
    .map(|(name, library)| ((*name).to_owned(), (*library).to_owned()))
    .collect();
    assert_eq!(kept, expected);
    let served = found
        .candidates
        .iter()
        .filter(|candidate| index.is_served(candidate.skill))
        .count();
    assert_eq!(served, 5);
}
