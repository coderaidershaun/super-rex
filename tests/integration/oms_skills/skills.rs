//! Integration tests of `rex skills list` and `rex skills check`. Every run
//! sets `HOME` to a temporary folder, so the user's own skills are never read.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use assert_cmd::Command;

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().expect("file has a folder")).expect("make folder");
    fs::write(path, text).expect("write file");
}

fn rex_skills(cwd: &Path, home: &Path, arguments: &[&str]) -> Output {
    Command::cargo_bin("rex")
        .expect("find rex binary")
        .current_dir(cwd)
        .env("HOME", home)
        .arg("skills")
        .args(arguments)
        .output()
        .expect("run rex skills")
}

fn text_of(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("utf8 output")
}

fn stdout_of_success(cwd: &Path, home: &Path, arguments: &[&str]) -> String {
    let output = rex_skills(cwd, home, arguments);
    assert_eq!(
        output.status.code(),
        Some(0),
        "rex skills {arguments:?}: {}",
        text_of(&output.stderr)
    );
    text_of(&output.stdout)
}

/// A temporary folder with a repo (`repo/`, run from `repo/sub/`) and a home
/// folder (`home/`). The root is canonical, so it matches the paths `rex`
/// prints.
struct ListFixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
}

impl ListFixture {
    fn new() -> ListFixture {
        let temp = tempfile::tempdir().expect("temp dir");
        let root = temp.path().canonicalize().expect("canonical temp dir");
        let repo_skills = root.join("repo/.claude/skills");
        let library = root.join("repo/.rex/skills");
        let user_skills = root.join("home/.claude/skills");
        fs::create_dir_all(root.join("repo/.git")).expect("make .git");
        fs::create_dir_all(root.join("repo/sub")).expect("make sub-folder");

        write(
            &repo_skills.join("alpha/SKILL.md"),
            "---\nname: alpha\ndescription: >\n  First   skill,\n  on two lines.\n\
             when_to_use: >\n  Use when   the test\n  needs alpha.\n---\nBody.\n",
        );
        write(
            &repo_skills.join("beta/SKILL.md"),
            "---\nname: beta\ndescription: Repo beta.\nwhen_to_use: Use when the repo needs beta.\n---\nBody.\n",
        );
        write(
            &repo_skills.join("broken/SKILL.md"),
            "---\nname: [broken\n---\nBody.\n",
        );
        write(&repo_skills.join("notes.txt"), "Not a skill.\n");
        write(
            &repo_skills.join("empty/reference.md"),
            "No SKILL.md here.\n",
        );
        write(
            &library.join("delta/SKILL.md"),
            "---\nname: delta\ndescription: Library delta.\nwhen_to_use: Use when an agent reads delta.\n---\nBody.\n",
        );
        write(
            &library.join("gamma/SKILL.md"),
            "---\nname: gamma\ndescription: Library gamma.\n---\nBody.\n",
        );
        write(
            &user_skills.join("beta/SKILL.md"),
            "---\nname: beta\ndescription: User beta.\nwhen_to_use: Use when the user needs beta.\n---\nBody.\n",
        );
        write(
            &user_skills.join("gamma/SKILL.md"),
            "---\nname: gamma\ndescription: User gamma.\n---\nBody.\n",
        );

        ListFixture { _temp: temp, root }
    }

    fn cwd(&self) -> PathBuf {
        self.root.join("repo/sub")
    }

    fn home(&self) -> PathBuf {
        self.root.join("home")
    }

    fn repo_skill(&self, name: &str) -> String {
        format!(
            "{}/repo/.claude/skills/{name}/SKILL.md",
            self.root.display()
        )
    }

    fn library_skill(&self, name: &str) -> String {
        format!("{}/repo/.rex/skills/{name}/SKILL.md", self.root.display())
    }

    fn user_skill(&self, name: &str) -> String {
        format!(
            "{}/home/.claude/skills/{name}/SKILL.md",
            self.root.display()
        )
    }
}

// A library skill that has the name of a user skill is not shadowed, because
// Claude Code does not load the library; an agent reads it by its path.
#[test]
fn skills_list_prints_repo_then_library_then_user_skills() {
    let fixture = ListFixture::new();

    let stdout = stdout_of_success(&fixture.cwd(), &fixture.home(), &["list"]);

    let expected = format!(
        "name=alpha\n\
         description=First skill, on two lines.\n\
         when_to_use=Use when the test needs alpha.\n\
         source=repo\n\
         shadowed=no\n\
         path={}\n\
         \n\
         name=beta\n\
         description=Repo beta.\n\
         when_to_use=Use when the repo needs beta.\n\
         source=repo\n\
         shadowed=yes\n\
         path={}\n\
         \n\
         name=broken\n\
         description=\n\
         when_to_use=\n\
         source=repo\n\
         shadowed=no\n\
         path={}\n\
         \n\
         name=delta\n\
         description=Library delta.\n\
         when_to_use=Use when an agent reads delta.\n\
         source=library\n\
         shadowed=no\n\
         path={}\n\
         \n\
         name=gamma\n\
         description=Library gamma.\n\
         when_to_use=\n\
         source=library\n\
         shadowed=no\n\
         path={}\n\
         \n\
         name=beta\n\
         description=User beta.\n\
         when_to_use=Use when the user needs beta.\n\
         source=user\n\
         shadowed=no\n\
         path={}\n\
         \n\
         name=gamma\n\
         description=User gamma.\n\
         when_to_use=\n\
         source=user\n\
         shadowed=no\n\
         path={}\n",
        fixture.repo_skill("alpha"),
        fixture.repo_skill("beta"),
        fixture.repo_skill("broken"),
        fixture.library_skill("delta"),
        fixture.library_skill("gamma"),
        fixture.user_skill("beta"),
        fixture.user_skill("gamma"),
    );
    assert_eq!(stdout, expected);
}

