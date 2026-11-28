//! The answer of a search is stable: the same arguments give the same bytes, every card is short,
//! and keyword results never reorder what rules and claims found.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{Value, json};
use super_rex::commands::skills::search::{
    FormTerms, Index, IndexVersion, Kind, RequestForm, SearchRequest, SkillEntry, SkillMetadata,
    lookup_c, search,
};

pub(super) fn main_library() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/skills-search")
}

fn request(arguments: Value) -> SearchRequest {
    serde_json::from_value(arguments).expect("valid search arguments")
}

/// The JSON of the answer that the library API gives for `arguments`.
pub(super) fn answer_of(index: &Index, arguments: Value) -> Value {
    let version = IndexVersion::of(index, None);
    let answer = search(index, &version, &request(arguments)).expect("the search passes");
    serde_json::to_value(answer).expect("JSON")
}

pub(super) fn skills(result: &Value) -> Vec<String> {
    let items = result["items"].as_array().expect("items");
    items
        .iter()
        .map(|item| item["skill"].as_str().expect("skill").to_owned())
        .collect()
}

fn rust_loop() -> Value {
    json!({ "id": "T1",
            "facts": { "writes_code": true, "languages": ["rust"], "latency_scale": "ns_us" },
            "activity": "build", "touches": ["threads"], "must_have": ["latency"] })
}

fn go_service() -> Value {
    json!({ "id": "T2",
            "facts": { "writes_code": true, "languages": ["go"], "latency_scale": "ms" },
            "must_have": ["latency"] })
}

pub(super) fn ten_item_form() -> Value {
    json!({ "id": "ALL",
            "facts": { "writes_code": true, "languages": ["rust", "ts"],
                       "repo_dependencies": ["proptest"], "latency_scale": "ns_us" },
            "activity": "build", "touches": ["threads"], "must_have": ["latency"],
            "artefacts": ["diagram", "plan"],
            "needs": ["write release notes for a new version",
                      "sketch a module design before code"] })
}

