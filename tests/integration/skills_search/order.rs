//! The order of a result: every tier A item before every tier B item and every tier B item before
//! every tier C item, each need covered once before any need repeats, on every page.

use std::path::PathBuf;

use serde_json::{Value, json};

use super::answer::{both_libraries, main_library, result, skills};

/// Two needs in tier A, two in tier B and three free-text needs, so that a
/// need with a second item meets needs that still wait for their first.
fn mixed_form() -> Value {
    json!({ "id": "O",
            "facts": { "writes_code": true, "languages": ["ts"] },
            "activity": "fix", "artefacts": ["unit-test"],
            "needs": ["write release notes for a new version", "find the cause of a bug",
                      "time a call to an outside api"] })
}

/// One need in tier B and two free-text needs that each find two skills.
fn keyword_form() -> Value {
    json!({ "id": "K", "activity": "fix",
            "needs": ["write a test for one function", "typescript"] })
}

fn tiers(items: &[Value]) -> Vec<String> {
    items
        .iter()
        .map(|item| item["tier"].as_str().expect("tier").to_owned())
        .collect()
}

fn named(items: &[Value]) -> Vec<String> {
    items
        .iter()
        .map(|item| item["skill"].as_str().expect("skill").to_owned())
        .collect()
}

/// Every item of every page, page by page, walking the cursor.
fn pages(libraries: &[PathBuf], form: &Value, limit: usize) -> Vec<Vec<Value>> {
    let mut pages = Vec::new();
    let mut cursor = Value::Null;
    loop {
        let page = result(
            libraries,
            &json!({ "requests": [form], "limit": limit, "cursor": cursor }),
        );
        pages.push(page["items"].as_array().expect("items").clone());
        cursor = page["cursor"].clone();
        if cursor.is_null() {
            return pages;
        }
    }
}

fn assert_tier_order(items: &[Value], context: &str) {
    let found = tiers(items);
    let mut sorted = found.clone();
    sorted.sort();
    assert_eq!(found, sorted, "{context}: {:?}", named(items));
}

#[test]
fn no_keyword_item_stands_before_a_declared_item_on_any_page() {
    for libraries in [vec![main_library()], both_libraries().to_vec()] {
        for form in [mixed_form(), keyword_form()] {
            for limit in [1, 3, 30] {
                let pages = pages(&libraries, &form, limit);
                let context = format!("{} at limit {limit} on {libraries:?}", form["id"]);
                for page in &pages {
                    assert_tier_order(page, &context);
                }
                let whole: Vec<Value> = pages.concat();
                assert_tier_order(&whole, &context);
                assert!(tiers(&whole).contains(&"C".to_owned()), "{context}");
            }
        }
    }
}

#[test]
fn an_unrelated_library_keeps_the_first_five_declared_items_in_place() {
    for form in [mixed_form(), keyword_form()] {
        let alone = result(&[main_library()], &json!({ "requests": [form] }));
        let with_unrelated = result(&both_libraries(), &json!({ "requests": [form] }));

        let ranked = |page: &Value, tier: &str| -> Vec<(usize, String)> {
            let items = page["items"].as_array().expect("items").iter();
            let in_tier = items.enumerate().filter(|(_, item)| item["tier"] == tier);
            let in_tier = in_tier.map(|(rank, item)| (rank + 1, item["skill"].to_string()));
            in_tier.take(5).collect()
        };
        for tier in ["A", "B"] {
            assert_eq!(
                ranked(&alone, tier),
                ranked(&with_unrelated, tier),
                "tier {tier} of {}",
                form["id"]
            );
        }
        let declared = ranked(&alone, "A").len() + ranked(&alone, "B").len();
        assert!(declared >= 2, "{}", form["id"]);
    }
}

#[test]
fn each_need_is_covered_once_before_any_need_repeats() {
    let mixed = result(
        &[main_library()],
        &json!({ "requests": [mixed_form()], "limit": 30 }),
    );
    let keyword = result(
        &[main_library()],
        &json!({ "requests": [keyword_form()], "limit": 30 }),
    );

    assert_eq!(
        skills(&mixed)[..7],
        [
            "fx-coding-rules",
            "fx-gap-ts",
            "fx-ts-errors",
            "fx-test-doubles",
            "fx-debugging",
            "fx-unit-tests",
            "fx-test-first"
        ]
    );
    let keyword_items: Vec<(&str, &str)> = keyword["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|item| {
            let skill = item["skill"].as_str().expect("skill");
            (skill, item["why"].as_str().expect("why"))
        })
        .collect();
    assert_eq!(
        keyword_items,
        [
            (
                "fx-debugging",
                "declared: activity fix; scope unknown: writes_code"
            ),
            (
                "fx-test-first",
                "declared: activity fix; scope unknown: writes_code"
            ),
            ("fx-unit-tests", "keyword: need 1"),
            ("fx-gap-ts", "keyword: need 2"),
            ("fx-test-doubles", "keyword: need 1"),
            ("fx-ts-errors", "keyword: need 2"),
            ("fx-coding-rules", "keyword: need 1"),
        ]
    );
}
