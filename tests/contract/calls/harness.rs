//! The harness that runs one call file of `fixtures/shared/calls/` against the
//! binary: a temp Git repo with storage on, a store in a temp folder named by
//! `REX_STORE`, the setup batches through `rex ingest`, then the call.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use assert_cmd::Command;
use regex::Regex;
use serde::Deserialize;
use serde_json::{Value, json};
use super_rex::store::{Store, StorePaths};
use tempfile::TempDir;

/// One call file.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Call {
    pub name: String,
    pub setup: Vec<String>,
    pub argv: Vec<String>,
    pub stdin_file: Option<String>,
    pub expect: Expect,
}

/// What the call must give. Each key that is there is checked.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Expect {
    pub exit: i32,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub stdout_json: Option<Value>,
    pub stdout_json_contains: Option<Value>,
    pub stdout_pattern: Option<String>,
}

/// A repo whose `.rex/config.json` turns storage on, and a store of its own
/// in another temp folder.
pub struct World {
    repo: TempDir,
    store: TempDir,
}

/// The repo's root, where every relative path of a call file starts.
pub fn crate_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

pub fn read_call(name: &str) -> Call {
    let path = crate_root()
        .join("fixtures/shared/calls")
        .join(format!("{name}.json"));
    let text =
        fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

impl World {
    pub fn new() -> World {
        let repo = tempfile::tempdir().unwrap();
        fs::create_dir(repo.path().join(".git")).unwrap();
        fs::create_dir(repo.path().join(".rex")).unwrap();
        fs::write(
            repo.path().join(".rex/config.json"),
            json!({ "version": 1, "storage": "on", "depth": 4, "jobs_from_sdk": true, "min_claude_code": "2.1.323" }).to_string(),
        )
        .unwrap();
        let store = tempfile::tempdir().unwrap();
        fs::create_dir(store.path().join(".git")).unwrap();
        Store::create(&StorePaths::locate(store.path()).unwrap()).unwrap();
        World { repo, store }
    }

    pub fn repo(&self) -> &Path {
        self.repo.path()
    }

    /// The `.rex` folder of the store, as `REX_STORE` names it.
    pub fn store_dir(&self) -> PathBuf {
        self.store.path().join(".rex")
    }

    pub fn store(&self) -> Store {
        Store::open(&StorePaths::locate(self.store.path()).unwrap()).unwrap()
    }

    /// `rex <args>` in the repo, with this world's store and none of the session's variables.
    pub fn rex(&self, args: &[&str]) -> Command {
        let mut command = Command::cargo_bin("rex").unwrap();
        command
            .args(args)
            .current_dir(self.repo())
            .env("REX_STORE", self.store_dir())
            .env_remove("CLAUDE_PROJECT_DIR")
            .env_remove("CLAUDE_CODE_SESSION_ID");
        command
    }

    /// `rex ingest` with the batch file at `path`, from the repo root.
    pub fn ingest_file(&self, path: &str) -> Output {
        let batch = fs::read(crate_root().join(path)).unwrap();
        self.ingest(&batch)
    }

    pub fn ingest(&self, batch: &[u8]) -> Output {
        self.rex(&["ingest"])
            .write_stdin(batch.to_vec())
            .output()
            .unwrap()
    }

    /// Runs the call's setup batches, each of which must be taken in.
    pub fn set_up(&self, call: &Call) {
        for path in &call.setup {
            let output = self.ingest_file(path);
            assert!(
                output.status.success(),
                "setup {path} of {}: {}",
                call.name,
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    /// Runs the call after its setup and checks every part of `expect`.
    pub fn check(&self, call: &Call) {
        self.set_up(call);
        assert_eq!(call.argv[0], "rex", "{}: a call runs rex", call.name);
        let args: Vec<&str> = call.argv[1..].iter().map(String::as_str).collect();
        let mut command = self.rex(&args);
        if let Some(stdin) = &call.stdin_file {
            command.write_stdin(fs::read(crate_root().join(stdin)).unwrap());
        }
        let output = command.output().unwrap();
        let stdout = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        let expect = &call.expect;
        assert_eq!(
            output.status.code(),
            Some(expect.exit),
            "{}: exit; stdout {stdout:?}; stderr {stderr:?}",
            call.name
        );
        if let Some(text) = &expect.stdout {
            assert_eq!(&stdout, text, "{}: stdout", call.name);
        }
        if let Some(text) = &expect.stderr {
            assert_eq!(&stderr, text, "{}: stderr", call.name);
        }
        if let Some(pattern) = &expect.stdout_pattern {
            let pattern = Regex::new(pattern).unwrap();
            assert!(
                pattern.is_match(&stdout),
                "{}: stdout {stdout:?} against {pattern}",
                call.name
            );
        }
        let parsed = || -> Value {
            serde_json::from_str(&stdout).unwrap_or_else(|error| {
                panic!("{}: stdout is not JSON ({error}): {stdout}", call.name)
            })
        };
        if let Some(value) = &expect.stdout_json {
            assert_eq!(&parsed(), value, "{}: stdout JSON", call.name);
        }
        if let Some(value) = &expect.stdout_json_contains {
            let found = parsed();
            if let Err(at) = contains(&found, value, "") {
                panic!("{}: stdout JSON lacks {at}\ngot: {found:#}", call.name);
            }
        }
    }
}

/// `found` contains `wanted`: each key of an object is there and contains its
/// value; an array has the same length and each element contains the one
/// listed; anything else is equal. The error names the first path that fails.
pub fn contains(found: &Value, wanted: &Value, at: &str) -> Result<(), String> {
    match (found, wanted) {
        (Value::Object(found), Value::Object(wanted)) => {
            for (key, value) in wanted {
                let path = format!("{at}.{key}");
                let Some(kept) = found.get(key) else {
                    return Err(path);
                };
                contains(kept, value, &path)?;
            }
            Ok(())
        }
        (Value::Array(found), Value::Array(wanted)) if found.len() == wanted.len() => {
            for (index, (kept, value)) in found.iter().zip(wanted).enumerate() {
                contains(kept, value, &format!("{at}[{index}]"))?;
            }
            Ok(())
        }
        (found, wanted) if found == wanted => Ok(()),
        _ => Err(format!("{at} = {wanted}")),
    }
}
