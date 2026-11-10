//! A first and a second `rex init`: the bundle files, `AGENTS.md`,
//! `.gitignore` and the pre-commit config each land once and are never
//! replaced.

use std::fs;

use super::support::{
    PRE_COMMIT_ARGS, SKILL_MARKER, bundle_file, bundle_file_count, installed, repo, rex_init_ok,
};

#[test]
fn fresh_install_writes_every_bundle_file() {
    let repo = repo();
    let n = bundle_file_count();

    let printed = rex_init_ok(&repo, &[]);

    assert_eq!(
        printed.stdout,
        format!(
            "rex init: {n} written, 0 skipped\nrex init: AGENTS.md created\nrex init: .gitignore created\nrex init: .pre-commit-config.yaml created\nrex init: pre-commit hooks installed\n"
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
            "rex init: {} written, 1 skipped (1 differs from this version; use --force to overwrite)\nrex init: AGENTS.md created\nrex init: .gitignore created\nrex init: .pre-commit-config.yaml created\nrex init: pre-commit hooks installed\n",
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
    assert!(agents_md.contains("`rex codebase --rust-only --with-context`"));
    assert!(agents_md.contains("`rex codebase --for-human --rust-only --with-context`"));
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
fn fresh_install_creates_gitignore_with_claude_block() {
    let repo = repo();

    rex_init_ok(&repo, &[]);

    let gitignore = fs::read(repo.path().join(".gitignore")).expect("read .gitignore");
    assert_eq!(
        gitignore,
        b"# rex:ignore claude\n.claude/skills/\n.claude/settings.local.json\n"
    );
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
        b"/target\n\n# rex:ignore claude\n.claude/skills/\n.claude/settings.local.json\n"
    );

    let second = rex_init_ok(&repo, &[]);

    assert!(!second.stdout.contains(".gitignore"));

    let after_second = fs::read(repo.path().join(".gitignore")).expect("read .gitignore");
    assert_eq!(after_first, after_second);
}

#[test]
fn pre_commit_config_is_created_and_hooks_installed() {
    let repo = repo();

    let printed = rex_init_ok(&repo, &[]);

    assert!(printed.stdout.ends_with(
        "rex init: .pre-commit-config.yaml created\nrex init: pre-commit hooks installed\n"
    ));

    let config = fs::read_to_string(repo.path().join(".pre-commit-config.yaml"))
        .expect("read .pre-commit-config.yaml");
    assert!(config.starts_with("# rex:pre-commit\n"));
    assert!(config.contains("repo: https://github.com/pre-commit/pre-commit-hooks\n"));
    assert!(config.contains("entry: gitleaks git --pre-commit --staged --redact --verbose\n"));
    assert!(
        config.contains("--scanners secret,vuln,misconfig --severity HIGH,CRITICAL --exit-code 1")
    );
    assert!(config.contains("stages: [pre-push]\n"));

    let args = fs::read_to_string(repo.bin().join(PRE_COMMIT_ARGS))
        .expect("fake pre-commit must have run");
    assert_eq!(args, "install --hook-type pre-commit --hook-type pre-push");
}

#[test]
fn existing_pre_commit_config_is_never_touched() {
    let repo = repo();
    fs::write(
        repo.path().join(".pre-commit-config.yaml"),
        b"sentinel content",
    )
    .expect("write sentinel config");

    let printed = rex_init_ok(&repo, &[]);

    assert!(
        printed
            .stdout
            .contains("rex init: .pre-commit-config.yaml skipped (already exists)\n")
    );

    let config = fs::read(repo.path().join(".pre-commit-config.yaml")).expect("read config");
    assert_eq!(config, b"sentinel content");
}

#[test]
fn second_run_leaves_pre_commit_config_alone() {
    let repo = repo();

    rex_init_ok(&repo, &[]);

    let before = fs::read(repo.path().join(".pre-commit-config.yaml")).expect("read config");

    let second = rex_init_ok(&repo, &[]);

    assert!(!second.stdout.contains(".pre-commit-config.yaml"));
    assert!(
        second
            .stdout
            .ends_with("rex init: pre-commit hooks installed\n")
    );

    let after = fs::read(repo.path().join(".pre-commit-config.yaml")).expect("read config");
    assert_eq!(before, after);
}

#[test]
fn missing_pre_commit_tool_prints_the_install_command() {
    let repo = repo();
    repo.remove_tool("pre-commit");

    let printed = rex_init_ok(&repo, &[]);

    assert!(printed.stdout.ends_with(
        "rex init: pre-commit hooks skipped (pre-commit not found; install it, then run: pre-commit install --hook-type pre-commit --hook-type pre-push)\n"
    ));
    assert!(repo.path().join(".pre-commit-config.yaml").is_file());
}
