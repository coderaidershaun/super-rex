//! `rex doctor`: whether this repo's binary, store, config, plugin and Claude
//! Code fit together, and which open jobs no live session owns any more.

use std::fs;
use std::path::Path;

use rusqlite::{Connection, OptionalExtension};
use serde_json::Value;

use super::stdout::print_to_stdout;
use crate::error::{CommandError, FailureClass, RexError};
use crate::store::{Store, StoreError, StorePaths};

/// Check that the binary, the store, the config, the plugin and Claude Code fit.
#[derive(clap::Args)]
#[command(
    after_help = "Prints `doctor=ok` or `doctor=problems`, then `binary=`, `store=`, `storage=`, `claude=`, `plugin=` (the versions the newest session reported), and one `problem=` line for each thing to fix. Exits 0 either way."
)]
pub struct DoctorOptions {}

#[derive(thiserror::Error, Debug)]
pub enum DoctorError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Rex(#[from] RexError),
}

impl CommandError for DoctorError {
    fn failure_class(&self) -> FailureClass {
        match self {
            DoctorError::Store(error) => error.failure_class(),
            DoctorError::Rex(error) => error.failure_class(),
        }
    }

    fn exit_code(&self) -> u8 {
        match self {
            DoctorError::Store(error) => error.exit_code(),
            DoctorError::Rex(error) => error.exit_code(),
        }
    }
}

/// What doctor found, one field for each line it prints before the problems.
struct Findings {
    store: String,
    storage: String,
    claude: String,
    plugin: String,
    problems: Vec<String>,
}

/// # Errors
/// [`StoreError::NotInGitRepo`] when `cwd` is in no Git repository; a store
/// that cannot be read is a problem line, not an error.
pub fn run(cwd: &Path, _options: DoctorOptions) -> Result<(), DoctorError> {
    let root = StorePaths::locate(cwd)?.root;
    let mut findings = Findings {
        store: "none".to_owned(),
        storage: "missing".to_owned(),
        claude: "unknown".to_owned(),
        plugin: "unknown".to_owned(),
        problems: Vec::new(),
    };
    let config = findings.read_config(&root);
    findings.read_store(cwd, config.as_ref());
    let binary = env!("CARGO_PKG_VERSION");
    if findings.plugin != "unknown" && findings.plugin != binary {
        findings.problems.push(format!(
            "version skew: plugin {}, binary {binary}; install the same version of both",
            findings.plugin
        ));
    }
    if let Some(minimum) = config
        .as_ref()
        .and_then(|config| config.get("min_claude_code"))
        .and_then(Value::as_str)
        && is_older(&findings.claude, minimum)
    {
        findings.problems.push(format!(
            "Claude Code {} is older than {minimum}, the oldest the plugin runs on; run claude update",
            findings.claude
        ));
    }

    let mut text = format!(
        "doctor={}\nbinary={binary}\nstore={}\nstorage={}\nclaude={}\nplugin={}\n",
        if findings.problems.is_empty() {
            "ok"
        } else {
            "problems"
        },
        findings.store,
        findings.storage,
        findings.claude,
        findings.plugin,
    );
    for problem in &findings.problems {
        text.push_str(&format!("problem={}\n", one_line(problem)));
    }
    print_to_stdout(&text)?;
    Ok(())
}

impl Findings {
    /// The config of the repo, or `None` with a problem when it is missing or unreadable.
    fn read_config(&mut self, root: &Path) -> Option<Value> {
        let path = root.join(".rex/config.json");
        let text = match fs::read(&path) {
            Ok(text) => text,
            Err(_) => {
                self.problems.push(
                    "no .rex/config.json, so the plugin is off in this repo; run rex init"
                        .to_owned(),
                );
                return None;
            }
        };
        match serde_json::from_slice::<Value>(&text) {
            Ok(config) => {
                let storage = config
                    .get("storage")
                    .and_then(Value::as_str)
                    .unwrap_or("off");
                self.storage = storage.to_owned();
                Some(config)
            }
            Err(error) => {
                self.storage = "unreadable".to_owned();
                self.problems.push(format!(
                    ".rex/config.json is not valid JSON ({error}), so the plugin is off in this repo; correct it or run rex init"
                ));
                None
            }
        }
    }