fn cli_bytes(arguments: &Value) -> Vec<u8> {
    let main = main_library();
    let mut child = Command::new(env!("CARGO_BIN_EXE_rex"))
        .args(["skills", "search", "--library"])
        .arg(&main)
        .arg("--library")
        .arg(main.join("unrelated"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("start rex skills search");
    let mut stdin = child.stdin.take().expect("stdin");
    stdin
        .write_all(arguments.to_string().as_bytes())
        .expect("write the arguments");
    drop(stdin);
    let output = child.wait_with_output().expect("run rex skills search");
    assert!(output.status.success());
    output.stdout
}

#[test]
fn same_arguments_give_a_byte_identical_answer() {
    let arguments = json!({ "requests": [rust_loop(), go_service()] });

    let runs: Vec<Vec<u8>> = (0..3).map(|_| cli_bytes(&arguments)).collect();

    assert!(!runs[0].is_empty());
    assert!(runs[0] == runs[1] && runs[1] == runs[2], "the runs differ");
}

#[test]
fn every_card_has_at_most_thirty_words() {
    let main = Index::from_folders(&[main_library()]).expect("load the fixture");
    let both = Index::from_folders(&[main_library(), main_library().join("unrelated")])
        .expect("load both fixtures");
    let forms = [
        rust_loop(),
        go_service(),
        ten_item_form(),
        json!({ "id": "D", "facts": { "writes_code": true }, "activity": "design" }),
        json!({ "id": "G", "facts": { "writes_code": true }, "artefacts": ["diagram"] }),
        json!({ "id": "U", "must_have": ["p99"], "activity": "biuld", "touches": ["concurency"],
                "facts": { "frobnicate": 1, "languages": ["golang"] },
                "needs": ["write release notes"] }),
        json!({ "id": "S", "needs": ["sketch a module design before code"] }),
        json!({ "id": "N", "needs": ["write release notes for a new version",
                                     "draw a sequence diagram"] }),
        json!({ "id": "F", "facts": { "writes_code": true }, "activity": "fix" }),
    ];

    let mut cards = 0;
    for index in [&main, &both] {
        let answer = answer_of(index, json!({ "requests": forms, "limit": 100 }));
        for result in answer["results"].as_array().expect("results") {
            for item in result["items"].as_array().expect("items") {
                let card = item["card"].as_str().expect("card");
                assert!(card.split_whitespace().count() <= 30, "{card}");
                cards += 1;
            }
        }
    }

    assert!(cards > 20, "only {cards} cards were checked");
}

fn fixing_skill(name: &str, intents: &[&str]) -> SkillEntry {
    let mut metadata = SkillMetadata::new(Kind::Method, "testing");
    metadata.claims.activity = vec!["fix".to_owned()];
    metadata.intents = intents.iter().map(|intent| (*intent).to_owned()).collect();
    let mut entry = SkillEntry::new(name, "main");
    entry.card = format!("{name} finds and fixes faults.");
    entry.metadata = Some(metadata);
    entry
}

fn declared_items(answer: &Value) -> Vec<Value> {
    let items = answer["results"][0]["items"].as_array().expect("items");
    let declared = items.iter().filter(|item| item["tier"] != "C");
    declared
        .map(|item| json!([item["skill"], item["tier"], item["slot"], item["why"]]))
        .collect()
}

#[test]
fn keyword_results_never_reorder_tier_a_or_b_items() {
    let arguments = json!({ "requests": [{ "id": "K", "activity": "fix", "needs": ["zebra"] }] });
    let small = Index {
        skills: vec![
            fixing_skill("x-first", &["fix a failing test"]),
            fixing_skill(
                "y-second",
                &[
                    "fix the faults that a long test run finds in the code of a service",
                    "fix the faults of a parser, a queue, a cache or a pool of threads",
                    "fix the faults of a build that breaks on one machine and not on another",
                    "fix a fault named after the zebra crossing of two old roads",
                ],
            ),
        ],
    };
    let mut large = small.clone();
    for name in ["zebra-one", "zebra-two", "zebra-three"] {
        let mut entry = SkillEntry::new(name, "main");
        entry.card = "zebra".to_owned();
        entry.when_to_use = "zebra".to_owned();
        large.skills.push(entry);
    }
    let form: RequestForm = serde_json::from_value(arguments["requests"][0].clone()).expect("form");
    let terms = FormTerms::read(&form);
    let keyword_names = |index: &Index| -> Vec<String> {
        let found = lookup_c(index, &terms).expect("lookup C");
        let skills = found.candidates.iter();
        skills
            .map(|candidate| index.skills[candidate.skill].name.clone())
            .collect()
    };

    let before = answer_of(&small, arguments.clone());
    let after = answer_of(&large, arguments);

    assert!(keyword_names(&small).contains(&"y-second".to_owned()));
    assert!(!keyword_names(&large).contains(&"y-second".to_owned()));
    let expected = [
        json!(["x-first", "B", "method", "declared: activity fix"]),
        json!(["y-second", "B", "method", "declared: activity fix"]),
    ];
    assert_eq!(declared_items(&before), expected);
    assert_eq!(declared_items(&after), expected);
}

fn artefact_skill(name: &str, artefact: &str) -> SkillEntry {
    let mut metadata = SkillMetadata::new(Kind::Tool, "diagrams");
    metadata.claims.artefact = vec![artefact.to_owned()];
    let mut entry = SkillEntry::new(name, "main");
    entry.card = format!("{name} draws one {artefact}.");
    entry.metadata = Some(metadata);
    entry
}

#[test]
fn a_skill_that_lookup_c_finds_never_pulls_a_declared_skill_forward() {
    let arguments = json!({ "requests": [{ "id": "R", "artefacts": ["diagram", "plan"],
                                          "needs": ["zebra"] }], "limit": 20 });
    let mut sketch = fixing_skill(
        "c-sketch",
        &[
            "fix the faults that a long test run finds in the code of a service",
            "fix the faults of a parser, a queue, a cache or a pool of threads",
            "fix the faults of a build that breaks on one machine and not on another",
            "fix a fault named after the zebra crossing of two old roads",
        ],
    );
    if let Some(metadata) = sketch.metadata.as_mut() {
        metadata.claims.activity.clear();
        metadata.requires = vec!["p-two".to_owned()];
    }
    let small = Index {
        skills: vec![
            sketch,
            artefact_skill("d-one", "diagram"),
            artefact_skill("d-two", "diagram"),
            artefact_skill("p-one", "plan"),
            artefact_skill("p-two", "plan"),
        ],
    };
    let mut large = small.clone();
    for name in ["zebra-one", "zebra-two", "zebra-three"] {
        let mut entry = SkillEntry::new(name, "other");
        entry.library_priority = 1;
        entry.card = "zebra".to_owned();
        entry.when_to_use = "zebra".to_owned();
        large.skills.push(entry);
    }
    let form: RequestForm = serde_json::from_value(arguments["requests"][0].clone()).expect("form");
    let terms = FormTerms::read(&form);
    let finds_sketch = |index: &Index| {
        let found = lookup_c(index, &terms).expect("lookup C");
        let mut skills = found.candidates.iter();
        skills.any(|candidate| index.skills[candidate.skill].name == "c-sketch")
    };

    let before = answer_of(&small, arguments.clone());
    let after = answer_of(&large, arguments);

    assert!(finds_sketch(&small));
    assert!(!finds_sketch(&large));
    let names = |answer: &Value| -> Vec<Value> {
        declared_items(answer)
            .iter()
            .map(|item| item[0].clone())
            .collect()
    };
    assert_eq!(names(&before), ["d-one", "p-one", "d-two", "p-two"]);
    assert_eq!(names(&after), ["d-one", "p-one", "d-two", "p-two"]);
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
fn a_skill_required_by_a_declared_and_a_keyword_skill_keeps_the_declared_order() {
    let temp = tempfile::tempdir().expect("temp folder");
    let library = temp.path().join("lib5");
    let sketch = claiming("method", "{}", "fx-r");
    let mut sketch: Vec<&str> = sketch.iter().map(String::as_str).collect();
    sketch.push("intents: [sketch a module design before code, name the pattern of a design]");
    let description = "Design sketch before code, with boxes, arrows and one named pattern.";
    write_skill(&library, "fx-design-sketch", description, &sketch);
    write_claiming(
        &library,
        "fx-diagrams",
        "tool",
        "{ artefact: [diagram] }",
        "",
    );
    write_claiming(
        &library,
        "fx-e-diagrams",
        "tool",
        "{ artefact: [diagram] }",
        "fx-r",
    );
    write_claiming(&library, "fx-a-plan", "rule", "{ artefact: [plan] }", "");
    write_claiming(&library, "fx-b-plan", "rule", "{ artefact: [plan] }", "");
    write_claiming(&library, "fx-r", "rule", "{}", "");
    let unrelated = temp.path().join("zzlib");
    for name in [
        "sketch-module-design",
        "module-design-code",
        "design-sketch-before",
    ] {
        write_skill(&unrelated, name, "Sketch a module design before code.", &[]);
    }
    let form = json!({ "id": "P", "artefacts": ["diagram", "plan"],
                       "needs": ["sketch a module design before code"] });
    let arguments = json!({ "requests": [form], "limit": 20 });

    let alone = Index::from_folders(std::slice::from_ref(&library)).expect("load lib5");
    let both = Index::from_folders(&[library, unrelated]).expect("load lib5 and zzlib");
    let alone = answer_of(&alone, arguments.clone())["results"][0].clone();
    let with_unrelated = answer_of(&both, arguments)["results"][0].clone();

    let first_keyword_item = |result: &Value| {
        let items = result["items"].as_array().expect("items").iter();
        let mut keyword_items = items.filter(|item| item["tier"] == "C");
        keyword_items.next().map(|item| item["skill"].clone())
    };
    assert_eq!(first_keyword_item(&alone), Some(json!("fx-design-sketch")));
    assert_eq!(
        first_keyword_item(&with_unrelated),
        Some(json!("design-sketch-before"))
    );
    let declared = |result: &Value| -> Vec<Value> {
        let wrapped = json!({ "results": [result] });
        let items = declared_items(&wrapped);
        items.iter().map(|item| item[0].clone()).collect()
    };
    let expected = [
        "fx-diagrams",
        "fx-a-plan",
        "fx-e-diagrams",
        "fx-r",
        "fx-b-plan",
    ];
    assert_eq!(declared(&alone), expected);
    assert_eq!(declared(&with_unrelated), expected);
}
