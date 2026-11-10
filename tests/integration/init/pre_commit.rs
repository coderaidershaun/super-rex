//! The pre-commit config `rex init` writes, the hook tools it looks for on
//! PATH, and what happens when `pre-commit install` cannot run or fails.

use std::fs;

use yaml_rust2::YamlLoader;

use super::support::{Repo, SKILL_MARKER, installed, repo, rex_init, rex_init_ok};

fn stderr_of_install(repo: &Repo) -> String {
    rex_init_ok(repo, &[]).stderr
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

    assert!(!output.status.success(), "rex init must fail");
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(
        stderr.contains("`pre-commit install` exited with"),
        "got: {stderr}"
    );
    assert!(
        stderr.contains("Cowardly refusing to install hooks"),
        "got: {stderr}"
    );
    assert!(installed(&repo, SKILL_MARKER).is_file());
    assert!(repo.path().join("AGENTS.md").is_file());
    assert!(repo.path().join(".pre-commit-config.yaml").is_file());
}

#[test]
fn a_git_file_instead_of_a_git_directory_skips_the_hooks_and_never_runs_pre_commit() {
    let repo = repo();
    let git = repo.path().join(".git");
    fs::remove_dir_all(&git).expect("remove .git directory");
    fs::write(&git, "gitdir: ../elsewhere\n").expect("write .git file");
    let ran = repo.path().join("pre-commit-ran");
    // The marker is made with a shell redirect, not `touch`: PATH holds only the
    // stand-in tools, so an external command would not be found.
    repo.write_tool("pre-commit", &format!(": > '{}'", ran.display()));

    let printed = rex_init_ok(&repo, &[]);

    assert!(!ran.exists(), "pre-commit must not run when .git is a file");
    assert!(
        printed
            .stdout
            .ends_with("rex init: pre-commit hooks skipped (no .git directory)\n"),
        "got: {}",
        printed.stdout
    );
}
