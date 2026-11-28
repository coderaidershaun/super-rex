//! `rex skills search` on the fixture libraries: what each lookup finds, gaps,
//! drops, filters and the order of the first page, end to end.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::{Value, json};
use super_rex::commands::skills::search::estimated_tokens;

use crate::skills_serve::Server;

pub(super) fn main_library() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/skills-search")
}

pub(super) fn both_libraries() -> [PathBuf; 2] {
    let main = main_library();
    let unrelated = main.join("unrelated");
    [main, unrelated]
}

/// `rex skills search` with `arguments` on stdin.
pub(super) fn run_search(libraries: &[PathBuf], arguments: &str) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rex"));
    command.args(["skills", "search"]);
    for library in libraries {
        command.arg("--library").arg(library);
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start rex skills search");
    let mut stdin = child.stdin.take().expect("stdin");
    stdin
        .write_all(arguments.as_bytes())
        .expect("write the arguments");
    drop(stdin);
    child.wait_with_output().expect("run rex skills search")
}

/// The answer of a search that must pass.
pub(super) fn answer(libraries: &[PathBuf], arguments: &Value) -> Value {
    let output = run_search(libraries, &arguments.to_string());
    assert!(
        output.status.success(),
        "rex skills search failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("the answer is JSON")
}

/// The one result of a search with one form.
pub(super) fn result(libraries: &[PathBuf], arguments: &Value) -> Value {
    answer(libraries, arguments)["results"][0].clone()
}

pub(super) fn field<'a>(result: &'a Value, key: &str) -> Vec<&'a str> {
    let items = result["items"].as_array().expect("items is a list");
    items
        .iter()
        .map(|item| item[key].as_str().unwrap_or(""))
        .collect()
}

pub(super) fn skills(result: &Value) -> Vec<&str> {
    field(result, "skill")
}

/// Form T1 of the latency example: a Rust matching loop.
pub(super) fn rust_loop() -> Value {
    json!({ "id": "T1",
            "facts": { "writes_code": true, "languages": ["rust"], "latency_scale": "ns_us" },
            "activity": "build", "touches": ["threads"], "must_have": ["latency"] })
}

/// Form T2 of the latency example: a Go service measured in milliseconds.
pub(super) fn go_service() -> Value {
    json!({ "id": "T2",
            "facts": { "writes_code": true, "languages": ["go"], "latency_scale": "ms" },
            "must_have": ["latency"] })
}

fn item<'a>(result: &'a Value, skill: &str) -> &'a Value {
    let items = result["items"].as_array().expect("items is a list");
    items
        .iter()
        .find(|item| item["skill"] == skill)
        .unwrap_or_else(|| panic!("{skill} is not an item"))
}

fn tier_a_and_b(result: &Value) -> Vec<Value> {
    let items = result["items"].as_array().expect("items is a list");
    let declared = items.iter().filter(|item| item["tier"] != "C");
    declared
        .map(|item| {
            json!([
                item["skill"],
                item["library"],
                item["tier"],
                item["slot"],
                item["why"]
            ])
        })
        .collect()
}

