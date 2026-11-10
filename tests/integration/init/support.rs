//! A Git repo to run `rex init` in, with stand-in tools on PATH, so a test
//! never depends on what is installed on the machine that runs it.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::{TempDir, tempdir};

/// The skill file that every bundle ships. Tests name no other skill, because
/// which skills ship can change.
pub const SKILL_MARKER: &str = "skills/rex-skill-language/SKILL.md";

/// An agent of the grid, for a test that needs a second bundle file.
pub const AGENT_MARKER: &str = "agents/model-medium-effort-medium.md";

/// The file the stand-in `pre-commit` writes its arguments to, in the folder
/// of the stand-in tools.
pub const PRE_COMMIT_ARGS: &str = "pre-commit-args";

/// A fresh Git repo plus a folder of stand-in tools that is the only folder
/// on PATH when `rex init` runs. The tools live outside the repo, so they
/// never show up in its tree.
pub struct Repo {
    dir: TempDir,
    bin: TempDir,
}

impl Repo {
    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    pub fn bin(&self) -> &Path {
        self.bin.path()
    }

    /// Take `tool` off PATH, to test what `rex init` does without it.
    pub fn remove_tool(&self, tool: &str) {
        fs::remove_file(self.bin().join(tool)).expect("remove stand-in tool");
    }

    /// Put a stand-in `tool` on PATH that runs `body` as a shell script.
    pub fn write_tool(&self, tool: &str, body: &str) {
        let path = self.bin().join(tool);
        // `rex` is a link to the binary under test, and a write through it
        // would overwrite that binary.
        assert!(
            !path.is_symlink(),
            "remove the {tool} link before replacing it"
        );
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("write stand-in tool");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("chmod");
        }
    }
}

/// A new Git repo with every tool `rex init` looks for on PATH. The test
/// process runs the real `git`, with its own PATH, so `rex init` never needs
/// it.
pub fn repo() -> Repo {
    let repo = Repo {
        dir: tempdir().expect("create repo tempdir"),
        bin: tempdir().expect("create tools tempdir"),
    };
    let git = Command::new("git")
        .args(["init", "-q"])
        .current_dir(repo.path())
        .status()
        .expect("run git init");
    assert!(git.success(), "git init failed");

    repo.write_tool("claude", "echo '2.1.322 (Claude Code)'");
    let args_file = repo.bin().join(PRE_COMMIT_ARGS);
    repo.write_tool(
        "pre-commit",
        &format!("printf '%s' \"$*\" > '{}'", args_file.display()),
    );
    for tool in ["gitleaks", "trivy", "gh", "bwrap", "socat"] {
        repo.write_tool(tool, "exit 0");
    }
    // The `rex` on PATH is the binary under test, so `rex init` finds itself
    // there and has nothing to warn about.
    #[cfg(unix)]
    std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_rex"), repo.bin().join("rex"))
        .expect("link rex onto PATH");
    repo
}

/// The command that runs `rex init` with `args` in `repo`, with only the
/// stand-in tools on PATH, for a test that sets more of its environment.
pub fn rex_init_command(repo: &Repo, args: &[&str]) -> assert_cmd::Command {
    let mut command = assert_cmd::Command::cargo_bin("rex").expect("find rex binary");
    command
        .arg("init")
        .args(args)
        .current_dir(repo.path())
        .env("PATH", repo.bin())
        // A store chosen in the developer's shell must not change where the
        // test's store goes.
        .env_remove("REX_STORE");
    command
}

/// Run `rex init` with `args` in `repo`, with only the stand-in tools on PATH.
pub fn rex_init(repo: &Repo, args: &[&str]) -> Output {
    rex_init_command(repo, args).output().expect("run rex init")
}

/// What a run of `rex init` printed.
pub struct Printed {
    pub stdout: String,
    pub stderr: String,
}

impl Printed {
    fn from_output(output: &Output) -> Self {
        Self {
            stdout: String::from_utf8(output.stdout.clone()).expect("utf8 stdout"),
            stderr: String::from_utf8(output.stderr.clone()).expect("utf8 stderr"),
        }
    }
}

/// Run `rex init` as [`rex_init`] does and check that it succeeded.
pub fn rex_init_ok(repo: &Repo, args: &[&str]) -> Printed {
    let output = rex_init(repo, args);
    let printed = Printed::from_output(&output);
    assert!(
        output.status.success(),
        "rex init failed: {}",
        printed.stderr
    );
    printed
}

/// Run `rex init` as [`rex_init`] does and check that it exited with 1.
pub fn rex_init_failed(repo: &Repo, args: &[&str]) -> Printed {
    let output = rex_init(repo, args);
    let printed = Printed::from_output(&output);
    assert_eq!(
        output.status.code(),
        Some(1),
        "rex init must exit with 1; stderr: {}",
        printed.stderr
    );
    printed
}

/// The file in `repo` that `rex init` merges its settings into.
pub fn settings_path(repo: &Repo) -> PathBuf {
    repo.path().join(".claude").join("settings.local.json")
}

pub fn bundle_file_count() -> usize {
    super_rex::bundle::walk().len()
}

/// The bundle contents of the file at `bundle_path`.
pub fn bundle_file(bundle_path: &str) -> &'static [u8] {
    super_rex::bundle::walk()
        .into_iter()
        .find(|(path, _)| path == Path::new(bundle_path))
        .unwrap_or_else(|| panic!("bundle must contain {bundle_path}"))
        .1
}

/// Where the bundle file at `bundle_path` is installed in `repo`.
pub fn installed(repo: &Repo, bundle_path: &str) -> PathBuf {
    repo.path().join(".claude").join(bundle_path)
}
