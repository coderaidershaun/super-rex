//! The pre-commit config `rex init` writes, the hook tools it looks for on
//! PATH, and what happens when `pre-commit install` cannot run or fails.

use std::fs;

use yaml_rust2::YamlLoader;

use super::support::{
    COMMIT_ALL, PRE_COMMIT_ARGS, Repo, SKILL_MARKER, installed, repo, rex_init, rex_init_ok,
};

fn stderr_of_install(repo: &Repo) -> String {
    rex_init_ok(repo, &[]).stderr
}

#[test]
fn pre_commit_config_is_created_and_hooks_installed() {
    let repo = repo();

    let printed = rex_init_ok(&repo, &[]);

    assert!(printed.stdout.ends_with(&format!(
        "rex init: .pre-commit-config.yaml created\nrex init: pre-commit hooks installed\n{COMMIT_ALL}"
    )));

    let config = fs::read_to_string(repo.path().join(".pre-commit-config.yaml"))
        .expect("read .pre-commit-config.yaml");
    assert!(config.starts_with("# rex:pre-commit\n"));
    assert!(
        config
            .contains("# Installed by `rex init`. Needs pre-commit, gitleaks and trivy on PATH.\n")
    );
    assert!(config.contains("--skip-dirs target,node_modules,.claude,.rex --skip-files"));
    assert!(config.contains("repo: https://github.com/pre-commit/pre-commit-hooks\n"));
    assert!(config.contains("entry: gitleaks git --pre-commit --staged --redact --verbose\n"));
    assert!(
        config.contains("--scanners secret,vuln,misconfig --severity HIGH,CRITICAL --exit-code 1")
    );
    assert!(config.contains("stages: [pre-push]\n"));

    let args = fs::read_to_string(repo.bin().join(PRE_COMMIT_ARGS))
        .expect("fake pre-commit must have run");
    assert_eq!(
        args,
        "install --install-hooks --hook-type pre-commit --hook-type pre-push"
    );
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

    assert!(printed.stdout.ends_with(&format!(
        "rex init: pre-commit hooks skipped (pre-commit not found; install it, then run: pre-commit install --install-hooks --hook-type pre-commit --hook-type pre-push)\n{COMMIT_ALL}"
    )));
    assert!(repo.path().join(".pre-commit-config.yaml").is_file());
}

#[test]
fn a_missing_hook_tool_is_named_on_stderr() {
    let repo = repo();
    repo.remove_tool("gitleaks");
    repo.remove_tool("trivy");

    let stderr = stderr_of_install(&repo);

    assert!(
        stderr.contains("rex init: warning: `gitleaks` not found on PATH"),
        "got: {stderr}"
    );
    assert!(
        stderr.contains("rex init: warning: `trivy` not found on PATH"),
        "got: {stderr}"
    );
}

#[test]
fn only_the_tool_that_is_missing_is_named() {
    let repo = repo();
    repo.remove_tool("trivy");

    let stderr = stderr_of_install(&repo);

    assert!(!stderr.contains("gitleaks"), "got: {stderr}");
    assert!(stderr.contains("`trivy` not found"), "got: {stderr}");
}

#[test]
fn no_warning_when_both_tools_are_installed() {
    let repo = repo();

    let stderr = stderr_of_install(&repo);

    assert_eq!(stderr, "");
}

#[test]
fn a_config_the_user_wrote_gets_no_tool_warning() {
    let repo = repo();
    repo.remove_tool("gitleaks");
    repo.remove_tool("trivy");
    fs::write(repo.path().join(".pre-commit-config.yaml"), "repos: []\n").expect("write config");

    let stderr = stderr_of_install(&repo);

    assert_eq!(
        stderr, "",
        "rex does not know which tools the user's hooks need"
    );
}

// Looking for a tool must not run it. A name on PATH can point at anything,
// and rex only needs to know that something is there.
#[test]
fn a_tool_on_path_is_not_run_while_rex_looks_for_it() {
    let repo = repo();
    repo.remove_tool("trivy");
    // A shell redirection, not `touch`, because PATH here holds nothing but
    // the stand-in tools. The file lands in the repo the script is run from.
    repo.write_tool("gitleaks", ": > ran\nexit 0");

    let stderr = stderr_of_install(&repo);

    assert!(
        !repo.path().join("ran").exists(),
        "rex started gitleaks to find out whether it is installed"
    );
    assert!(!stderr.contains("gitleaks"), "got: {stderr}");
    assert!(stderr.contains("`trivy` not found"), "got: {stderr}");
}

