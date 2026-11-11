//! `rex skills check`: a skill that passes, the rule each bad skill breaks,
//! and a bad path.

use std::fs;
use std::path::PathBuf;
use std::process::Output;

use super::{rex_skills, text_of, write};

const CLEAN_SKILL: &str = "---\nname: clean\ndescription: A skill that breaks no rule.\n\
    when_to_use: >\n  Use when a test needs a clean skill.\nuser-invocable: true\n---\n\
    Read [the reference](reference.md#top) and `.claude/skills/clean/SKILL.md`.\n";

const CLEAN_REFERENCE: &str =
    "Back to [the skill](SKILL.md). See [the web](https://example.com/page.md).\n";

const LIBRARY_SKILL: &str = "---\nname: lib\ndescription: A library skill that breaks no rule.\n\
    when_to_use: >\n  Use when a test needs a library skill.\n---\n\
    Read [the reference](reference.md), `.rex/skills/lib/reference.md` and \
    `.claude/skills/clean/SKILL.md`.\n";

/// A repo whose skills folder holds the skill `clean` and whose library holds
/// the skill `lib`, and an empty home folder. Commands run from the repo root.
struct CheckFixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
}

impl CheckFixture {
    fn new() -> CheckFixture {
        let temp = tempfile::tempdir().expect("temp dir");
        let root = temp.path().to_path_buf();
        fs::create_dir_all(root.join("repo/.git")).expect("make .git");
        fs::create_dir_all(root.join("home")).expect("make home");
        let fixture = CheckFixture { _temp: temp, root };
        fixture.add("clean/SKILL.md", CLEAN_SKILL);
        fixture.add("clean/reference.md", CLEAN_REFERENCE);
        let library = fixture.repo().join(".rex/skills/lib");
        write(&library.join("SKILL.md"), LIBRARY_SKILL);
        write(&library.join("reference.md"), CLEAN_REFERENCE);
        fixture
    }

    fn add(&self, file_in_skills_folder: &str, text: &str) {
        write(
            &self
                .repo()
                .join(".claude/skills")
                .join(file_in_skills_folder),
            text,
        );
    }

    fn repo(&self) -> PathBuf {
        self.root.join("repo")
    }

    fn check(&self, paths: &[&str]) -> Output {
        let mut arguments = vec!["check"];
        arguments.extend(paths);
        rex_skills(&self.repo(), &self.root.join("home"), &arguments)
    }
}

#[test]
fn skills_check_prints_ok_for_a_skill_that_passes() {
    let fixture = CheckFixture::new();

    let output = fixture.check(&[
        ".claude/skills/clean",
        ".claude/skills/clean/SKILL.md",
        ".rex/skills/lib",
    ]);

    assert_eq!(text_of(&output.stderr), "");
    assert_eq!(
        text_of(&output.stdout),
        "ok .claude/skills/clean\nok .claude/skills/clean/SKILL.md\nok .rex/skills/lib\n"
    );
    assert_eq!(output.status.code(), Some(0));

    // A repo with no skills folder and no library of its own: the bundle
    // stands in for both, as `rex init` would install it.
    let bare = fixture.root.join("bare");
    fs::create_dir_all(bare.join(".git")).expect("make .git");
    write(
        &bare.join("drafts/draft/SKILL.md"),
        &(header("draft", DESCRIPTION, WHEN_TO_USE)
            + &format!(
                "Read `{}` and `{}`.\n",
                bundled_sibling(".claude/skills"),
                bundled_sibling(".rex/skills")
            )),
    );
    let output = rex_skills(
        &bare,
        &fixture.root.join("home"),
        &["check", "drafts/draft"],
    );
    assert_eq!(text_of(&output.stdout), "ok drafts/draft\n");
    assert_eq!(output.status.code(), Some(0));
}

const DESCRIPTION: &str = "A skill for a test.";
const WHEN_TO_USE: &str = "Use when a test needs it.";

fn header(name: &str, description: &str, when_to_use: &str) -> String {
    format!("---\nname: {name}\ndescription: {description}\nwhen_to_use: {when_to_use}\n---\n")
}

