//! A first and a second `rex init`: the bundle files, `AGENTS.md` and
//! `.gitignore` each land once and are never replaced, and the person is
//! told which project files to commit.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::support::{
    COMMIT_ALL, SKILL_MARKER, bundle_file, bundle_file_count, installed, repo, rex_init_ok,
};

/// The block `rex init` adds to `.gitignore`, byte for byte.
const SUPERREX_IGNORE_BLOCK: &[u8] = b"# rex:ignore superrex\n.claude/skills/\n.claude/agents/\n.claude/settings.local.json\n/.rex/\n/.claude/worktrees/\n";

/// The first line of a first run.
const SETTINGS_CREATED: &str = "rex init: .claude/settings.local.json created\n";

#[test]
fn fresh_install_writes_every_bundle_file() {
    let repo = repo();
    let n = bundle_file_count();

    let printed = rex_init_ok(&repo, &[]);

    assert_eq!(
        printed.stdout,
        format!(
            "{SETTINGS_CREATED}rex init: {n} written, 0 skipped\nrex init: AGENTS.md created\nrex init: .gitignore created\nrex init: .rex/ created\nrex init: .pre-commit-config.yaml created\nrex init: pre-commit hooks installed\n{COMMIT_ALL}"
        )
    );

    let marker = fs::read(installed(&repo, SKILL_MARKER)).expect("read marker skill");
    assert_eq!(marker, bundle_file(SKILL_MARKER));
}

// This test must never be deleted. It is the only check that running the
// install command a second time does not rewrite files it already wrote.
#[test]
fn second_run_writes_nothing() {
    let repo = repo();
    let n = bundle_file_count();

    rex_init_ok(&repo, &[]);

    let before = fs::read(installed(&repo, SKILL_MARKER)).expect("read marker skill");

    let printed = rex_init_ok(&repo, &[]);

    assert_eq!(
        printed.stdout,
        format!("rex init: 0 written, {n} skipped\nrex init: pre-commit hooks installed\n")
    );

    let after = fs::read(installed(&repo, SKILL_MARKER)).expect("read marker skill");
    assert_eq!(
        before, after,
        "second run must not rewrite an already-installed file"
    );
}

// This test must never be deleted. It is the only check that a file the
// user has already edited is never overwritten by the install command.
#[test]
fn existing_modified_file_is_never_touched() {
    let repo = repo();
    let n = bundle_file_count();

    let marker = installed(&repo, SKILL_MARKER);
    fs::create_dir_all(marker.parent().expect("marker has a folder"))
        .expect("create marker skill folder");
    fs::write(&marker, b"sentinel content").expect("write sentinel marker skill");

    let printed = rex_init_ok(&repo, &[]);

    assert_eq!(
        printed.stdout,
        format!(
            "{SETTINGS_CREATED}rex init: {} written, 1 skipped (1 differs from this version; use --force to overwrite)\nrex init: AGENTS.md created\nrex init: .gitignore created\nrex init: .rex/ created\nrex init: .pre-commit-config.yaml created\nrex init: pre-commit hooks installed\n{COMMIT_ALL}",
            n - 1
        )
    );

    let contents = fs::read(&marker).expect("read marker skill");
    assert_eq!(contents, b"sentinel content");
}

#[test]
fn missing_file_is_backfilled() {
    let repo = repo();
    let n = bundle_file_count();

    rex_init_ok(&repo, &[]);

    let marker = installed(&repo, SKILL_MARKER);
    fs::remove_file(&marker).expect("remove marker skill");

    let printed = rex_init_ok(&repo, &[]);

    assert_eq!(
        printed.stdout,
        format!(
            "rex init: 1 written, {} skipped\nrex init: pre-commit hooks installed\n",
            n - 1
        )
    );

    let restored = fs::read(&marker).expect("read marker skill");
    assert_eq!(restored, bundle_file(SKILL_MARKER));
}

