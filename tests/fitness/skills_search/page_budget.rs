//! A search page keeps to its token budget: ten items fit with their files and open-when items,
//! the hashes stay when asked, and a page over its budget moves its last items to the next page.

use serde_json::{Value, json};
use super_rex::commands::skills::search::{
    FileEntry, FileRole, Index, Kind, OpenWhen, SkillEntry, SkillMetadata, estimated_tokens,
};

use super::answer_stability::{answer_of, main_library, skills, ten_item_form};

/// The default budget of a page is 200 tokens for each item of `limit`.
const BUDGET_OF_TEN: usize = 2000;
/// The fields that the size limit may leave out. `files` and `open_when` are
/// never among them.
const OPTIONAL_FIELDS: [&str; 3] = ["placement", "sha256", "words"];

/// The files and open-when items of each item, by skill.
fn pointers(result: &Value) -> Vec<Value> {
    let items = result["items"].as_array().expect("items");
    let pointers = items.iter().map(|item| {
        let files = item["files"].as_array().into_iter().flatten();
        let paths: Vec<&Value> = files.map(|file| &file["path"]).collect();
        json!([item["skill"], paths, item["open_when"]])
    });
    pointers.collect()
}

#[test]
fn ten_items_fit_the_token_budget() {
    let index = Index::from_folders(&[main_library()]).expect("load the fixture");
    let full = answer_of(
        &index,
        json!({ "requests": [ten_item_form()], "limit": 10, "token_budget": 100_000 }),
    );

    let answer = answer_of(
        &index,
        json!({ "requests": [ten_item_form()], "limit": 10 }),
    );

    let result = &answer["results"][0];
    let by_tier = |tier: &str| -> Vec<String> {
        let items = result["items"].as_array().expect("items");
        let in_tier = items.iter().filter(|item| item["tier"] == tier);
        in_tier
            .map(|item| item["skill"].as_str().expect("skill").to_owned())
            .collect()
    };
    assert_eq!(
        by_tier("A"),
        [
            "fx-coding-rules",
            "fx-gap-ts",
            "fx-proptest",
            "fx-ts-errors"
        ]
    );
    assert_eq!(
        by_tier("B"),
        [
            "fx-latency",
            "fx-diagrams",
            "fx-validation-plan",
            "fx-test-first"
        ]
    );
    assert_eq!(by_tier("C"), ["fx-release-notes", "fx-design-sketch"]);
    assert_eq!(result["more"], 0);
    assert_eq!(result["cursor"], Value::Null);
    assert!(result.get("over_budget").is_none());
    let tokens = estimated_tokens(&result.to_string());
    assert!(tokens <= BUDGET_OF_TEN, "{tokens} tokens");
    let full_tokens = estimated_tokens(&full["results"][0].to_string());
    assert!(
        full_tokens > BUDGET_OF_TEN,
        "the full page fits: {full_tokens}"
    );
    assert_eq!(pointers(result), pointers(&full["results"][0]));
    let held = pointers(result);
    let with_files = held.iter().filter(|item| item[1] != json!([]));
    assert_eq!(
        with_files.count(),
        2,
        "fx-coding-rules and fx-latency have files"
    );
    for item in result["items"].as_array().expect("items") {
        for required in ["skill", "library", "slot", "tier", "why", "card"] {
            assert!(item[required].is_string(), "{required} missing in {item}");
        }
    }
}

#[test]
fn keep_hashes_holds_every_hash_of_ten_items() {
    let index = Index::from_folders(&[main_library()]).expect("load the fixture");

    let answer = answer_of(
        &index,
        json!({ "requests": [ten_item_form()], "limit": 10, "keep_hashes": true }),
    );

    let result = &answer["results"][0];
    let items = result["items"].as_array().expect("items");
    let more = result["more"].as_u64().expect("more");
    assert_eq!(items.len() + usize::try_from(more).expect("small"), 10);
    for item in items {
        assert!(item["sha256"].is_string(), "{item}");
        for file in item["files"].as_array().into_iter().flatten() {
            assert!(file["sha256"].is_string(), "{file}");
        }
    }
    let over = result.get("over_budget").is_some();
    assert!(over || estimated_tokens(&result.to_string()) <= BUDGET_OF_TEN);
}

fn building_skill(number: usize) -> SkillEntry {
    let name = format!("skill-{number:02}");
    let mut metadata = SkillMetadata::new(Kind::Method, "testing");
    metadata.claims.activity = vec!["build".to_owned()];
    metadata.not_when = Some("Work that writes no code at all.".to_owned());
    metadata.open_when = vec![OpenWhen {
        skill: Some("skill-01".to_owned()),
        file: None,
        when: "the first test of a change".to_owned(),
    }];
    let mut entry = SkillEntry::new(&name, "main");
    let words: Vec<String> = (1..=30).map(|word| format!("word{word}")).collect();
    entry.card = words.join(" ");
    entry.sha256 = format!("{:064x}", number);
    entry.words = 300;
    entry.files = vec![FileEntry {
        path: "topic.md".to_owned(),
        sha256: format!("{:064x}", number + 100),
        role: Some(FileRole::Topic),
        when: Some("a topic case".to_owned()),
        ..FileEntry::default()
    }];
    entry.metadata = Some(metadata);
    entry
}

#[test]
fn a_page_over_budget_carries_its_last_items_over() {
    let index = Index {
        skills: (1..=12).map(building_skill).collect(),
    };
    let form = json!({ "id": "B", "activity": "build" });

    let first = answer_of(
        &index,
        json!({ "requests": [form], "limit": 12, "token_budget": 1000 }),
    );

    let page = &first["results"][0];
    let held = skills(page);
    let moved = 12 - held.len();
    assert!(held.len() < 12, "the page kept all 12 items");
    assert_eq!(page["more"], moved);
    let cursor = page["cursor"].as_str().expect("a cursor");
    let offset: usize = cursor
        .split('.')
        .nth(1)
        .expect("offset")
        .parse()
        .expect("number");
    assert_eq!(offset, held.len());
    assert!(estimated_tokens(&page.to_string()) <= 1000);
    for item in page["items"].as_array().expect("items") {
        for optional in OPTIONAL_FIELDS {
            assert!(item[optional].is_null(), "{optional} left in {item}");
        }
        assert_eq!(item["open_when"][0]["skill"], "skill-01");
        assert_eq!(item["open_when"][1]["file"], "topic.md");
    }

    let next = answer_of(
        &index,
        json!({ "requests": [form], "limit": 12, "token_budget": 1000, "cursor": cursor }),
    );

    let first_moved = format!("skill-{:02}", held.len() + 1);
    assert_eq!(skills(&next["results"][0])[0], first_moved);
}