/// One skill that breaks `rule` and no other. The last file is the one the
/// finding is in.
struct BadSkill {
    folder: &'static str,
    rule: &'static str,
    files: Vec<(&'static str, String)>,
}

fn bad_skills() -> Vec<BadSkill> {
    let one = |folder, rule, text: String| BadSkill {
        folder,
        rule,
        files: vec![("SKILL.md", text + "Body.\n")],
    };
    let good = |name| header(name, DESCRIPTION, WHEN_TO_USE);
    let misspelt_key = format!("---\nwhen-to-use: {WHEN_TO_USE}\n");
    vec![
        one("bad-h1", "H1", "# No header\n\n".to_owned()),
        one("bad-h2", "H2", "---\nname: [bad-h2\n---\n".to_owned()),
        one("bad-h3", "H3", good("other-name")),
        one("bad-h4", "H4", header("bad-h4", WHEN_TO_USE, WHEN_TO_USE)),
        one(
            "bad-h5",
            "H5",
            header("bad-h5", DESCRIPTION, "Load it for a test."),
        ),
        one(
            "bad-h6",
            "H6",
            header("bad-h6", "Checks #[cfg(test)] blocks.", WHEN_TO_USE),
        ),
        one(
            "bad-h7",
            "H7",
            header("bad-h7", &"a".repeat(1600), WHEN_TO_USE),
        ),
        one(
            "bad-h8",
            "H8",
            good("bad-h8").replacen("---\n", &misspelt_key, 1),
        ),
        one(
            "bad-h9",
            "H9",
            header("bad-h9", &[DESCRIPTION; 11].join(" "), WHEN_TO_USE),
        ),
        BadSkill {
            folder: "bad-b1",
            rule: "B1",
            files: vec![("SKILL.md", good("bad-b1"))],
        },
        one("bad-b2", "B2", good("bad-b2") + &"line\n".repeat(500)),
        BadSkill {
            folder: "bad-b3",
            rule: "B3",
            files: vec![
                ("SKILL.md", good("bad-b3") + "Read [it](reference.md).\n"),
                ("reference.md", "See [gone](gone.md).\n".to_owned()),
            ],
        },
        one(
            "bad-b4",
            "B4",
            good("bad-b4") + "Read `.claude/skills/no-such-skill/SKILL.md`.\n",
        ),
        one(
            "bad-b4-folder",
            "B4",
            good("bad-b4-folder") + "Read `.claude/skills/lib/SKILL.md`.\n",
        ),
        // The repo has a library, so a bundled file that it does not hold is
        // not there for an agent to read.
        one(
            "bad-b4-bundled",
            "B4",
            good("bad-b4-bundled") + &format!("Read `{}`.\n", bundled_sibling(".rex/skills")),
        ),
        BadSkill {
            folder: "bad-b4-file",
            rule: "B4",
            files: vec![
                ("os/notes.md", "Notes one folder down.\n".to_owned()),
                ("SKILL.md", good("bad-b4-file") + &paths_to_skill_files()),
            ],
        },
        one("bad-b5", "B5", good("bad-b5") + &"word ".repeat(1500)),
    ]
}

/// The path of a Markdown file besides `SKILL.md` of a bundled skill in
/// `root`, such as `.rex/skills/rex-code-tdd/mocking.md`.
fn bundled_sibling(root: &str) -> String {
    super_rex::bundle::walk()
        .into_iter()
        .find_map(|(path, _)| {
            let file = path.strip_prefix(root).ok()?;
            let is_sibling = file.extension()? == "md" && file.file_name()? != "SKILL.md";
            is_sibling.then(|| path.display().to_string())
        })
        .unwrap_or_else(|| panic!("a skill in {root} has a Markdown file besides SKILL.md"))
}

/// Paths to files of skills that are right: one folder down in this skill, in
/// another skill of the repo, in the repo's library, and a template. The one
/// wrong path, at the end of a sentence, names a file that `clean` does not
/// have.
fn paths_to_skill_files() -> String {
    "Read `.claude/skills/bad-b4-file/os/notes.md`, `.claude/skills/clean/reference.md`, \
     `.rex/skills/lib/reference.md` and `.claude/skills/<name>/<file>.md`.\n\
     Then read .claude/skills/clean/gone.md.\n"
        .to_owned()
}

#[test]
fn skills_check_names_the_rule_each_bad_skill_breaks() {
    let fixture = CheckFixture::new();
    let skills = bad_skills();
    for bad in &skills {
        for (file, text) in &bad.files {
            fixture.add(&format!("{}/{file}", bad.folder), text);
        }
    }

    for BadSkill {
        folder,
        rule,
        files,
    } in &skills
    {
        let finding_file = files.last().expect("a skill has files").0;
        let output = fixture.check(&[&format!(".claude/skills/{folder}")]);

        let stdout = text_of(&output.stdout);
        let prefix = format!(".claude/skills/{folder}/{finding_file}: {rule}: ");
        assert!(
            stdout.lines().count() == 1 && stdout.starts_with(&prefix),
            "{folder} must give exactly one {rule} finding, got:\n{stdout}"
        );
        assert_eq!(
            text_of(&output.stderr),
            "error: 1 findings in 1 of 1 skills\n",
            "{folder}"
        );
        assert_eq!(output.status.code(), Some(1), "{folder}");
    }

    let mut every_path = vec![".claude/skills/clean".to_owned()];
    every_path.extend(
        skills
            .iter()
            .map(|bad| format!(".claude/skills/{}", bad.folder)),
    );
    let every_path: Vec<&str> = every_path.iter().map(String::as_str).collect();
    let output = fixture.check(&every_path);

    assert_eq!(text_of(&output.stdout).lines().count(), 18);
    assert_eq!(
        text_of(&output.stderr),
        "error: 17 findings in 17 of 18 skills\n"
    );
    assert_eq!(output.status.code(), Some(1));

    let output = fixture.check(&[".claude/skills/bad-b4-folder"]);
    assert_eq!(
        text_of(&output.stdout),
        ".claude/skills/bad-b4-folder/SKILL.md: B4: .claude/skills/lib/SKILL.md names `lib`, \
         a skill in .rex/skills/; write .rex/skills/lib/SKILL.md\n",
        "a path to the wrong folder says which folder holds the skill"
    );
}

#[test]
fn skills_check_rejects_a_bad_path_before_checking() {
    let fixture = CheckFixture::new();
    write(&fixture.repo().join("README.md"), "Not a skill.\n");
    fs::create_dir_all(fixture.repo().join("no-skill-file")).expect("make folder");

    for (paths, code) in [
        (vec![".claude/skills/clean", "no-such-skill"], 1),
        (vec![".claude/skills/clean", "no-skill-file"], 1),
        (vec![".claude/skills/clean", "README.md"], 2),
    ] {
        let output = fixture.check(&paths);

        assert_eq!(output.status.code(), Some(code), "{paths:?}");
        assert!(output.stdout.is_empty(), "{paths:?}: nothing is checked");
        let stderr = text_of(&output.stderr);
        let bad_path = paths.last().expect("a bad path");
        assert!(
            stderr.lines().count() == 1
                && stderr.starts_with("error: ")
                && stderr.contains(bad_path),
            "{paths:?}: one error line that names the path, got: {stderr}"
        );
    }
}
