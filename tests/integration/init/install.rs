//! A first and a second `rex init`: the bundle files, `AGENTS.md` and
//! `.gitignore` each land once and are never replaced, and the person is
//! told which project files to commit.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::support::{
    AGENT_MARKER, COMMIT_ALL, SKILL_MARKER, bundle_file, bundle_file_count, installed, repo,
    rex_init_ok,
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
    assert_eq!(
        printed.stderr, "",
        "no warning when every tool is installed"
    );

    let mut roots = BTreeSet::new();
    for (path, contents) in super_rex::bundle::walk() {
        let installed = fs::read(repo.path().join(&path)).expect("read installed");
        assert_eq!(installed, contents, "{} was changed", path.display());
        roots.extend(super_rex::bundle::root_of(&path));
    }
    assert_eq!(
        roots,
        BTreeSet::from([".claude/agents", ".claude/skills", ".rex/skills"]),
        "the skills, the agents and the library are each installed"
    );

    let agents_md = fs::read_to_string(repo.path().join("AGENTS.md")).expect("read AGENTS.md");
    assert!(agents_md.starts_with("<!-- rex:codebase -->\n## Codebase map\n"));
    assert!(agents_md.contains("`rex codebase --only code --with-context`"));
    assert!(agents_md.contains("`rex codebase --for-human --only code --with-context`"));
    assert!(agents_md.contains("\n## Code smells\n"));
    assert!(agents_md.contains("- `rex smells`: "));

    let gitignore = fs::read(repo.path().join(".gitignore")).expect("read .gitignore");
    assert_eq!(gitignore, SUPERREX_IGNORE_BLOCK);

    let store = repo.path().join(".rex");
    assert!(store.join("orchestrators").is_dir());
    assert!(store.join("rex.db").is_file());
}

// This test must never be deleted. It is the only check that a file the
// user has already edited is never overwritten by the install command, and
// that a deleted file is put back.
#[test]
fn existing_modified_file_is_never_touched() {
    let repo = repo();
    let n = bundle_file_count();

    let marker = installed(&repo, SKILL_MARKER);
    let older_files: [(&str, &[u8]); 2] = [
        (".claude/skills/INDEX.md", b"an older skill index\n"),
        (".claude/skills/rex-code-lead/SKILL.md", b"an older skill\n"),
    ];
    fs::create_dir_all(marker.parent().expect("marker has a folder"))
        .expect("create marker skill folder");
    fs::write(&marker, b"sentinel content").expect("write sentinel marker skill");
    for (path, contents) in older_files {
        let path = repo.path().join(path);
        fs::create_dir_all(path.parent().expect("file has a folder")).expect("create folder");
        fs::write(&path, contents).expect("write older file");
    }

    let printed = rex_init_ok(&repo, &[]);

    assert_eq!(
        printed.stdout,
        format!(
            "{SETTINGS_CREATED}rex init: {} written, 1 skipped (1 differs from this version; use --force to overwrite)\nrex init: AGENTS.md created\nrex init: .gitignore created\nrex init: .rex/ created\nrex init: .pre-commit-config.yaml created\nrex init: pre-commit hooks installed\n{COMMIT_ALL}",
            n - 1
        )
    );
    assert_eq!(
        fs::read(&marker).expect("read marker skill"),
        b"sentinel content"
    );
    for (path, contents) in older_files {
        let after = fs::read(repo.path().join(path)).expect("read older file");
        assert_eq!(after, contents, "{path} must not change");
    }

    let agent = installed(&repo, AGENT_MARKER);
    fs::remove_file(&agent).expect("remove agent file");

    let printed = rex_init_ok(&repo, &[]);

    assert_eq!(
        printed.stdout,
        format!(
            "rex init: 1 written, {} skipped (1 differs from this version; use --force to overwrite)\nrex init: pre-commit hooks installed\n",
            n - 1
        )
    );
    assert_eq!(
        fs::read(&agent).expect("read agent file"),
        bundle_file(AGENT_MARKER)
    );
    assert_eq!(
        fs::read(&marker).expect("read marker skill"),
        b"sentinel content"
    );
}

#[test]
fn existing_agents_md_gets_section_on_top_once() {
    let own = repo();
    let old: &[u8] = b"# My project\n\nKeep me exactly as I am.\n";
    fs::write(own.path().join("AGENTS.md"), old).expect("write AGENTS.md");
    fs::write(own.path().join(".gitignore"), b"/target").expect("write .gitignore");

    let first = rex_init_ok(&own, &[]);

    assert!(first.stdout.contains("rex init: AGENTS.md updated\n"));
    assert!(first.stdout.contains("rex init: .gitignore updated\n"));

    let agents_first = fs::read(own.path().join("AGENTS.md")).expect("read AGENTS.md");
    assert!(agents_first.starts_with(b"<!-- rex:codebase -->\n## Codebase map\n"));
    assert!(
        agents_first.ends_with(&[b"\n\n", old].concat()),
        "old bytes must follow the section after one blank line"
    );
    let ignore_first = fs::read(own.path().join(".gitignore")).expect("read .gitignore");
    assert_eq!(
        ignore_first,
        [b"/target\n\n", SUPERREX_IGNORE_BLOCK].concat()
    );

    let second = rex_init_ok(&own, &[]);

    assert!(!second.stdout.contains("AGENTS.md"));
    assert!(!second.stdout.contains(".gitignore"));
    assert_eq!(
        agents_first,
        fs::read(own.path().join("AGENTS.md")).expect("read AGENTS.md")
    );
    assert_eq!(
        ignore_first,
        fs::read(own.path().join(".gitignore")).expect("read .gitignore")
    );
}

// The promise that `rex init` is safe to run twice: the second run
// leaves every file in the repo as it was and adds none.
#[test]
fn a_second_run_changes_no_file() {
    let repo = repo();
    let n = bundle_file_count();
    rex_init_ok(&repo, &[]);
    let before = files_under(repo.path());

    let printed = rex_init_ok(&repo, &[]);

    assert_eq!(
        printed.stdout,
        format!("rex init: 0 written, {n} skipped\nrex init: pre-commit hooks installed\n")
    );
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
