//! `rex worktree add` and `rex worktree remove` on real Git repos: a job's
//! worktree goes only in `<repo>.worktrees/`, and its path is listed in the
//! settings file so that Claude Code's sandbox lets git commit there.

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::Duration;

use serde_json::{Value, json};
use tempfile::{TempDir, tempdir};

/// Git sets these when it runs a hook, such as the pre-push hook that runs
/// these tests; left in place they point git at the repo being pushed.
const GIT_LOCATION_VARIABLES: [&str; 4] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_COMMON_DIR",
    "GIT_INDEX_FILE",
];

/// A repo `app` in a temp folder of its own, so `app.worktrees` sits beside it.
struct Repo {
    parent: TempDir,
}

impl Repo {
    fn new() -> Self {
        let repo = Self {
            parent: tempdir().expect("create parent tempdir"),
        };
        fs::create_dir(repo.path()).expect("create repo folder");
        git(&repo.path(), &["init", "-q", "-b", "main"]);
        fs::write(repo.path().join("a"), "a\n").expect("write a file");
        git(&repo.path(), &["add", "a"]);
        git(&repo.path(), &["commit", "-q", "-m", "first"]);
        repo
    }

    /// The repo as the person names it, which may be through a link (on
    /// macOS, the temp folder is).
    fn path(&self) -> PathBuf {
        self.parent.path().join("app")
    }

    fn parent(&self) -> &Path {
        self.parent.path()
    }

    /// The folder that holds the job worktrees, spelled as the person spells it.
    fn root(&self) -> PathBuf {
        self.parent.path().join("app.worktrees")
    }

    /// `path` with every link resolved, as rex writes it.
    fn canonical(path: &Path) -> String {
        path.canonicalize()
            .expect("canonical path")
            .to_str()
            .expect("utf8 path")
            .to_string()
    }

    fn head(&self) -> String {
        git_out(&self.path(), &["rev-parse", "HEAD"])
    }

    fn settings_path(&self) -> PathBuf {
        self.path().join(".claude").join("settings.local.json")
    }

    fn settings(&self) -> Value {
        let text = fs::read_to_string(self.settings_path()).expect("read settings");
        serde_json::from_str(&text).expect("settings JSON")
    }

    fn write_settings(&self, value: &Value) {
        fs::create_dir_all(self.path().join(".claude")).expect("create .claude");
        fs::write(self.settings_path(), format!("{value:#}\n")).expect("write settings");
    }

    fn registered(&self) -> Value {
        self.settings()["permissions"]["additionalDirectories"].clone()
    }

    /// The worktree paths git lists, the main checkout first.
    fn git_worktrees(&self) -> Vec<String> {
        git_out(&self.path(), &["worktree", "list", "--porcelain"])
            .lines()
            .filter_map(|line| line.strip_prefix("worktree "))
            .map(str::to_string)
            .collect()
    }
}

fn git_command(dir: &Path) -> Command {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.email=t@example.invalid", "-c", "user.name=t"]);
    for variable in GIT_LOCATION_VARIABLES {
        command.env_remove(variable);
    }
    command
}

fn git(dir: &Path, args: &[&str]) {
    let status = git_command(dir).args(args).status().expect("run git");
    assert!(status.success(), "git {args:?} failed");
}

fn git_out(dir: &Path, args: &[&str]) -> String {
    let output = git_command(dir).args(args).output().expect("run git");
    assert!(output.status.success(), "git {args:?} failed");
    String::from_utf8(output.stdout)
        .expect("utf8 git output")
        .trim()
        .to_string()
}

fn rex(args: &[&dyn AsRef<std::ffi::OsStr>]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rex"));
    command.arg("worktree");
    for arg in args {
        command.arg(arg);
    }
    command.env_remove("REX_STORE");
    for variable in GIT_LOCATION_VARIABLES {
        command.env_remove(variable);
    }
    command
}

fn run(args: &[&dyn AsRef<std::ffi::OsStr>]) -> Output {
    rex(args).output().expect("run rex worktree")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("utf8 stdout")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("utf8 stderr")
}

