//! The choice of files inside one found skill: topic files by router case,
//! `max_topics`, and language files and code twins on a language match.

use super_rex::commands::skills::search::{
    FileEntry, FileRole, FormTerms, Kind, OpenWhen, SkillEntry, SkillMetadata, choose_files,
};

fn file(path: &str, role: FileRole, language: Option<&str>, when: Option<&str>) -> FileEntry {
    FileEntry {
        path: path.to_owned(),
        sha256: format!("hash-of-{path}"),
        role: Some(role),
        language: language.map(str::to_owned),
        when: when.map(str::to_owned),
        intents: Vec::new(),
    }
}

/// `fx-latency` of the fixture library, its files in path order.
fn latency(max_topics: Option<usize>) -> SkillEntry {
    let mut entry = SkillEntry::new("fx-latency", "skills-search");
    let mut metadata = SkillMetadata::new(Kind::Reference, "performance");
    metadata.max_topics = max_topics;
    entry.metadata = Some(metadata);
    let mut measure = file(
        "measure.md",
        FileRole::Topic,
        None,
        Some("you have no measurement yet"),
    );
    measure.intents = vec!["measure a hot path first".to_owned()];
    entry.files = vec![
        file(
            "allocation.md",
            FileRole::Topic,
            None,
            Some("allocation shows in the profile"),
        ),
        file("measure-code.md", FileRole::Code, Some("rust"), None),
        measure,
        file(
            "queues.md",
            FileRole::Topic,
            None,
            Some("a queue between threads is slow"),
        ),
        file("rust.md", FileRole::Language, Some("rust"), None),
    ];
    entry
}

fn form(needs: &[&str], languages: &[&str]) -> FormTerms {
    let mut form = FormTerms {
        id: "T1".to_owned(),
        needs: needs.iter().map(|need| (*need).to_owned()).collect(),
        ..FormTerms::default()
    };
    form.facts.writes_code = Some(true);
    form.facts.languages = languages
        .iter()
        .map(|language| (*language).to_owned())
        .collect();
    form
}

fn paths(choice: &super_rex::commands::skills::search::FileChoice) -> Vec<&str> {
    choice.files.iter().map(|file| file.path.as_str()).collect()
}

#[test]
fn topic_files_never_exceed_max_topics() {
    let need = "no measurement yet and a slow queue between threads and allocation in the profile";

    let capped = choose_files(&latency(Some(2)), &form(&[need], &["rust"])).expect("choose files");
    let uncapped = choose_files(&latency(None), &form(&[need], &["rust"])).expect("choose files");

    let topics = |choice: &super_rex::commands::skills::search::FileChoice| {
        choice
            .files
            .iter()
            .filter(|file| file.role == FileRole::Topic)
            .count()
    };
    assert_eq!(paths(&capped), ["allocation.md", "queues.md", "rust.md"]);
    assert_eq!(topics(&capped), 2);
    assert_eq!(topics(&uncapped), 3);
    assert!(capped.open_when.is_empty());
}

#[test]
fn no_symptom_lists_topic_files_as_open_when() {
    let choice = choose_files(&latency(Some(2)), &form(&[], &["rust"])).expect("choose files");

    let open = |path: &str, when: &str| OpenWhen {
        skill: None,
        file: Some(path.to_owned()),
        when: when.to_owned(),
    };
    assert_eq!(paths(&choice), ["rust.md"]);
    assert_eq!(choice.files[0].sha256.as_deref(), Some("hash-of-rust.md"));
    assert_eq!(
        choice.open_when,
        [
            open("allocation.md", "allocation shows in the profile"),
            open("measure.md", "you have no measurement yet"),
            open("measure-code.md", "you have no measurement yet"),
            open("queues.md", "a queue between threads is slow"),
        ]
    );
}

#[test]
fn language_files_and_code_twins_come_on_a_language_match() {
    let mut entry = SkillEntry::new("parsers", "main");
    entry.metadata = Some(SkillMetadata::new(Kind::Reference, "performance"));
    entry.files = vec![
        file("go.md", FileRole::Language, Some("go"), None),
        file("rust.md", FileRole::Language, Some("rust"), None),
        file("t-code.md", FileRole::Code, Some("rust"), None),
        file("t.md", FileRole::Topic, None, Some("the parser is slow")),
    ];
    let need = "make a slow parser fast";

    let go = choose_files(&entry, &form(&[need], &["go"])).expect("choose files for go");
    let rust = choose_files(&entry, &form(&[need], &["rust"])).expect("choose files for rust");

    assert_eq!(paths(&go), ["t.md", "go.md"]);
    assert_eq!(paths(&rust), ["t.md", "t-code.md", "rust.md"]);
    assert!(go.open_when.is_empty() && rust.open_when.is_empty());
}
