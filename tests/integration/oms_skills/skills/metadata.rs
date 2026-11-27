//! `rex skills check` on the `metadata` map of a skill header: a correct map
//! passes, and each broken field gives the finding of its own rule.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use super::{rex_skills, text_of, write};

/// The skill whose header carries every field of the map, with the files
/// that its `files` list names.
const FULL_SKILL: &str = "tests/fixtures/skills-metadata/full";

const SMALL_HEADER: &str = "---\nname: small\ndescription: A skill with a short metadata map.\n\
    when_to_use: Use when a test needs a short metadata map.\n";

/// A repo with one draft skill in `drafts/<name>`, and an empty home folder.
struct Draft {
    _temp: tempfile::TempDir,
    root: PathBuf,
    name: &'static str,
}

impl Draft {
    fn new(name: &'static str) -> Draft {
        let temp = tempfile::tempdir().expect("temp dir");
        let root = temp.path().to_path_buf();
        fs::create_dir_all(root.join("repo/.git")).expect("make .git");
        fs::create_dir_all(root.join("home")).expect("make home");
        Draft {
            _temp: temp,
            root,
            name,
        }
    }

    /// A draft that is a copy of the fixture `full`.
    fn full() -> Draft {
        let draft = Draft::new("full");
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join(FULL_SKILL);
        for entry in fs::read_dir(&fixture).expect("read the fixture") {
            let path = entry.expect("read a fixture entry").path();
            let text = fs::read_to_string(&path).expect("read a fixture file");
            let file = path.file_name().expect("a file name").to_string_lossy();
            draft.add(&file, &text);
        }
        draft
    }

    fn folder(&self) -> PathBuf {
        self.root.join("repo/drafts").join(self.name)
    }

    fn add(&self, file: &str, text: &str) {
        write(&self.folder().join(file), text);
    }

    fn remove(&self, file: &str) {
        fs::remove_file(self.folder().join(file)).expect("remove a draft file");
    }

    /// Replaces `old` in `SKILL.md`, which must hold it once.
    fn edit(&self, old: &str, new: &str) {
        let path = self.folder().join("SKILL.md");
        let text = fs::read_to_string(&path).expect("read SKILL.md");
        assert_eq!(
            text.matches(old).count(),
            1,
            "SKILL.md must hold `{old}` once"
        );
        fs::write(&path, text.replacen(old, new, 1)).expect("write SKILL.md");
    }

    /// What `rex skills check` prints on stderr before its `error:` line: the
    /// skip line of the intent check when the map holds `intents`.
    fn intent_skip_line(&self) -> &'static str {
        let text = fs::read_to_string(self.folder().join("SKILL.md")).expect("read SKILL.md");
        if text.contains("\n  intents:") {
            "warning: intent check skipped; give --pool <folder> of other skills to check that \
             each intent finds its skill\n"
        } else {
            ""
        }
    }

    fn check(&self) -> Output {
        let path = format!("drafts/{}", self.name);
        rex_skills(
            &self.root.join("repo"),
            &self.root.join("home"),
            &["check", &path],
        )
    }
}

fn assert_passes(draft: &Draft) {
    let output = draft.check();
    assert_eq!(
        text_of(&output.stdout),
        format!("ok drafts/{}\n", draft.name)
    );
    assert_eq!(text_of(&output.stderr), draft.intent_skip_line());
    assert_eq!(output.status.code(), Some(0));
}

/// Asserts that the draft gives exactly one finding, of `rule`, in
/// `SKILL.md`, and that its message holds `words`.
fn assert_one_finding(draft: &Draft, case: &str, rule: &str, words: &str) {
    let output = draft.check();
    let stdout = text_of(&output.stdout);
    let prefix = format!("drafts/{}/SKILL.md: {rule}: ", draft.name);
    assert!(
        stdout.lines().count() == 1 && stdout.starts_with(&prefix) && stdout.contains(words),
        "{case}: must give one {rule} finding that holds `{words}`, got:\n{stdout}"
    );
    assert_eq!(
        text_of(&output.stderr),
        format!(
            "{}error: 1 findings in 1 of 1 skills\n",
            draft.intent_skip_line()
        ),
        "{case}"
    );
    assert_eq!(output.status.code(), Some(1), "{case}");
}

#[test]
fn skills_check_passes_a_skill_with_the_full_metadata_map() {
    assert_passes(&Draft::full());
}

// Claude Code treats `metadata` as free-form, so a map of other keys, or no
// map at all, is a skill that is not in the search and is still valid.
#[test]
fn skills_check_passes_a_skill_whose_metadata_holds_no_key_of_the_map() {
    for metadata in ["", "metadata:\n  author: a test\n", "metadata: a note\n"] {
        let draft = Draft::new("small");
        draft.add("SKILL.md", &format!("{SMALL_HEADER}{metadata}---\nBody.\n"));
        draft.add("notes.md", "Notes that no files list names.\n");
        assert_passes(&draft);
    }
}