#[test]
fn the_claude_install_is_byte_for_byte_the_bundle() {
    let repo = repo();
    rex_init_ok(&repo, &[]);

    for (path, contents) in super_rex::bundle::walk() {
        let installed = fs::read(repo.path().join(".claude").join(&path)).expect("read installed");
        assert_eq!(installed, contents, "{} was changed", path.display());
    }
}

#[test]
fn fresh_install_creates_agents_md_with_codebase_section() {
    let repo = repo();

    let printed = rex_init_ok(&repo, &[]);

    assert!(printed.stdout.contains("rex init: AGENTS.md created\n"));

    let agents_md = fs::read_to_string(repo.path().join("AGENTS.md")).expect("read AGENTS.md");
    assert!(agents_md.starts_with("<!-- rex:codebase -->\n## Codebase map\n"));
    assert!(agents_md.contains("`rex codebase --only code --with-context`"));
    assert!(agents_md.contains("`rex codebase --for-human --only code --with-context`"));
    assert!(agents_md.contains("\n## Code smells\n"));
    assert!(agents_md.contains("- `rex smells`: "));
}

#[test]
fn existing_agents_md_gets_section_on_top_once() {
    let repo = repo();
    let old: &[u8] = b"# My project\n\nKeep me exactly as I am.\n";
    fs::write(repo.path().join("AGENTS.md"), old).expect("write AGENTS.md");

    let first = rex_init_ok(&repo, &[]);

    assert!(first.stdout.contains("rex init: AGENTS.md updated\n"));

    let after_first = fs::read(repo.path().join("AGENTS.md")).expect("read AGENTS.md");
    assert!(after_first.starts_with(b"<!-- rex:codebase -->\n## Codebase map\n"));
    assert!(
        after_first.ends_with(&[b"\n\n", old].concat()),
        "old bytes must follow the section after one blank line"
    );

    let second = rex_init_ok(&repo, &[]);

    assert!(!second.stdout.contains("AGENTS.md"));

    let after_second = fs::read(repo.path().join("AGENTS.md")).expect("read AGENTS.md");
    assert_eq!(after_first, after_second);
}

#[test]
fn fresh_install_creates_gitignore_with_the_superrex_block() {
    let repo = repo();

    rex_init_ok(&repo, &[]);

    let gitignore = fs::read(repo.path().join(".gitignore")).expect("read .gitignore");
    assert_eq!(gitignore, SUPERREX_IGNORE_BLOCK);
}

#[test]
fn existing_gitignore_gets_block_appended_once() {
    let repo = repo();
    fs::write(repo.path().join(".gitignore"), b"/target").expect("write .gitignore");

    let first = rex_init_ok(&repo, &[]);

    assert!(first.stdout.contains("rex init: .gitignore updated\n"));

    let after_first = fs::read(repo.path().join(".gitignore")).expect("read .gitignore");
    assert_eq!(
        after_first,
        [b"/target\n\n", SUPERREX_IGNORE_BLOCK].concat()
    );

    let second = rex_init_ok(&repo, &[]);

    assert!(!second.stdout.contains(".gitignore"));

    let after_second = fs::read(repo.path().join(".gitignore")).expect("read .gitignore");
    assert_eq!(after_first, after_second);
}

#[test]
fn the_commit_line_names_only_the_files_this_run_changed() {
    let repo = repo();
    let n = bundle_file_count();
    rex_init_ok(&repo, &[]);

    fs::write(repo.path().join(".gitignore"), b"/target\n").expect("write .gitignore");
    let printed = rex_init_ok(&repo, &[]);

    assert_eq!(
        printed.stdout,
        format!(
            "rex init: 0 written, {n} skipped\nrex init: .gitignore updated\nrex init: pre-commit hooks installed\nrex init: commit .gitignore (git add + git commit); a job works in the main checkout only when it is clean\n"
        )
    );

    fs::remove_file(repo.path().join("AGENTS.md")).expect("remove AGENTS.md");
    fs::remove_file(repo.path().join(".gitignore")).expect("remove .gitignore");
    let printed = rex_init_ok(&repo, &[]);

    assert!(
        printed.stdout.ends_with(
            "rex init: commit AGENTS.md and .gitignore (git add + git commit); a job works in the main checkout only when it is clean\n"
        ),
        "got: {}",
        printed.stdout
    );
}

