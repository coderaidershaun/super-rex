//! The two measurement options of `rex skills search`: `--keyword-ratio` sets the share of the
//! best keyword score that a candidate must reach, and `--keyword-scores` prints the scores.

use std::io::Write;
use std::process::{Command, Output, Stdio};

use serde_json::{Value, json};

use super::answer::main_library;

/// `rex skills search` on the fixture library with `options` and `arguments`.
fn search_with(options: &[&str], arguments: &Value) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_rex"))
        .args(["skills", "search", "--library"])
        .arg(main_library())
        .args(options)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start rex skills search");
    let mut stdin = child.stdin.take().expect("stdin");
    stdin
        .write_all(arguments.to_string().as_bytes())
        .expect("write the arguments");
    drop(stdin);
    child.wait_with_output().expect("run rex skills search")
}

fn first_result(options: &[&str], arguments: &Value) -> Value {
    let output = search_with(options, arguments);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let answer: Value = serde_json::from_slice(&output.stdout).expect("JSON");
    answer["results"][0].clone()
}

fn keyword_request() -> Value {
    json!({ "requests": [{ "id": "K", "facts": { "writes_code": true }, "activity": "fix",
                           "needs": ["write a test for one function"] }],
            "limit": 30 })
}

fn keyword_items(result: &Value) -> Vec<Value> {
    let items = result["items"].as_array().expect("items");
    items
        .iter()
        .filter(|item| item["tier"] == "C")
        .cloned()
        .collect()
}

#[test]
fn the_ratio_option_changes_how_many_candidates_a_need_keeps() {
    let built_in = keyword_items(&first_result(&[], &keyword_request()));
    let strict = keyword_items(&first_result(&["--keyword-ratio", "1"], &keyword_request()));
    let loose = keyword_items(&first_result(
        &["--keyword-ratio", "0.01"],
        &keyword_request(),
    ));

    assert!(strict.len() < built_in.len(), "{strict:?} {built_in:?}");
    assert!(loose.len() > built_in.len(), "{loose:?} {built_in:?}");
    assert!(loose.len() <= 5);
}

#[test]
fn the_scores_option_prints_each_keyword_score_and_the_best_of_its_need() {
    let result = first_result(&["--keyword-scores"], &keyword_request());

    let keyword = keyword_items(&result);
    assert!(keyword.len() >= 2, "{result}");
    let best = keyword[0]["best_score"].as_f64().expect("best_score");
    for item in &keyword {
        let score = item["score"].as_f64().expect("score");
        assert_eq!(item["best_score"].as_f64(), Some(best), "{item}");
        assert!(score <= best && score >= 0.5 * best, "{item}");
    }
    assert_eq!(keyword[0]["score"].as_f64(), Some(best));
    let items = result["items"].as_array().expect("items");
    for declared in items.iter().filter(|item| item["tier"] != "C") {
        assert!(declared.get("score").is_none(), "{declared}");
        assert!(declared.get("best_score").is_none(), "{declared}");
    }
}

#[test]
fn without_the_options_the_answer_is_unchanged_and_has_no_scores() {
    let arguments = json!({ "requests": [keyword_request()["requests"][0].clone()], "limit": 2 });

    let plain = search_with(&[], &arguments);
    let built_in_ratio = search_with(&["--keyword-ratio", "0.5"], &arguments);

    assert!(plain.status.success());
    assert_eq!(plain.stdout, built_in_ratio.stdout);
    let text = String::from_utf8(plain.stdout).expect("UTF-8");
    assert!(!text.contains("score"), "{text}");
}

#[test]
fn a_cursor_made_with_another_ratio_is_bad() {
    let arguments = json!({ "requests": [keyword_request()["requests"][0].clone()], "limit": 1 });
    let first = first_result(&[], &arguments);
    let mut next = arguments.clone();
    next["cursor"] = first["cursor"].clone();

    let output = search_with(&["--keyword-ratio", "0.3"], &next);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr.starts_with("error: the cursor cannot be read"),
        "{stderr}"
    );
}

#[test]
fn a_ratio_outside_zero_to_one_is_refused() {
    for ratio in ["0", "1.5", "-0.2", "NaN"] {
        let option = format!("--keyword-ratio={ratio}");
        let output = search_with(&[&option], &keyword_request());

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(1), "{ratio}: {stderr}");
        assert!(output.stdout.is_empty(), "{ratio}");
        assert!(stderr.starts_with("error: "), "{stderr}");
        assert!(stderr.contains("`--keyword-ratio`"), "{ratio}: {stderr}");
    }
}