/// A skill whose map holds `kind`, `category` and a `not_when` of
/// `not_when_words` words, so the map holds two more words than that.
fn small_skill(not_when_words: usize) -> Draft {
    let not_when = vec!["word"; not_when_words].join(" ");
    let metadata = format!(
        "metadata:\n  author: a test with words that do not count\n  kind: rule\n  \
         category: testing\n  not_when: \"{not_when}\"\n"
    );
    let draft = Draft::new("small");
    draft.add("SKILL.md", &format!("{SMALL_HEADER}{metadata}---\nBody.\n"));
    draft
}

#[test]
fn skills_check_passes_a_map_of_120_words_and_fails_one_of_121() {
    assert_passes(&small_skill(118));

    assert_one_finding(
        &small_skill(119),
        "121 words",
        "M13",
        "121 words, more than 120",
    );
}

// File paths do not count, so a skill with many files is not pushed over the
// limit by their names.
#[test]
fn skills_check_does_not_count_file_paths_in_the_size_of_the_map() {
    let draft = small_skill(117);
    let long_path = format!("{}.md", vec!["part"; 30].join("-"));
    draft.edit(
        "  kind: rule\n",
        &format!("  kind: rule\n  files:\n    - {{ path: {long_path}, role: topic }}\n"),
    );
    draft.add(&long_path, "A topic.\n");
    assert_passes(&draft);
}

/// One way to break the map of the fixture `full`: replace `old` in its
/// `SKILL.md` with `new`. The draft must then give one finding of `rule`
/// whose message holds `words`.
struct BadEdit {
    case: &'static str,
    rule: &'static str,
    old: &'static str,
    new: &'static str,
    words: &'static str,
}

const FILES_LIST: &str = "  files:\n";
const LAST_INTENT: &str = "    - write a doc comment\n";
const NOT_WHEN: &str = "  not_when: \"Generated code that nobody reads.\"\n";