#[test]
fn latency_example_gives_the_skill_for_rust_and_a_gap_for_go() {
    let arguments = json!({ "requests": [rust_loop(), go_service()] });

    let answer = answer(&[main_library()], &arguments);

    let rust = &answer["results"][0];
    let go = &answer["results"][1];
    assert_eq!(rust["request"], "T1");
    assert_eq!(
        skills(rust),
        ["fx-coding-rules", "fx-latency", "fx-test-first"]
    );
    assert_eq!(field(rust, "slot"), ["rule", "quality:latency", "method"]);
    assert_eq!(go["request"], "T2");
    assert_eq!(skills(go), ["fx-coding-rules"]);
    let go_files: Vec<&Value> = item(go, "fx-coding-rules")["files"]
        .as_array()
        .expect("files")
        .iter()
        .map(|file| &file["path"])
        .collect();
    assert_eq!(go_files, [&json!("go.md")]);
    assert_eq!(
        go["gaps"],
        json!([
            { "need": "must_have:latency",
              "reason": "scope: languages go not in rust; latency_scale ms not in ns_us",
              "skill": "fx-latency", "library": "skills-search",
              "not_when": "Latency in milliseconds from a database or web call." },
            { "need": "languages:go", "reason": "no skill claims the language go" }
        ])
    );
    for page in [rust, go] {
        assert!(estimated_tokens(&page.to_string()) <= 2000);
    }
    let latency = item(rust, "fx-latency");
    assert_eq!(
        latency["why"],
        "declared: quality latency, touches threads; scope met"
    );
    for optional in ["placement", "sha256", "words", "files", "open_when"] {
        assert!(!latency[optional].is_null(), "{optional} was left out");
    }
}

#[test]
fn unrelated_library_changes_no_tier_a_or_b_item() {
    let design = json!({ "id": "F7", "facts": { "writes_code": true }, "activity": "design" });
    let arguments = json!({ "requests": [rust_loop(), design], "limit": 100 });

    let alone = answer(&[main_library()], &arguments);
    let with_unrelated = answer(&both_libraries(), &arguments);

    let shadowed = json!({ "skill": "fx-coding-rules", "library": "unrelated",
                           "reason": "shadowed by fx-coding-rules of library skills-search" });
    for form in 0..2 {
        let before = &alone["results"][form];
        let after = &with_unrelated["results"][form];
        assert_eq!(tier_a_and_b(before), tier_a_and_b(after));
        assert_eq!(before["dropped"], json!([]));
        assert_eq!(after["dropped"], json!([shadowed]));
    }
    assert_eq!(
        skills(&alone["results"][1]),
        ["fx-coding-rules", "fx-design-sketch", "fx-validation-plan"]
    );
}

#[test]
fn replaced_and_shadowed_skills_are_dropped_with_reasons() {
    let form = json!({ "id": "D", "facts": { "writes_code": true }, "artefacts": ["diagram"] });

    let result = result(&both_libraries(), &json!({ "requests": [form] }));

    assert_eq!(skills(&result), ["fx-coding-rules", "fx-diagrams"]);
    assert_eq!(
        result["dropped"],
        json!([
            { "skill": "fx-old-diagrams", "library": "skills-search",
              "reason": "replaced by fx-diagrams" },
            { "skill": "fx-coding-rules", "library": "unrelated",
              "reason": "shadowed by fx-coding-rules of library skills-search" }
        ])
    );
}

