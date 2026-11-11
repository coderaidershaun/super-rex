//! `rex skills list`: the three places it reads, its JSON form, and a
//! linked worktree.

use std::fs;
use std::path::PathBuf;

use super::{rex_skills, stdout_of_success, text_of, write};

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
fn skills_list_in_a_worktree_reads_the_main_checkout_too() {
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
    // The worktree tracks one library skill of its own, so its library
    // folder holds that one only. The main checkout's library fills in the
    // rest, and the worktree's copy wins.
    for (checkout, name, description) in [
        (&main, "delta", "Main delta."),
        (&main, "notes", "Main notes."),
        (&worktree, "notes", "Worktree notes."),
    ] {
        write(
            &checkout.join(format!(".rex/skills/{name}/SKILL.md")),
            &format!("---\nname: {name}\ndescription: {description}\n---\nBody.\n"),
        );
    }
    fs::create_dir_all(root.join("home")).expect("make home");

    let stdout = stdout_of_success(&worktree, &root.join("home"), &["list"]);

    let expected = format!(
        "name=alpha\ndescription=Main alpha.\nwhen_to_use=Use when main.\nsource=repo\n\
         shadowed=no\npath={main}/.claude/skills/alpha/SKILL.md\n\n\
         name=delta\ndescription=Main delta.\nwhen_to_use=\nsource=library\n\
         shadowed=no\npath={main}/.rex/skills/delta/SKILL.md\n\n\
         name=notes\ndescription=Worktree notes.\nwhen_to_use=\nsource=library\n\
         shadowed=no\npath={worktree}/.rex/skills/notes/SKILL.md\n",
        main = main.display(),
        worktree = worktree.display(),
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
