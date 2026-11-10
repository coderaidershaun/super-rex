//! The pre-commit config `rex init` writes, the hook tools it looks for on
//! PATH, what happens when `pre-commit install` cannot run or fails, and the
//! folders where `rex init` refuses to run.

use std::fs;

use yaml_rust2::YamlLoader;

use super::support::{
    COMMIT_ALL, PRE_COMMIT_ARGS, SKILL_MARKER, installed, repo, rex_init, rex_init_ok,
};

/// The `stages` of the local hook called `id` in `config`. `None` means the
/// hook has no `stages` line, so it runs at commit.
fn stages_of(config: &str, id: &str) -> Option<Vec<String>> {
    let docs = YamlLoader::load_from_str(config).expect("the config is valid YAML");

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
fn pre_commit_config_is_created_and_hooks_installed() {
    let repo = repo();

    let printed = rex_init_ok(&repo, &[]);

    assert!(printed.stdout.ends_with(&format!(
        "rex init: .pre-commit-config.yaml created\nrex init: pre-commit hooks installed\n{COMMIT_ALL}"
    )));
    assert_eq!(printed.stderr, "");

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

    // The slow checks run at push, so a commit stays fast.
    let pre_push = Some(vec!["pre-push".to_string()]);
    assert_eq!(stages_of(&config, "gitleaks"), None);
    assert_eq!(stages_of(&config, "trivy"), pre_push);
    assert_eq!(stages_of(&config, "cargo-test"), pre_push);
    assert_eq!(stages_of(&config, "cargo-fmt"), None);
    assert_eq!(stages_of(&config, "cargo-clippy"), None);

    let args = fs::read_to_string(repo.bin().join(PRE_COMMIT_ARGS))
        .expect("fake pre-commit must have run");
    assert_eq!(
        args,
        "install --install-hooks --hook-type pre-commit --hook-type pre-push"
    );
}

#[test]
fn missing_pre_commit_tool_prints_the_install_command() {
    let all_missing = repo();
    all_missing.remove_tool("pre-commit");
    all_missing.remove_tool("gitleaks");
    all_missing.remove_tool("trivy");

    let printed = rex_init_ok(&all_missing, &[]);

    assert!(printed.stdout.ends_with(&format!(
        "rex init: pre-commit hooks skipped (pre-commit not found; install it, then run: pre-commit install --install-hooks --hook-type pre-commit --hook-type pre-push)\n{COMMIT_ALL}"
    )));
    assert!(all_missing.path().join(".pre-commit-config.yaml").is_file());
    let stderr = printed.stderr;
    assert!(
        stderr.contains(
            "rex init: warning: `gitleaks` not found on PATH. Its hook in .pre-commit-config.yaml fails every commit"
        ),
        "got: {stderr}"
    );
    assert!(
        stderr.contains(
            "rex init: warning: `trivy` not found on PATH. Its hook in .pre-commit-config.yaml fails every push"
        ),
        "got: {stderr}"
    );

    // Only the tool that is missing is named. Looking for a tool must not run
    // it: a name on PATH can point at anything.
    let one_missing = repo();
    one_missing.remove_tool("trivy");
    // A shell redirection, not `touch`, because PATH here holds nothing but
    // the stand-in tools. The file lands in the repo the script is run from.
    one_missing.write_tool("gitleaks", ": > ran\nexit 0");

    let stderr = rex_init_ok(&one_missing, &[]).stderr;

    assert!(
        !one_missing.path().join("ran").exists(),
        "rex started gitleaks to find out whether it is installed"
    );
    assert!(!stderr.contains("gitleaks"), "got: {stderr}");
    assert!(stderr.contains("`trivy` not found"), "got: {stderr}");

    let own_config = repo();
    own_config.remove_tool("gitleaks");
    own_config.remove_tool("trivy");
    fs::write(
        own_config.path().join(".pre-commit-config.yaml"),
        "repos: []\n",
    )
    .expect("write config");

    let stderr = rex_init_ok(&own_config, &[]).stderr;

    assert_eq!(
        stderr, "",
        "rex does not know which tools the user's hooks need"
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
        "echo 'Cowardly refusing to install hooks with core.hooksPath set' >&2\necho 'Run git config --unset-all core.hooksPath' >&2\nexit 1",
    );

    let output = rex_init(&repo, &[]);

    assert_eq!(output.status.code(), Some(1), "rex init must fail");
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    // The tool printed two lines. The message holds both on one line.
    assert_eq!(
        stderr,
        "rex init: failed: `pre-commit install` exited with exit status: 1; fix what it names, then run rex init again: Cowardly refusing to install hooks with core.hooksPath set Run git config --unset-all core.hooksPath\n\
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

#[test]
fn outside_a_repo_root_nothing_is_written() {
    let repo = repo();
    fs::remove_dir_all(repo.path().join(".git")).expect("remove .git directory");

    let output = rex_init(&repo, &[]);

    assert_eq!(output.status.code(), Some(1), "rex init must fail");
    let cwd = repo.path().canonicalize().expect("canonical repo path");
    assert_eq!(
        String::from_utf8(output.stderr).expect("utf8 stderr"),
        format!(
            "error: {} is not the root of a Git repository; run rex init in the folder that holds .git, or run git init first\n",
            cwd.display()
        )
    );
    assert_eq!(output.stdout, b"");
    assert_eq!(
        fs::read_dir(repo.path()).expect("read repo folder").count(),
        0,
        "nothing is written"
    );
}