#[test]
fn unknown_words_are_listed_and_every_doubt_has_a_question() {
    let unknown = json!({ "id": "U", "must_have": ["p99"], "activity": "biuld",
                          "touches": ["concurency"],
                          "facts": { "frobnicate": 1, "languages": ["cobol"] },
                          "needs": ["write release notes"] });
    let design = json!({ "id": "S", "needs": ["sketch a module design before code"] });

    let output = run_search(
        &[main_library()],
        &json!({ "requests": [unknown] }).to_string(),
    );
    let design = result(&[main_library()], &json!({ "requests": [design] }));

    assert!(output.status.success());
    let unknown: Value = serde_json::from_slice(&output.stdout).expect("JSON");
    let unknown = &unknown["results"][0];
    assert_eq!(
        unknown["unrecognised"],
        json!([
            "facts:frobnicate",
            "languages:cobol",
            "activity:biuld",
            "touches:concurency"
        ])
    );
    assert_eq!(
        item(unknown, "fx-latency")["confirm"],
        "Does the task have writes_code true and languages rust and latency_scale ns_us?"
    );
    let items = unknown["items"].as_array().expect("items");
    let keyword_items: Vec<&Value> = items.iter().filter(|item| item["tier"] == "C").collect();
    assert!(!keyword_items.is_empty());
    for keyword_item in keyword_items {
        assert!(
            keyword_item["confirm"].is_string(),
            "{keyword_item} has no confirm"
        );
    }
    let design_items: Vec<Value> = design["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|item| json!([item["skill"], item["tier"], item["why"], item["confirm"]]))
        .collect();
    assert_eq!(
        design_items,
        [
            json!([
                "fx-design-sketch",
                "C",
                "keyword: need 1",
                "Is fx-design-sketch right for need 1?"
            ]),
            json!([
                "fx-validation-plan",
                "C",
                "required by fx-design-sketch",
                "As for fx-design-sketch."
            ]),
        ]
    );
}

#[test]
fn kinds_and_categories_leave_out_other_items() {
    let search = |filter: Value| {
        let mut arguments = json!({ "requests": [rust_loop()] });
        arguments
            .as_object_mut()
            .expect("object")
            .extend(filter.as_object().expect("object").clone());
        result(&[main_library()], &arguments)
    };

    let methods = search(json!({ "kinds": ["method"] }));
    let performance = search(json!({ "categories": ["performance"] }));
    let widgets = search(json!({ "kinds": ["widget"] }));
    let design = result(
        &[main_library()],
        &json!({ "requests": [{ "id": "R", "activity": "design" }], "kinds": ["rule"] }),
    );

    assert_eq!(skills(&methods), ["fx-test-first"]);
    assert_eq!(skills(&performance), ["fx-latency"]);
    assert_eq!(skills(&widgets), Vec::<&str>::new());
    let unrecognised = widgets["unrecognised"].as_array().expect("list");
    assert_eq!(unrecognised.last(), Some(&json!("kinds:widget")));
    assert_eq!(skills(&design), Vec::<&str>::new());
    let methods_by_design = result(
        &[main_library()],
        &json!({ "requests": [{ "id": "M", "activity": "design" }], "kinds": ["method"] }),
    );
    assert_eq!(
        skills(&methods_by_design),
        ["fx-design-sketch", "fx-validation-plan"]
    );
    assert_eq!(design["more"], 0);
}

#[test]
fn first_page_covers_each_need_before_a_second_item() {
    let form = json!({ "id": "N",
                       "needs": ["write release notes for a new version", "draw a sequence diagram"] });

    let result = result(
        &[main_library()],
        &json!({ "requests": [form], "limit": 2 }),
    );

    assert_eq!(skills(&result), ["fx-release-notes", "fx-diagrams"]);
    assert_eq!(
        field(&result, "why"),
        ["keyword: need 1", "keyword: need 2"]
    );
}

#[test]
fn equal_ranks_come_in_name_order() {
    let form = json!({ "id": "F", "facts": { "writes_code": true }, "activity": "fix" });

    let result = result(&[main_library()], &json!({ "requests": [form] }));

    assert_eq!(
        skills(&result),
        ["fx-coding-rules", "fx-debugging", "fx-test-first"]
    );
}

#[test]
fn a_scope_question_is_asked_once_per_page() {
    let form = json!({ "id": "Q", "activity": "fix" });

    let result = result(&[main_library()], &json!({ "requests": [form] }));

    assert_eq!(skills(&result), ["fx-debugging", "fx-test-first"]);
    assert_eq!(
        field(&result, "confirm"),
        [
            "Does the task have writes_code true?",
            "As for fx-debugging."
        ]
    );
}

#[test]
fn bad_arguments_name_the_field_at_fault() {
    let two_forms = json!([{ "id": "T1" }, { "id": "T2" }]);
    let cases = [
        (json!({ "requests": [] }), "requests"),
        (
            json!({ "requests": [{ "id": "T1" }], "limits": 5 }),
            "limits",
        ),
        (
            json!({ "requests": [{ "id": "T1" }, { "id": "T1" }] }),
            "id",
        ),
        (json!({ "requests": [{ "id": "" }] }), "id"),
        (
            json!({ "requests": [{ "id": "T1", "needs": ["a", "b", "c", "d"] }] }),
            "needs",
        ),
        (json!({ "requests": [{ "id": "T1" }], "limit": 0 }), "limit"),
        (
            json!({ "requests": [{ "id": "T1" }], "token_budget": 0 }),
            "token_budget",
        ),
        (
            json!({ "requests": two_forms, "cursor": "c1.2.0000000000000000.0000000000000000" }),
            "cursor",
        ),
    ];
    let mut server = Server::start(&[&main_library()]);
    server.initialize();

    for (arguments, named) in cases {
        let output = run_search(&[main_library()], &arguments.to_string());
        let (is_error, tool) = server.call("search", &arguments);

        let stderr = String::from_utf8_lossy(&output.stderr);
        let lines: Vec<&str> = stderr.lines().collect();
        assert_eq!(output.status.code(), Some(1), "{arguments}");
        assert!(output.stdout.is_empty(), "{arguments} printed a result");
        assert_eq!(lines.len(), 1, "{arguments}: {stderr}");
        assert!(lines[0].starts_with("error: "), "{stderr}");
        assert!(
            lines[0].contains(&format!("`{named}`")),
            "{named} not in {stderr}"
        );
        assert!(is_error, "{arguments}");
        assert_eq!(tool["code"], "bad_arguments", "{arguments}");
    }
}

/// Write one skill whose header holds `name`, `description` and the
/// `metadata` lines given, each indented by two spaces under `metadata:`.
fn write_skill(library: &Path, name: &str, description: &str, metadata: &[&str]) {
    let folder = library.join(name);
    std::fs::create_dir_all(&folder).expect("make the skill folder");
    let mut header = format!("---\nname: {name}\ndescription: {description}\n");
    header.push_str(&format!("when_to_use: {description}\n"));
    if !metadata.is_empty() {
        header.push_str("metadata:\n");
        for line in metadata {
            header.push_str(&format!("  {line}\n"));
        }
    }
    header.push_str("---\n# Body\n\nText.\n");
    std::fs::write(folder.join("SKILL.md"), header).expect("write SKILL.md");
}

fn claiming(kind: &str, claims: &str, requires: &str) -> Vec<String> {
    let mut lines = vec![format!("kind: {kind}"), "category: design".to_owned()];
    lines.push(format!("claims: {claims}"));
    if !requires.is_empty() {
        lines.push(format!("requires: [{requires}]"));
    }
    lines
}

fn write_claiming(library: &Path, name: &str, kind: &str, claims: &str, requires: &str) {
    let lines = claiming(kind, claims, requires);
    let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
    write_skill(library, name, &format!("The {name} skill."), &lines);
}

#[test]
fn a_required_skill_names_the_first_requirer_that_a_filter_keeps() {
    let temp = tempfile::tempdir().expect("temp folder");
    let library = temp.path().join("lib2");
    write_claiming(
        &library,
        "fx-design-sketch",
        "method",
        "{ activity: [design] }",
        "fx-plan",
    );
    write_claiming(
        &library,
        "fx-zrule",
        "rule",
        "{ activity: [design] }",
        "fx-plan",
    );
    write_claiming(&library, "fx-plan", "rule", "{}", "");
    let arguments = json!({ "requests": [{ "id": "R", "activity": "design" }], "kinds": ["rule"] });

    let result = result(&[library], &arguments);

    assert_eq!(skills(&result), ["fx-zrule", "fx-plan"]);
    assert_eq!(field(&result, "why")[1], "required by fx-zrule");
}

#[test]
fn an_open_when_item_that_a_topic_file_repeats_is_listed_once() {
    let form = json!({ "id": "W", "facts": { "writes_code": true }, "artefacts": ["unit-test"] });

    let result = result(&[main_library()], &json!({ "requests": [form] }));

    assert_eq!(
        item(&result, "fx-unit-tests")["open_when"],
        json!([{ "file": "shared-setup.md", "when": "you share setup between tests" }])
    );
}