#[test]
fn add_makes_the_worktree_and_registers_its_real_path() {
    let repo = Repo::new();
    // Keys of the person's stay, in their order, and so do their folders.
    repo.write_settings(&json!({
        "model": "opus",
        "permissions": { "additionalDirectories": ["/elsewhere"], "allow": ["Bash(npm test)"] }
    }));
    let dir = repo.root().join("0gjob1");

    let output = run(&[
        &"add",
        &repo.path(),
        &dir,
        &repo.head(),
        &"--branch",
        &"rex/0gjob1",
    ]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let real = Repo::canonical(&dir);
    assert_eq!(
        stdout(&output),
        format!("worktree={real}\nsettings=added\n")
    );
    assert!(
        dir.join(".git").is_file(),
        "a linked worktree has a .git file"
    );
    assert_eq!(git_out(&dir, &["branch", "--show-current"]), "rex/0gjob1");
    assert_eq!(
        fs::read_to_string(repo.settings_path()).expect("read settings"),
        format!(
            "{:#}\n",
            json!({
                "model": "opus",
                "permissions": { "additionalDirectories": ["/elsewhere", real], "allow": ["Bash(npm test)"] }
            })
        )
    );
}

#[test]
fn add_with_no_branch_makes_a_detached_worktree_and_a_fresh_settings_file() {
    let repo = Repo::new();
    let dir = repo.root().join("0gjob1--review");

    let output = run(&[&"add", &repo.path(), &dir, &repo.head()]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(
        git_out(&dir, &["rev-parse", "--abbrev-ref", "HEAD"]),
        "HEAD"
    );
    assert_eq!(repo.registered(), json!([Repo::canonical(&dir)]));
}

#[test]
fn add_refuses_each_place_outside_the_worktrees_folder_and_writes_nothing() {
    let repo = Repo::new();
    let outside = repo.parent().join("outside");
    fs::create_dir_all(&outside).expect("create outside folder");
    fs::create_dir_all(repo.root()).expect("create worktrees folder");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside, repo.root().join("link")).expect("link out");
    let head = repo.head();
    let cases: [(&str, PathBuf); 6] = [
        (
            "a sibling folder",
            repo.parent().join("other").join("0gjob1"),
        ),
        ("the worktrees folder itself", repo.root()),
        ("a path with ..", repo.root().join("..").join("app-x")),
        ("a relative path", PathBuf::from("app.worktrees/0gjob1")),
        (
            "a path through a link",
            repo.root().join("link").join("0gjob1"),
        ),
        (
            "a folder inside the repo",
            repo.path().join(".rex").join("worktrees").join("0gjob1"),
        ),
    ];

    for (case, dir) in cases {
        let output = rex(&[&"add", &repo.path(), &dir, &head])
            .current_dir(repo.parent())
            .output()
            .expect("run rex worktree");

        assert_eq!(output.status.code(), Some(2), "{case}: {}", stderr(&output));
        assert!(
            stderr(&output).starts_with("error: "),
            "{case}: {}",
            stderr(&output)
        );
        assert!(!repo.settings_path().exists(), "{case}: settings written");
        assert_eq!(repo.git_worktrees().len(), 1, "{case}: a worktree was made");
    }
    assert_eq!(fs::read_dir(&outside).expect("list outside").count(), 0);
}

#[test]
fn add_refuses_a_bad_commit_branch_or_repo_and_writes_nothing() {
    let repo = Repo::new();
    let head = repo.head();
    let dir = repo.root().join("0gjob1");
    let short = &head[..12];

    for (case, args) in [
        ("a short commit", vec![short.to_string()]),
        ("a commit that is not hex", vec!["z".repeat(40)]),
        (
            "a branch like an option",
            vec![head.clone(), "--branch".into(), "-f".into()],
        ),
    ] {
        let mut command = rex(&[&"add", &repo.path(), &dir]);
        command.args(&args);
        let output = command.output().expect("run rex worktree");

        assert_eq!(output.status.code(), Some(2), "{case}: {}", stderr(&output));
        assert!(!dir.exists(), "{case}: worktree made");
        assert!(!repo.settings_path().exists(), "{case}: settings written");
    }

    // A linked worktree is not a main checkout, and a plain folder is not a repo.
    let first = repo.root().join("0gjob0");
    assert!(run(&[&"add", &repo.path(), &first, &head]).status.success());
    let before = repo.settings();
    let plain = tempdir().expect("plain folder");
    for not_main in [first.clone(), plain.path().to_path_buf()] {
        let output = run(&[&"add", &not_main, &dir, &head]);

        assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
        assert!(!dir.exists());
        assert_eq!(repo.settings(), before);
    }
}

#[test]
fn add_refuses_a_folder_that_exists_or_settings_it_cannot_edit() {
    let repo = Repo::new();
    let head = repo.head();
    let dir = repo.root().join("0gjob1");
    assert!(run(&[&"add", &repo.path(), &dir, &head]).status.success());
    let before = fs::read_to_string(repo.settings_path()).expect("read settings");

    let again = run(&[&"add", &repo.path(), &dir, &head]);

    assert_eq!(again.status.code(), Some(2), "{}", stderr(&again));
    assert_eq!(
        fs::read_to_string(repo.settings_path()).expect("read"),
        before
    );

    // Settings rex cannot edit: the worktree is not made at all.
    let next = repo.root().join("0gjob2");
    for (case, text) in [
        ("not JSON", "{ not json"),
        ("not an object", "[1, 2]"),
        (
            "a list that is not one",
            r#"{"permissions": {"additionalDirectories": "/x"}}"#,
        ),
    ] {
        fs::write(repo.settings_path(), text).expect("write settings");

        let output = run(&[&"add", &repo.path(), &next, &head]);

        assert_eq!(output.status.code(), Some(1), "{case}: {}", stderr(&output));
        assert!(!next.exists(), "{case}: worktree made");
        assert_eq!(
            fs::read_to_string(repo.settings_path()).expect("read"),
            text
        );
    }

    #[cfg(unix)]
    {
        let target = repo.parent().join("target.json");
        fs::write(&target, "{}\n").expect("write link target");
        fs::remove_file(repo.settings_path()).expect("remove settings");
        std::os::unix::fs::symlink(&target, repo.settings_path()).expect("link settings");

        let output = run(&[&"add", &repo.path(), &next, &head]);

        assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
        assert!(!next.exists());
        assert_eq!(fs::read_to_string(&target).expect("read target"), "{}\n");
    }
}

// Two agents can make their worktrees at the same moment; a write that does
// not wait for the other would drop one path.
#[test]
fn add_waits_for_the_settings_lock_and_two_adds_both_land() {
    let repo = Repo::new();
    let head = repo.head();
    let lock =
        File::create(repo.path().join(".git").join("rex-settings.lock")).expect("create lock file");
    lock.lock().expect("hold the lock");
    let dir = repo.root().join("0gjob1");

    let mut waiting = rex(&[&"add", &repo.path(), &dir, &head])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("start rex worktree");
    thread::sleep(Duration::from_millis(500));

    assert!(
        waiting.try_wait().expect("poll rex").is_none(),
        "add must wait while another rex holds the lock"
    );
    assert!(!repo.settings_path().exists());
    lock.unlock().expect("release the lock");
    assert!(waiting.wait().expect("wait for rex").success());

    let racers: Vec<_> = ["0gjob2", "0gjob3", "0gjob4"]
        .iter()
        .map(|name| {
            rex(&[&"add", &repo.path(), &repo.root().join(name), &head])
                .stdout(Stdio::null())
                .spawn()
                .expect("start rex worktree")
        })
        .collect();
    for mut racer in racers {
        assert!(racer.wait().expect("wait for rex").success());
    }
    let mut registered: Vec<String> = repo
        .registered()
        .as_array()
        .expect("a list")
        .iter()
        .map(|entry| entry.as_str().expect("text").to_string())
        .collect();
    registered.sort();
    let mut expected: Vec<String> = ["0gjob1", "0gjob2", "0gjob3", "0gjob4"]
        .iter()
        .map(|name| Repo::canonical(&repo.root().join(name)))
        .collect();
    expected.sort();
    assert_eq!(registered, expected);
}

#[test]
fn remove_takes_out_the_worktree_and_its_entry() {
    let repo = Repo::new();
    let head = repo.head();
    let kept = repo.root().join("0gjob1");
    let dir = repo.root().join("0gjob2");
    for path in [&kept, &dir] {
        assert!(run(&[&"add", &repo.path(), path, &head]).status.success());
    }
    let real = Repo::canonical(&dir);

    let output = run(&[&"remove", &repo.path(), &dir]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(
        stdout(&output),
        format!("worktree={real}\nsettings=removed\n")
    );
    assert!(!dir.exists());
    assert_eq!(repo.registered(), json!([Repo::canonical(&kept)]));
    assert_eq!(repo.git_worktrees().len(), 2);

    // Once it is gone, a second remove has nothing left to do.
    let again = run(&[&"remove", &repo.path(), &dir]);

    assert!(again.status.success(), "stderr: {}", stderr(&again));
    assert_eq!(
        stdout(&again),
        format!("worktree={real}\nsettings=unchanged\n")
    );
}

#[test]
fn remove_keeps_a_worktree_with_uncommitted_work_and_its_entry() {
    let repo = Repo::new();
    let dir = repo.root().join("0gjob1");
    assert!(
        run(&[&"add", &repo.path(), &dir, &repo.head()])
            .status
            .success()
    );
    fs::write(dir.join("new"), "work\n").expect("write work");
    let before = repo.settings();

    let output = run(&[&"remove", &repo.path(), &dir]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(dir.join("new").exists());
    assert_eq!(repo.settings(), before);
}

#[test]
fn remove_refuses_a_folder_that_is_not_a_job_worktree() {
    let repo = Repo::new();
    let plain = repo.root().join("plain");
    fs::create_dir_all(&plain).expect("create plain folder");
    let outside = repo.parent().join("outside");
    fs::create_dir_all(&outside).expect("create outside folder");

    for dir in [plain.clone(), outside.clone(), repo.path()] {
        let output = run(&[&"remove", &repo.path(), &dir]);

        assert_eq!(
            output.status.code(),
            Some(2),
            "{}: {}",
            dir.display(),
            stderr(&output)
        );
        assert!(dir.exists());
    }
    assert!(!repo.settings_path().exists());
}