/// The `stages` of the local hook called `id` in the config that `rex init`
/// writes. `None` means the hook has no `stages` line, so it runs at commit.
fn stages_of(id: &str) -> Option<Vec<String>> {
    let repo = repo();
    rex_init_ok(&repo, &[]);

    let text =
        fs::read_to_string(repo.path().join(".pre-commit-config.yaml")).expect("read config");
    let docs = YamlLoader::load_from_str(&text).expect("the config is valid YAML");

    let repos = docs[0]["repos"].as_vec().expect("repos is a list");
    let hook = repos
        .iter()
        .filter_map(|repo| repo["hooks"].as_vec())
        .flatten()
        .find(|hook| hook["id"].as_str() == Some(id))
        .unwrap_or_else(|| panic!("the config has no hook `{id}`"));

    hook["stages"].as_vec().map(|stages| {
        stages
            .iter()
            .map(|stage| stage.as_str().expect("a stage is text").to_string())
            .collect()
    })
}

#[test]
fn the_secret_scan_of_the_staged_diff_runs_at_commit() {
    assert_eq!(stages_of("gitleaks"), None);
}

#[test]
fn the_slow_whole_tree_scan_runs_at_push() {
    assert_eq!(stages_of("trivy"), Some(vec!["pre-push".to_string()]));
}

#[test]
fn the_tests_run_at_push() {
    assert_eq!(stages_of("cargo-test"), Some(vec!["pre-push".to_string()]));
}

#[test]
fn the_fast_format_and_lint_checks_run_at_commit() {
    assert_eq!(stages_of("cargo-fmt"), None);
    assert_eq!(stages_of("cargo-clippy"), None);
}

#[test]
fn a_missing_trivy_is_reported_as_a_push_problem_and_gitleaks_as_a_commit_problem() {
    let repo = repo();
    repo.remove_tool("gitleaks");
    repo.remove_tool("trivy");

    let stderr = stderr_of_install(&repo);

    assert!(
        stderr.contains(
            "`gitleaks` not found on PATH. Its hook in .pre-commit-config.yaml fails every commit"
        ),
        "got: {stderr}"
    );
    assert!(
        stderr.contains(
            "`trivy` not found on PATH. Its hook in .pre-commit-config.yaml fails every push"
        ),
        "got: {stderr}"
    );
}

// This test pins how a failed `pre-commit install` looks to the caller. The
// real tool refuses when git's core.hooksPath is set, and the person running
// rex must see why, not just that it failed.
#[test]
fn a_failed_pre_commit_install_exits_with_its_error_text_after_the_rest_is_done() {
    let repo = repo();
    repo.write_tool(
        "pre-commit",
        "echo 'Cowardly refusing to install hooks with core.hooksPath set' >&2\nexit 1",
    );

    let output = rex_init(&repo, &[]);

    assert_eq!(output.status.code(), Some(1), "rex init must fail");
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert_eq!(
        stderr,
        "rex init: failed: `pre-commit install` exited with exit status: 1; fix what it names, then run rex init again: Cowardly refusing to install hooks with core.hooksPath set\n\
         error: rex init did not finish 1 step(s); each is named above\n"
    );
    assert!(installed(&repo, SKILL_MARKER).is_file());
    assert!(repo.path().join("AGENTS.md").is_file());
    assert!(repo.path().join(".pre-commit-config.yaml").is_file());
}

#[test]
fn a_git_file_instead_of_a_git_directory_is_refused_and_never_runs_pre_commit() {
    let repo = repo();
    let git = repo.path().join(".git");
    fs::remove_dir_all(&git).expect("remove .git directory");
    fs::write(&git, "gitdir: ../elsewhere\n").expect("write .git file");
    let ran = repo.bin().join("pre-commit-ran");
    // The marker is made with a shell redirect, not `touch`: PATH holds only the
    // stand-in tools, so an external command would not be found.
    repo.write_tool("pre-commit", &format!(": > '{}'", ran.display()));

    let output = rex_init(&repo, &[]);

    assert_eq!(output.status.code(), Some(1), "rex init must fail");
    assert!(!ran.exists(), "pre-commit must not run when .git is a file");
    let cwd = repo.path().canonicalize().expect("canonical repo path");
    assert_eq!(
        String::from_utf8(output.stderr).expect("utf8 stderr"),
        format!(
            "error: {} is a linked worktree, a submodule or a --separate-git-dir checkout (.git is a file); run rex init in the main checkout of the repository\n",
            cwd.display()
        )
    );
    let names: Vec<_> = fs::read_dir(repo.path())
        .expect("read repo folder")
        .map(|entry| entry.expect("read repo entry").file_name())
        .collect();
    assert_eq!(names, [".git"], "nothing is written");
}
