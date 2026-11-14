//! A first and a second `rex init`: the plugin is installed through the `claude`
//! command, the project files each land once and are never replaced, and the
//! person is told which project files to commit.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::support::{
    COMMIT_ALL, PLUGIN_INSTALL_CALLS, PLUGIN_INSTALLED, PLUGIN_SKIPPED, Repo, repo, rex_init_ok,
};

/// The block `rex init` adds to `.gitignore`, byte for byte.
const SUPERREX_IGNORE_BLOCK: &[u8] = b"# rex:ignore superrex\n.claude/settings.local.json\n/.claude/worktrees/\n/.rex/rex.db*\n/.rex/events.jsonl\n/.rex/orchestrators/\n/.rex/plans/\n/.rex/worktrees/\n/.rex/live/\n";

/// The first line of a first run.
const SETTINGS_CREATED: &str = "rex init: .claude/settings.local.json created\n";

fn config(repo: &Repo) -> Value {
    let text = fs::read_to_string(repo.path().join(".rex/config.json")).expect("read config");
    serde_json::from_str(&text).expect("config JSON")
}

fn config_with_storage(storage: &str) -> Value {
    json!({ "version": 1, "storage": storage, "depth": 4, "jobs_from_sdk": true, "min_claude_code": "2.1.323" })
}

#[test]
fn fresh_install_writes_the_project_files_and_no_store() {
    let repo = repo();

    let printed = rex_init_ok(&repo, &[]);

    assert_eq!(
        printed.stdout,
        format!(
            "{SETTINGS_CREATED}rex init: .rex/config.json created\n{PLUGIN_INSTALLED}rex init: AGENTS.md created\nrex init: .gitignore created\nrex init: .pre-commit-config.yaml created\nrex init: pre-commit hooks installed\n{COMMIT_ALL}"
        )
    );
    assert_eq!(
        printed.stderr, "",
        "no warning when every tool is installed"
    );
    assert_eq!(config(&repo), config_with_storage("off"));
    assert!(!repo.path().join(".rex/rex.db").exists(), "storage is off");
    for copied in [".claude/skills", ".claude/agents", ".rex/skills"] {
        assert!(
            !repo.path().join(copied).exists(),
            "rex init copies no skill or agent file, found {copied}"
        );
    }

    let agents_md = fs::read_to_string(repo.path().join("AGENTS.md")).expect("read AGENTS.md");
    assert!(agents_md.starts_with("<!-- rex:codebase -->\n## Codebase map\n"));
    assert!(agents_md.contains("`rex codebase --only code --with-context`"));
    assert!(agents_md.contains("\n## Code smells\n"));

    let gitignore = fs::read(repo.path().join(".gitignore")).expect("read .gitignore");
    assert_eq!(gitignore, SUPERREX_IGNORE_BLOCK);
}

// The plugin is installed only through Claude Code's own command, at the
// project's scope, so this test is the only proof of the two commands rex runs.
#[test]
fn init_installs_the_plugin_through_the_claude_command() {
    let default_source = repo();

    rex_init_ok(&default_source, &[]);

    assert_eq!(default_source.claude_calls(), PLUGIN_INSTALL_CALLS);

    let this_checkout = repo();

    let printed = rex_init_ok(&this_checkout, &["--plugin-source", "."]);

    assert_eq!(
        this_checkout.claude_calls(),
        "claude plugin marketplace add . --scope project\nclaude plugin install super-rex@super-rex --scope project\n"
    );
    assert!(
        printed
            .stdout
            .contains("rex init: plugin super-rex@super-rex installed at project scope from .\n"),
        "got: {}",
        printed.stdout
    );

    let failing = repo();
    failing.write_tool(
        "claude",
        "if [ \"$1\" = plugin ]; then echo 'marketplace not found' >&2; exit 1; fi\necho '2.1.323 (Claude Code)'",
    );

    let printed = super::support::rex_init_failed(&failing, &[]);

    assert_eq!(
        printed.stderr,
        format!(
            "rex init: failed: \"claude plugin marketplace add coderaidershaun/super-rex --scope project\" exited with exit status: 1, so the super-rex plugin is not installed; fix what it names, or give another --plugin-source, then run rex init again: marketplace not found\n{}",
            super::support::ONE_STEP_FAILED
        )
    );
    assert!(
        failing.path().join("AGENTS.md").is_file(),
        "the rest of the install still happens"
    );
}

#[test]
fn init_with_and_without_storage_writes_config() {
    let repo = repo();
    rex_init_ok(&repo, &[]);
    assert_eq!(config(&repo), config_with_storage("off"));

    let printed = rex_init_ok(&repo, &["--with-storage"]);

    assert_eq!(
        printed.stdout,
        format!(
            "rex init: .rex/config.json updated\n{PLUGIN_SKIPPED}rex init: .rex/rex.db created\nrex init: pre-commit hooks installed\nrex init: commit .rex/config.json (git add + git commit); a job works in the main checkout only when it is clean\n"
        )
    );
    assert_eq!(config(&repo), config_with_storage("on"));
    assert!(repo.path().join(".rex/rex.db").is_file());
    assert!(repo.path().join(".rex/orchestrators").is_dir());

    // A later run without the flag keeps the record: storage is turned off
    // only by the person, in the file.
    let printed = rex_init_ok(&repo, &[]);

    assert!(
        !printed.stdout.contains("config.json"),
        "got: {}",
        printed.stdout
    );
    assert_eq!(config(&repo), config_with_storage("on"));

    // A key of the person's stays, and a depth they asked for is recorded.
    let own = super::support::repo();
    fs::create_dir_all(own.path().join(".rex")).expect("create .rex");
    fs::write(
        own.path().join(".rex/config.json"),
        "{\"storage\": \"off\", \"jobs_from_sdk\": false, \"note\": \"mine\"}\n",
    )
    .expect("write config");

    rex_init_ok(&own, &["--with-storage", "--depth", "6"]);

    assert_eq!(
        config(&own),
        json!({ "storage": "on", "jobs_from_sdk": false, "note": "mine", "version": 1, "depth": 6, "min_claude_code": "2.1.323" })
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

// The promise that `rex init` is safe to run twice: the second run leaves
// every file in the repo as it was, adds none and installs nothing again.
#[test]
fn init_run_twice_changes_nothing() {
    let repo = repo();
    rex_init_ok(&repo, &["--with-storage"]);
    let before = files_under(repo.path());
    let calls = repo.claude_calls();

    let printed = rex_init_ok(&repo, &["--with-storage"]);

    assert_eq!(
        printed.stdout,
        format!("{PLUGIN_SKIPPED}rex init: pre-commit hooks installed\n")
    );
    assert_eq!(repo.claude_calls(), calls, "no claude plugin command again");
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
