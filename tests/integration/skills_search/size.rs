//! The size of a search result: the token budget of a page, the steps that shorten a page over
//! it, the items that move to the next page, and the field that says a page is still over it.

use serde_json::{Value, json};
use super_rex::commands::skills::search::estimated_tokens;

use super::answer::{go_service, main_library, result, run_search, rust_loop, skills};

const TOKENS_PER_ITEM: usize = 200;

/// Every term of each group of the vocabulary that the binary embeds.
fn terms_of(group: &str) -> Vec<String> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugin/vocab/terms.json");
    let text = std::fs::read_to_string(path).expect("read the vocabulary");
    let vocabulary: Value = serde_json::from_str(&text).expect("the vocabulary is JSON");
    let terms = vocabulary[group]["terms"].as_array().expect("terms");
    terms
        .iter()
        .map(|term| term["term"].as_str().expect("term").to_owned())
        .collect()
}

/// The form of the size probe that gate A named `probe-nocode-all`: a task
/// that writes no code and asks for every term, so most terms become gaps.
fn no_code_probe() -> Value {
    json!({ "id": "probe-nocode-all", "facts": { "writes_code": false }, "activity": "build",
            "touches": terms_of("touches"), "must_have": terms_of("qualities"),
            "artefacts": terms_of("artefacts") })
}

fn items(result: &Value) -> &Vec<Value> {
    result["items"].as_array().expect("items")
}

fn tokens(result: &Value) -> usize {
    estimated_tokens(&result.to_string())
}

/// Every item of every page at `budget`, walking the cursor.
fn all_pages(form: &Value, budget: usize) -> Vec<Value> {
    let mut pages = Vec::new();
    let mut cursor = Value::Null;
    loop {
        let page = result(
            &[main_library()],
            &json!({ "requests": [form], "token_budget": budget, "cursor": cursor }),
        );
        cursor = page["cursor"].clone();
        pages.push(page);
        if cursor.is_null() {
            return pages;
        }
    }
}

#[test]
fn the_size_probe_keeps_every_item_within_its_budget() {
    let probe = result(&[main_library()], &json!({ "requests": [no_code_probe()] }));

    assert!(items(&probe).len() >= 2, "{probe}");
    assert_eq!(probe["more"], 0, "{probe}");
    assert!(probe.get("over_budget").is_none(), "{probe}");
    assert!(
        tokens(&probe) <= TOKENS_PER_ITEM * 10,
        "{} tokens",
        tokens(&probe)
    );
    let gaps = probe["gaps"].as_array().expect("gaps");
    assert!(gaps.len() > 20, "{} gaps", gaps.len());
}

#[test]
fn a_smaller_budget_moves_items_to_the_next_page_and_loses_none() {
    let form = json!({ "id": "S", "facts": { "writes_code": true, "languages": ["rust", "ts"],
                       "latency_scale": "ns_us" }, "activity": "fix", "must_have": ["latency"],
                       "artefacts": ["unit-test", "diagram"] });
    let whole = result(
        &[main_library()],
        &json!({ "requests": [form], "limit": 100, "token_budget": 100_000 }),
    );

    let pages = all_pages(&form, 600);

    assert!(pages.len() > 1, "one page held everything");
    let mut seen: Vec<&str> = Vec::new();
    for page in &pages {
        assert!(tokens(page) <= 600, "{} tokens: {page}", tokens(page));
        assert!(page.get("over_budget").is_none());
        seen.extend(skills(page));
    }
    assert_eq!(seen, skills(&whole));
    let first = &pages[0];
    assert_eq!(
        first["more"].as_u64().expect("more") as usize + items(first).len(),
        items(&whole).len()
    );
    let by_skill = |page: &Value, skill: &str| {
        items(page)
            .iter()
            .find(|item| item["skill"] == skill)
            .cloned()
    };
    for page in &pages {
        for item in items(page) {
            let skill = item["skill"].as_str().expect("skill");
            let full = by_skill(&whole, skill).expect("in the whole list");
            assert_eq!(
                item["files"].as_array().map(Vec::len),
                full["files"].as_array().map(Vec::len)
            );
            assert_eq!(item["open_when"], full["open_when"], "{skill}");
        }
    }
}

#[test]
fn an_item_larger_than_the_budget_says_so() {
    let page = result(
        &[main_library()],
        &json!({ "requests": [rust_loop()], "token_budget": 40 }),
    );

    assert_eq!(skills(&page), ["fx-coding-rules"]);
    assert_eq!(page["over_budget"], true);
    assert_eq!(page["more"], 2);
    assert!(page["cursor"].is_string());
    let item = &items(&page)[0];
    assert_eq!(item["files"][0]["path"], "rust.md");
    assert!(item["open_when"].is_array());
    let fitting = result(&[main_library()], &json!({ "requests": [rust_loop()] }));
    assert!(fitting.get("over_budget").is_none(), "{fitting}");
}

#[test]
fn a_page_over_budget_cuts_each_reason_to_its_head() {
    let full = result(&[main_library()], &json!({ "requests": [go_service()] }));
    let page = result(
        &[main_library()],
        &json!({ "requests": [go_service()], "token_budget": 120 }),
    );

    assert_eq!(
        full["gaps"][0]["reason"],
        "scope: languages go not in rust; latency_scale ms not in ns_us"
    );
    assert_eq!(page["gaps"][0]["reason"], "scope: languages go not in rust");
    assert_eq!(page["gaps"][0]["not_when"], full["gaps"][0]["not_when"]);
}

#[test]
fn keep_hashes_holds_every_hash_whatever_the_size() {
    let form = json!({ "id": "H", "facts": { "writes_code": true, "languages": ["rust"],
                       "latency_scale": "ns_us" }, "must_have": ["latency"],
                       "needs": ["measure a hot path first"] });
    let shortened = result(
        &[main_library()],
        &json!({ "requests": [form], "token_budget": 700 }),
    );
    let kept = result(
        &[main_library()],
        &json!({ "requests": [form], "token_budget": 700, "keep_hashes": true }),
    );

    assert!(
        items(&shortened)
            .iter()
            .all(|item| item["sha256"].is_null())
    );
    assert!(!items(&kept).is_empty());
    for item in items(&kept) {
        assert!(item["sha256"].is_string(), "{item}");
        for file in item["files"].as_array().into_iter().flatten() {
            assert!(file["sha256"].is_string(), "{file}");
        }
    }
    let latency = items(&kept)
        .iter()
        .find(|item| item["skill"] == "fx-latency")
        .expect("fx-latency is on the page");
    assert!(!latency["files"].as_array().expect("files").is_empty());
}

#[test]
fn a_budget_of_zero_is_refused() {
    let arguments = json!({ "requests": [rust_loop()], "token_budget": 0 });

    let output = run_search(&[main_library()], &arguments.to_string());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr.contains("`token_budget`"), "{stderr}");
}