#[rustfmt::skip]
const BAD_EDITS: [BadEdit; 39] = [
    BadEdit { case: "missing kind", rule: "M1", old: "  kind: rule\n", new: "", words: "kind is missing" },
    BadEdit { case: "kind of no term", rule: "M1", old: "kind: rule", new: "kind: guide", words: "`guide`" },
    BadEdit { case: "two kinds", rule: "M1", old: "kind: rule", new: "kind: [rule, method]", words: "kind is not one text value" },
    BadEdit { case: "missing category", rule: "M2", old: "  category: coding-standards\n", new: "", words: "category is missing" },
    BadEdit { case: "category of no term", rule: "M2", old: "category: coding-standards", new: "category: style", words: "`style`" },
    BadEdit { case: "term of no vocabulary", rule: "M3", old: "activity: [build, fix]", new: "activity: [biuld, fix]", words: "`biuld`" },
    BadEdit { case: "claims key of no field", rule: "M3", old: "    artefact: [unit-test]\n", new: "    artefacts: [unit-test]\n", words: "`artefacts`" },
    BadEdit { case: "claims list as text", rule: "M3", old: "touches: [file-layout]", new: "touches: file-layout", words: "claims.touches is not a list" },
    BadEdit { case: "fact of no vocabulary", rule: "M3", old: "role: plan-reviewer }", new: "role: plan-reviewer, os: linux }", words: "`os`" },
    BadEdit { case: "fact a skill may not claim", rule: "M3", old: "role: plan-reviewer }", new: "role: plan-reviewer, latency_scale: ns_us }", words: "`latency_scale`" },
    BadEdit { case: "fact of the wrong type", rule: "M3", old: "writes_code: true,", new: "writes_code: yes,", words: "writes_code" },
    BadEdit { case: "fact value of no closed list", rule: "M3", old: "languages: [rust], role", new: "languages: [klingon], role", words: "`klingon`" },
    BadEdit { case: "fact value against its pattern", rule: "M3", old: "role: plan-reviewer", new: "role: Plan_Reviewer", words: "`Plan_Reviewer`" },
    BadEdit { case: "placement of no form", rule: "M4", old: "placement: full", new: "placement: read-first", words: "`read-first`" },
    BadEdit { case: "scope key of no vocabulary", rule: "M5", old: "latency_scale: [ns_us] }", new: "latency_scale: [ns_us], os: [linux] }", words: "`os`" },
    BadEdit { case: "scope value of no closed list", rule: "M5", old: "languages: [rust], latency", new: "languages: [klingon], latency", words: "`klingon`" },
    BadEdit { case: "scope value as text", rule: "M5", old: "latency_scale: [ns_us] }", new: "latency_scale: ns_us }", words: "scope.latency_scale is not a list" },
    BadEdit { case: "scope with no not_when", rule: "M6", old: NOT_WHEN, new: "", words: "not_when is missing" },
    BadEdit { case: "not_when of two lines", rule: "M6", old: NOT_WHEN, new: "  not_when: |\n    Generated code\n    that nobody reads.\n", words: "one line" },
    BadEdit { case: "two intents", rule: "M7", old: LAST_INTENT, new: "", words: "2 intents" },
    BadEdit { case: "six intents", rule: "M7", old: LAST_INTENT, new: "    - a\n    - b\n    - c\n    - d\n", words: "6 intents" },
    BadEdit { case: "intent that is not text", rule: "M7", old: LAST_INTENT, new: "    - [write, a doc comment]\n", words: "not a list of text" },
    BadEdit { case: "files item with no path", rule: "M8", old: FILES_LIST, new: "  files:\n    - { role: topic }\n", words: "no `path`" },
    BadEdit { case: "files item that is not a map", rule: "M8", old: FILES_LIST, new: "  files:\n    - extra.md\n", words: "not a map" },
    BadEdit { case: "files role of no form", rule: "M8", old: "measure.md, role: topic", new: "measure.md, role: reference", words: "`reference`" },
    BadEdit { case: "language file with no language", rule: "M8", old: "role: language, language: rust", new: "role: language", words: "no `language`" },
    BadEdit { case: "code file of no language", rule: "M8", old: "role: code, language: rust", new: "role: code, language: klingon", words: "`klingon`" },
    BadEdit { case: "SKILL.md in files", rule: "M8", old: FILES_LIST, new: "  files:\n    - { path: SKILL.md, role: topic }\n", words: "SKILL.md" },
    BadEdit { case: "file intents of six", rule: "M8", old: "intents: [measure a hot path]", new: "intents: [a, b, c, d, e, f]", words: "6 intents; give 1 to 5" },
    BadEdit { case: "empty file intent", rule: "M8", old: "intents: [measure a hot path]", new: "intents: [\"\"]", words: "empty" },
    BadEdit { case: "open_when with no when", rule: "M10", old: ", when: \"you measure the hot path\"", new: "", words: "no `when`" },
    BadEdit { case: "open_when skill that does not exist", rule: "M10", old: "skill: rex-code-error-writing", new: "skill: rex-no-such-skill", words: "`rex-no-such-skill`" },
    BadEdit { case: "open_when file of another skill that does not exist", rule: "M10", old: "file: mocking.md", new: "file: no-such-file.md", words: "`rex-code-tdd/no-such-file.md`" },
    BadEdit { case: "open_when file of this skill that does not exist", rule: "M10", old: "file: measure.md", new: "file: gone.md", words: "`gone.md`" },
    BadEdit { case: "requires a skill that does not exist", rule: "M11", old: "requires: [rex-code-philosophy]", new: "requires: [rex-no-such-skill]", words: "`rex-no-such-skill`" },
    BadEdit { case: "replaces a skill that does not exist", rule: "M11", old: "replaces: [rex-code-ergonomics]", new: "replaces: [rex-old]", words: "`rex-old`" },
    BadEdit { case: "max_topics of zero", rule: "M12", old: "max_topics: 2", new: "max_topics: 0", words: "positive whole number" },
    BadEdit { case: "max_topics as text", rule: "M12", old: "max_topics: 2", new: "max_topics: two", words: "positive whole number" },
    BadEdit { case: "max_topics as a fraction", rule: "M12", old: "max_topics: 2", new: "max_topics: 1.5", words: "positive whole number" },
];

#[test]
fn skills_check_names_the_metadata_rule_each_bad_field_breaks() {
    for BadEdit {
        case,
        rule,
        old,
        new,
        words,
    } in BAD_EDITS
    {
        let draft = Draft::full();
        draft.edit(old, new);
        assert_one_finding(&draft, case, rule, words);
    }
}

#[test]
fn skills_check_fails_a_file_of_the_folder_that_files_does_not_list() {
    let draft = Draft::full();
    draft.add("notes/extra.txt", "A file that no files item names.\n");

    assert_one_finding(&draft, "unlisted file", "M9", "`notes/extra.txt`");
}

#[test]
fn skills_check_fails_a_listed_file_that_is_not_in_the_folder() {
    let draft = Draft::full();
    draft.remove("measure-code.md");

    assert_one_finding(&draft, "listed file missing", "M9", "`measure-code.md`");
}

// A skill may send an agent to any file of another skill, not only to its
// Markdown files.
#[test]
fn skills_check_finds_an_open_when_file_of_another_skill_that_is_not_markdown() {
    let draft = Draft::full();
    draft.edit(
        "  open_when:\n",
        "  open_when:\n    - { skill: other, file: data.json, when: \"you need the data\" }\n",
    );
    draft.add(
        "../other/SKILL.md",
        "---\nname: other\n---\nAnother skill.\n",
    );
    draft.add("../other/data.json", "{}\n");
    assert_passes(&draft);

    draft.remove("../other/data.json");
    assert_one_finding(
        &draft,
        "open_when file of another skill that is gone",
        "M10",
        "`other/data.json`",
    );
}