// An older rex-skills install has files that this version does not ship and
// older texts in the files it does. None of them may change, and the new
// ignore entries and the agents must still arrive.
#[test]
fn an_older_rex_skills_install_keeps_its_files_and_gets_the_new_entries() {
    let repo = repo();
    let n = bundle_file_count();
    let old_ignore_block: &[u8] =
        b"# rex:ignore claude\n.claude/skills/\n.claude/settings.local.json\n";
    let kept: [(&str, &[u8]); 5] = [
        (".claude/skills/INDEX.md", b"an older skill index\n"),
        (".claude/skills/rex-code-lead/SKILL.md", b"an older skill\n"),
        (".claude/skills/rex-skill-language/SKILL.md", b"an older text of this skill\n"),
        (
            "AGENTS.md",
            b"<!-- rex:codebase -->\n## Codebase map\n\n- `rex codebase --rust-only --with-context`: an older section.\n",
        ),
        (
            ".pre-commit-config.yaml",
            b"# rex:pre-commit\n# Installed by `rex skills`.\nrepos: []\n",
        ),
    ];
    for (path, contents) in kept {
        let path = repo.path().join(path);
        fs::create_dir_all(path.parent().expect("file has a folder")).expect("create folder");
        fs::write(&path, contents).expect("write older file");
    }
    fs::write(repo.path().join(".gitignore"), old_ignore_block).expect("write .gitignore");

    let printed = rex_init_ok(&repo, &[]);

    for (path, contents) in kept {
        let after = fs::read(repo.path().join(path)).expect("read older file");
        assert_eq!(after, contents, "{path} must not change");
    }
    let gitignore = fs::read(repo.path().join(".gitignore")).expect("read .gitignore");
    assert_eq!(
        gitignore,
        [old_ignore_block, b"\n", SUPERREX_IGNORE_BLOCK].concat()
    );
    for (path, contents) in super_rex::bundle::walk() {
        if path.starts_with("agents") {
            let installed = fs::read(repo.path().join(".claude").join(&path)).expect("read agent");
            assert_eq!(installed, contents, "{} must be installed", path.display());
        }
    }
    assert!(
        printed.stdout.contains(&format!(
            "rex init: {} written, 1 skipped (1 differs from this version; use --force to overwrite)\n",
            n - 1
        )),
        "got: {}",
        printed.stdout
    );
}

// The promise that `rex init` is safe to run twice: the second run
// leaves every file in the repo as it was and adds none.
#[test]
fn a_second_run_changes_no_file() {
    let repo = repo();
    rex_init_ok(&repo, &[]);
    let before = files_under(repo.path());

    rex_init_ok(&repo, &[]);

    let after = files_under(repo.path());
    assert_eq!(
        before.keys().collect::<Vec<_>>(),
        after.keys().collect::<Vec<_>>(),
        "the second run must add or remove no file"
    );
    for (path, contents) in &before {
        assert!(
            after[path] == *contents,
            "the second run changed {}",
            path.display()
        );
    }
}

/// Every file under `root` except those in its `.git` folder, by path
/// relative to `root`, with its bytes.
fn files_under(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut files = BTreeMap::new();
    let mut folders = vec![root.to_path_buf()];
    while let Some(folder) = folders.pop() {
        for entry in fs::read_dir(&folder).expect("read folder") {
            let path = entry.expect("read folder entry").path();
            if path == root.join(".git") {
                continue;
            }
            if path.is_dir() {
                folders.push(path);
            } else {
                let contents = fs::read(&path).expect("read file");
                let relative = path.strip_prefix(root).expect("path under root");
                files.insert(relative.to_path_buf(), contents);
            }
        }
    }
    files
}