    /// The store's version, the versions the newest session reported, and the
    /// open jobs of older sessions.
    fn read_store(&mut self, cwd: &Path, config: Option<&Value>) {
        let is_storage_on = config
            .and_then(|config| config.get("storage"))
            .and_then(Value::as_str)
            == Some("on");
        let store = match StorePaths::discover(cwd).and_then(|paths| Store::open(&paths)) {
            Ok(store) => store,
            Err(StoreError::NotFound { .. }) => {
                if is_storage_on {
                    self.problems.push(
                        "storage is on but there is no store; run rex init --with-storage"
                            .to_owned(),
                    );
                }
                return;
            }
            Err(error @ StoreError::VersionOne { .. }) => {
                self.store = "v1".to_owned();
                self.problems.push(error.to_string());
                return;
            }
            Err(StoreError::NewerSchema { found, .. }) => {
                self.store = format!("v{found}");
                self.problems.push(format!(
                    "store version {found} is newer than this rex; install a newer rex"
                ));
                return;
            }
            Err(error) => {
                self.store = "unreadable".to_owned();
                self.problems
                    .push(format!("the store cannot be opened: {error}"));
                return;
            }
        };
        self.store = "v2".to_owned();
        if let Err(error) = self.read_record(store.conn()) {
            self.problems
                .push(format!("the store cannot be read: {error}"));
        }
    }

    fn read_record(&mut self, conn: &Connection) -> rusqlite::Result<()> {
        let newest_start: Option<String> = conn
            .query_row(
                "SELECT data FROM ingest_event WHERE kind = 'session.start'
                 ORDER BY at DESC, seq DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(data) = newest_start.and_then(|data| serde_json::from_str::<Value>(&data).ok())
        {
            let text = |key: &str| data.get(key).and_then(Value::as_str).map(str::to_owned);
            self.claude = text("claude_code").unwrap_or_else(|| "unknown".to_owned());
            self.plugin = text("plugin").unwrap_or_else(|| "unknown".to_owned());
        }
        for (job, status, session) in orphans(conn)? {
            self.problems.push(format!(
                "open orphan: job {job} is {status}, but its session {session} is not the newest; close it, or resume it with /rex master resume"
            ));
        }
        Ok(())
    }
}

/// The open jobs whose session is not the newest one that wrote to the store:
/// a session ended without closing them. Newest is the session of the newest
/// event, or of the newest job when no event came.
fn orphans(conn: &Connection) -> rusqlite::Result<Vec<(String, String, String)>> {
    let mut statement = conn.prepare(
        "WITH newest AS (
             SELECT session_id FROM (
                 SELECT session_id, at FROM ingest_event
                 UNION ALL SELECT session_id, created_at FROM job)
             ORDER BY at DESC LIMIT 1)
         SELECT id, status, session_id FROM job
         WHERE status IN ('created', 'planned')
           AND session_id <> (SELECT session_id FROM newest)
         ORDER BY created_at, id",
    )?;
    let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
    rows.collect()
}

/// True when `found` is a version older than `minimum`, both `major.minor.patch`.
fn is_older(found: &str, minimum: &str) -> bool {
    let parts = |text: &str| -> Option<Vec<u32>> {
        text.split('.').map(|part| part.parse().ok()).collect()
    };
    match (parts(found), parts(minimum)) {
        (Some(found), Some(minimum)) => found < minimum,
        _ => false,
    }
}

/// A problem on one line, whatever text an error brought into it.
fn one_line(text: &str) -> String {
    text.replace("\r\n", " ").replace(['\r', '\n'], " ")
}