#[test]
fn skills_list_json_has_the_same_fields() {
    let fixture = ListFixture::new();

    let stdout = stdout_of_success(&fixture.cwd(), &fixture.home(), &["list", "--json"]);

    let entry = |name: &str, description: &str, when: &str, source: &str, shadowed: bool| {
        let path = match source {
            "repo" => fixture.repo_skill(name),
            "library" => fixture.library_skill(name),
            _ => fixture.user_skill(name),
        };
        format!(
            r#"{{"name":"{name}","description":"{description}","when_to_use":"{when}","source":"{source}","shadowed":{shadowed},"path":"{path}"}}"#
        )
    };
    let entries = [
        entry(
            "alpha",
            "First skill, on two lines.",
            "Use when the test needs alpha.",
            "repo",
            false,
        ),
        entry(
            "beta",
            "Repo beta.",
            "Use when the repo needs beta.",
            "repo",
            true,
        ),
        entry("broken", "", "", "repo", false),
        entry(
            "delta",
            "Library delta.",
            "Use when an agent reads delta.",
            "library",
            false,
        ),
        entry("gamma", "Library gamma.", "", "library", false),
        entry(
            "beta",
            "User beta.",
            "Use when the user needs beta.",
            "user",
            false,
        ),
        entry("gamma", "User gamma.", "", "user", false),
    ];
    assert_eq!(stdout, format!("[{}]\n", entries.join(",")));
}

#[test]
fn skills_list_in_a_worktree_reads_the_main_checkout() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = temp.path().canonicalize().expect("canonical temp dir");
    let main = root.join("main");
    let worktree = root.join("wt");
    write(&main.join(".git/worktrees/wt/commondir"), "../..\n");
    write(
        &worktree.join(".git"),
        &format!("gitdir: {}\n", main.join(".git/worktrees/wt").display()),
    );
    write(
        &main.join(".claude/skills/alpha/SKILL.md"),
        "---\nname: alpha\ndescription: Main alpha.\nwhen_to_use: Use when main.\n---\nBody.\n",
    );
    fs::create_dir_all(root.join("home")).expect("make home");

    let stdout = stdout_of_success(&worktree, &root.join("home"), &["list"]);

    let expected = format!(
        "name=alpha\ndescription=Main alpha.\nwhen_to_use=Use when main.\nsource=repo\n\
         shadowed=no\npath={}/.claude/skills/alpha/SKILL.md\n",
        main.display()
    );
    assert_eq!(stdout, expected);
}

#[cfg(unix)]
#[test]
fn skills_list_fails_when_a_place_cannot_be_read() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = ListFixture::new();
    let place = fixture.root.join("repo/.claude/skills");
    fs::set_permissions(&place, fs::Permissions::from_mode(0o000)).expect("lock place");

    let output = rex_skills(&fixture.cwd(), &fixture.home(), &["list"]);

    fs::set_permissions(&place, fs::Permissions::from_mode(0o755)).expect("unlock place");
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty(), "stdout stays empty");
    let stderr = text_of(&output.stderr);
    assert_eq!(stderr.lines().count(), 1, "one line: {stderr}");
    assert!(
        stderr.starts_with("error: ") && stderr.contains(&place.display().to_string()),
        "the error names the place: {stderr}"
    );
}

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

/// Paths to files of skills that are right: one folder down in this skill, in
/// another skill of the repo, in the repo's library, in a bundled skill and a
/// bundled library skill that the repo does not hold, and a template. The one
/// wrong path, at the end of a sentence, names a file that `clean` does not
/// have.
fn paths_to_skill_files() -> String {
    let bundled = |root: &str| {
        super_rex::bundle::walk()
            .into_iter()
            .find_map(|(path, _)| {
                let file = path.strip_prefix(root).ok()?;
                let is_sibling = file.extension()? == "md" && file.file_name()? != "SKILL.md";
                is_sibling.then(|| path.display().to_string())
            })
            .unwrap_or_else(|| panic!("a skill in {root} has a Markdown file besides SKILL.md"))
    };
    format!(
        "Read `.claude/skills/bad-b4-file/os/notes.md`, `.claude/skills/clean/reference.md`, \
         `.rex/skills/lib/reference.md`, `{}`, `{}` and `.claude/skills/<name>/<file>.md`.\n\
         Then read .claude/skills/clean/gone.md.\n",
        bundled(".claude/skills"),
        bundled(".rex/skills"),
    )
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

    assert_eq!(text_of(&output.stdout).lines().count(), 17);
    assert_eq!(
        text_of(&output.stderr),
        "error: 16 findings in 16 of 17 skills\n"
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
